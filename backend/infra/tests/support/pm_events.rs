use super::*;
use domain::{PmDispatchIntent, PmRunReservation, PmRuntimeStatus};
use infra::runtime::LocalRuntimeSupervisor;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::sync::broadcast;

const NATIVE: &str = "run_pm_stream";
const EFFECTIVE: &str = "native-pm-effective-session";

async fn journal(
    origin: String,
    config: &AppConfig,
) -> (PostgresFleetRepository, PmRunReservation) {
    let (repo, mut reservation) = pm_fixture()
        .await
        .expect("isolated PostgreSQL required for PM stream tests");
    let original = repo.get_session(reservation.session_id).await.unwrap();
    let mut binding = repo
        .get_task_chat_binding(original.id)
        .await
        .unwrap()
        .unwrap();
    binding.task_id = Uuid::new_v4();
    binding.root_task_id = binding.task_id;
    let session = repo
        .create_pm_draft_chat(
            domain::CreatePmDraftChat {
                binding: binding.clone(),
                title: "PM stream".into(),
                task_key: reservation.identity.task.clone(),
                idempotency_key: "pm-stream-chat".into(),
            },
            original.user_id,
        )
        .await
        .unwrap();
    reservation.session_id = session.id;
    reservation.identity.task_ref = binding.task_id.to_string();
    reservation.identity.root_ref = binding.root_task_id.to_string();
    assert!(
        repo.list_session_agent_runs(session.id)
            .await
            .unwrap()
            .is_empty()
    );
    let agent = reservation.identity.agent_id().unwrap();
    let port = reqwest::Url::parse(&origin).unwrap().port().unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        [agent.into(), i32::from(port).into()],
    ))
    .await
    .unwrap();
    repo.reserve_pm_run(reservation.clone()).await.unwrap();
    let runs = repo.list_session_agent_runs(session.id).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, reservation.session_run_id);
    let mut hash = Sha256::new();
    hash.update(b"fleet-hermes-default-profile-v1\0");
    hash.update(
        infra::agent_runtime_token(config, agent)
            .unwrap()
            .as_bytes(),
    );
    repo.prepare_pm_dispatch(PmDispatchIntent {
        session_run_id: reservation.session_run_id,
        origin,
        credential_fingerprint: format!("{:x}", hash.finalize()),
        request_body:
            json!({"input":"Original task", "session_id":reservation.runtime_session_id()})
                .to_string(),
        workflow_assignment: json!({"operation_key":reservation.identity.assignment_operation_key}),
        workflow_origin: "http://workflow.test".into(),
        workflow_credential_fingerprint: "b".repeat(64),
        runtime_context: json!({}),
        submitted: false,
        hermes_run_ref: None,
    })
    .await
    .unwrap();
    assert!(
        repo.claim_pm_submission(reservation.session_run_id)
            .await
            .unwrap()
    );
    (repo, reservation)
}

fn packet(id: Uuid) -> app::HermesTerminalCommit {
    app::HermesTerminalCommit {
        message_id: id,
        run_id: id,
        runtime_run_id: NATIVE.into(),
        runtime_session_id: EFFECTIVE.into(),
        state: SessionRunState::Completed,
        body: Some("Verified PM reply".into()),
        error: None,
    }
}

async fn accepted() -> (PostgresFleetRepository, PmRunReservation) {
    let (repo, reservation) = journal("http://127.0.0.1:23810".into(), &AppConfig::default()).await;
    repo.record_pm_submission(reservation.session_run_id, NATIVE.into())
        .await
        .unwrap();
    repo.accept_pm_run(reservation.session_run_id, NATIVE.into(), EFFECTIVE.into())
        .await
        .unwrap();
    (repo, reservation)
}

async fn pm_recovery_queue_contains(repo: &PostgresFleetRepository, id: Uuid) -> bool {
    let mut after = None;
    loop {
        let page = repo.list_recoverable_pm_streams(after).await.unwrap();
        assert!(page.len() <= 20);
        assert!(page.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            page.first()
                .is_none_or(|first| after.is_none_or(|previous| *first > previous))
        );
        if page.contains(&id) {
            return true;
        }
        let Some(&last) = page.last() else {
            return false;
        };
        if last > id {
            return false;
        }
        after = Some(last);
    }
}

#[tokio::test]
async fn pm_disabled_continuation_is_pending_for_custody_not_required_only_when_absent() {
    use app::RuntimeSupervisor;
    let (repo, reservation) = pm_fixture()
        .await
        .expect("isolated PostgreSQL required for PM continuation tests");
    let session = repo.get_session(reservation.session_id).await.unwrap();
    let binding = repo
        .get_task_chat_binding(session.id)
        .await
        .unwrap()
        .unwrap();
    let actor = domain::ClarificationCommandActor {
        session_id: session.id,
        user_id: session.user_id,
        subject: binding.owner_subject.clone(),
        binding,
    };
    let saved = repo
        .store_clarification_command(
            &actor,
            Uuid::new_v4(),
            domain::ClarificationAnswerRequest {
                expected_question_version: 1,
                requirement_revision: 1,
                selected_option_ids: vec![],
                text: Some("Saved answer".into()),
                comment: None,
                idempotency_key: "continuation-status".into(),
            },
        )
        .await
        .unwrap();
    let repo = Arc::new(repo);
    let (events, _) = broadcast::channel(16);
    let runtime = LocalRuntimeSupervisor::new(Arc::new(AppConfig::default()), repo.clone(), events);
    assert!(!repo.has_pm_run_custody(session.id).await.unwrap());
    assert_eq!(
        runtime.resume_pm_answer(&actor, &saved).await.unwrap(),
        domain::PmContinuationOutcome::NotRequired
    );
    repo.reserve_pm_run(reservation).await.unwrap();
    assert!(repo.has_pm_run_custody(session.id).await.unwrap());
    assert_eq!(
        runtime.resume_pm_answer(&actor, &saved).await.unwrap(),
        domain::PmContinuationOutcome::Pending
    );
}

#[tokio::test]
async fn pm_terminal_packet_concurrent_replay_is_once_and_keeps_ordinary_guard() {
    let (repo, reservation) = accepted().await;
    let id = reservation.session_run_id;
    assert!(repo.commit_hermes_terminal(packet(id)).await.is_err());
    let (a, b) = tokio::join!(
        repo.commit_pm_terminal(packet(id), PmRuntimeStatus::Completed),
        repo.commit_pm_terminal(packet(id), PmRuntimeStatus::Completed)
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert_ne!(a.2, b.2);
    assert_eq!(a.1.as_ref().unwrap().id, b.1.as_ref().unwrap().id);
    assert_eq!(a.0.updated_at, b.0.updated_at);
    let cursor = repo
        .session_event_cursor(reservation.session_id)
        .await
        .unwrap();
    let replay = repo
        .commit_pm_terminal(packet(id), PmRuntimeStatus::Completed)
        .await
        .unwrap();
    assert!(!replay.2);
    assert_eq!(replay.0.updated_at, a.0.updated_at);
    assert_eq!(
        repo.session_event_cursor(reservation.session_id)
            .await
            .unwrap(),
        cursor
    );
    repo.observe_pm_run(id, PmRuntimeStatus::Completed)
        .await
        .unwrap();
    assert_eq!(
        repo.get_session_agent_run(id).await.unwrap().updated_at,
        a.0.updated_at
    );
    assert_eq!(
        repo.session_event_cursor(reservation.session_id)
            .await
            .unwrap(),
        cursor
    );
    assert!(repo.pm_stream_context(id).await.unwrap().1);
    assert!(!pm_recovery_queue_contains(&repo, id).await);
    let mut changed = packet(id);
    changed.body = Some("Contradiction".into());
    assert!(
        repo.commit_pm_terminal(changed, PmRuntimeStatus::Completed)
            .await
            .is_err()
    );
    assert_eq!(
        repo.list_session_messages(reservation.session_id)
            .await
            .unwrap()
            .iter()
            .filter(|m| m.message_kind == MessageKind::AssistantMessage)
            .count(),
        1
    );
    let mut next = reservation.clone();
    next.session_run_id = Uuid::new_v4();
    next.dispatch_operation_key = "next-pm-stream".into();
    next.fence += 1;
    repo.reserve_pm_run(next).await.unwrap();
}

#[tokio::test]
async fn pm_terminal_pin_owner_and_atomic_rollback_are_enforced() {
    let (repo, reservation) = accepted().await;
    let id = reservation.session_run_id;
    let mut wrong = packet(id);
    wrong.runtime_session_id = reservation.runtime_session_id();
    assert!(
        repo.commit_pm_terminal(wrong, PmRuntimeStatus::Completed)
            .await
            .is_err()
    );
    assert!(!repo.pm_stream_context(id).await.unwrap().1);
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    // A contradictory pre-existing mirror must roll back both PM proof and journal marker.
    let message = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO session_messages(id,session_id,author_type,author_agent_id,body,message_kind,runtime_message_id,delivery_state,created_at)
         VALUES($1,$2,'agent',$3,'different','assistant_message',$4,'mirrored',now())",
        [message.into(),reservation.session_id.into(),reservation.identity.agent_id().unwrap().into(),NATIVE.into()])).await.unwrap();
    assert!(
        repo.commit_pm_terminal(packet(id), PmRuntimeStatus::Completed)
            .await
            .is_err()
    );
    assert!(!repo.pm_stream_context(id).await.unwrap().1);
    assert!(repo.get_pm_run(id).await.unwrap().terminal_status.is_none());
    assert_eq!(
        repo.get_session_agent_run(id).await.unwrap().state,
        SessionRunState::Running
    );
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM session_messages WHERE id=$1",
        [message.into()],
    ))
    .await
    .unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE users SET is_active=false WHERE id=(SELECT user_id FROM agent_sessions WHERE id=$1)",[reservation.session_id.into()])).await.unwrap();
    assert!(repo.pm_stream_context(id).await.is_err());
    assert!(
        repo.commit_pm_terminal(packet(id), PmRuntimeStatus::Completed)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn pm_observed_terminal_remains_recoverable_until_mirrored_and_marker_is_monotonic() {
    let mut fixtures = Vec::new();
    for _ in 0..22 {
        fixtures.push(accepted().await);
    }
    fixtures.sort_by_key(|(_, reservation)| reservation.session_run_id);
    for status in [PmRuntimeStatus::Completed, PmRuntimeStatus::Stopped] {
        let (repo, reservation) = fixtures.pop().unwrap();
        let id = reservation.session_run_id;
        repo.observe_pm_run(id, status).await.unwrap();
        assert!(!repo.pm_stream_context(id).await.unwrap().1);
        let first_page = repo.list_recoverable_pm_streams(None).await.unwrap();
        assert_eq!(first_page.len(), 20);
        assert!(!first_page.contains(&id));
        assert!(pm_recovery_queue_contains(&repo, id).await);
        let mut command = packet(id);
        if status == PmRuntimeStatus::Stopped {
            command.state = SessionRunState::Cancelled;
            command.body = None;
        }
        assert!(repo.commit_pm_terminal(command, status).await.unwrap().2);
        assert_eq!(
            repo.get_pm_run(id).await.unwrap().terminal_status,
            Some(status)
        );
        assert!(!pm_recovery_queue_contains(&repo, id).await);
        let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE pm_dispatch_journal SET terminal_committed=false WHERE session_run_id=$1",
                [id.into()]
            ))
            .await
            .is_err()
        );
    }
}

#[derive(Clone, Copy)]
enum StreamCase {
    Final,
    TerminalAtRestart,
    EofComplete,
    EofRunning,
    WrongSession,
    WrongReadbackPin,
    UnknownAck,
}

async fn http_stream(case: StreamCase, dispatch_enabled: bool) {
    use axum::{
        Json,
        http::{HeaderMap, Method, StatusCode, Uri},
        response::IntoResponse,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let mut config = AppConfig::default();
    config.pm.dispatch.enabled = dispatch_enabled;
    config.fleet.runtime_token_secret = "isolated-pm-stream-secret".into();
    let (repo, reservation) = journal(origin, &config).await;
    let id = reservation.session_run_id;
    let token =
        infra::agent_runtime_token(&config, reservation.identity.agent_id().unwrap()).unwrap();
    let bearer = format!("Bearer {token}");
    let server_token = token.clone();
    let reads = Arc::new(AtomicUsize::new(0));
    let streams = Arc::new(AtomicUsize::new(0));
    let posts = Arc::new(AtomicUsize::new(0));
    let unauthorized = Arc::new(AtomicUsize::new(0));
    let (r, s, p, u) = (
        reads.clone(),
        streams.clone(),
        posts.clone(),
        unauthorized.clone(),
    );
    let router=axum::Router::new().fallback(move |method:Method,uri:Uri,headers:HeaderMap| {
        let (r,s,p,u,bearer,token)=(r.clone(),s.clone(),p.clone(),u.clone(),bearer.clone(),server_token.clone());
        async move {
            if method==Method::POST { p.fetch_add(1,Ordering::SeqCst); return StatusCode::CONFLICT.into_response(); }
            if !uri.path().starts_with("/v1/runs/") { return StatusCode::NOT_FOUND.into_response(); }
            if headers.get("authorization").and_then(|h|h.to_str().ok())!=Some(bearer.as_str()) {
                u.fetch_add(1,Ordering::SeqCst); return StatusCode::UNAUTHORIZED.into_response();
            }
            assert_eq!(method,Method::GET);
            if uri.path()==format!("/v1/runs/{NATIVE}/events") {
                assert_eq!(headers["accept"],"text/event-stream");
                assert_eq!(headers["accept-encoding"],"identity");
                s.fetch_add(1,Ordering::SeqCst);
                let session=if matches!(case,StreamCase::WrongSession) { "foreign-session" } else { EFFECTIVE };
                let mut frames=String::new();
                for delta in ["PM ",token.as_str()," reply"] {
                    frames.push_str(&format!("event: message.delta\ndata: {}\n\n",json!({"run_id":NATIVE,"session_id":session,"delta":delta})));
                }
                if matches!(case,StreamCase::Final|StreamCase::WrongSession) {
                    frames.push_str(&format!("event: run.completed\ndata: {}\n\n",json!({"run_id":NATIVE,"session_id":session,
                        "completed":true,"partial":false,"interrupted":false,"output":format!("PM {token} reply")})));
                }
                return runtime_stream_bounds::EventResponse::body(frames.as_bytes().chunks(7).map(|b|b.to_vec()).collect()).response();
            }
            if uri.path()==format!("/v1/runs/{NATIVE}") {
                let first=r.fetch_add(1,Ordering::SeqCst)==0;
                let terminal=matches!(case,StreamCase::TerminalAtRestart|StreamCase::WrongReadbackPin)
                    || (!first && !matches!(case,StreamCase::EofRunning|StreamCase::WrongSession));
                let session=if matches!(case,StreamCase::WrongReadbackPin) { "foreign-session" } else { EFFECTIVE };
                let mut payload=json!({"object":"hermes.run","run_id":NATIVE,"session_id":session,"status":if terminal {"completed"} else {"running"}});
                if terminal { payload["completed"]=json!(true); payload["partial"]=json!(false); payload["interrupted"]=json!(false);
                    payload["final_response"]=json!(format!("PM {token} reply")); }
                return Json(payload).into_response();
            }
            StatusCode::NOT_FOUND.into_response()
        }
    });
    let _server = pm_dispatch::AbortServer(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap()
    }));
    // ACK (and, when disabled, its accepted effective pin) predates this supervisor.
    if !matches!(case, StreamCase::UnknownAck) {
        repo.record_pm_submission(id, NATIVE.into()).await.unwrap();
        if !dispatch_enabled {
            repo.accept_pm_run(id, NATIVE.into(), EFFECTIVE.into())
                .await
                .unwrap();
        }
    }
    let cursor = repo
        .session_event_cursor(reservation.session_id)
        .await
        .unwrap();
    let repo = Arc::new(repo);
    let (events, _) = broadcast::channel(64);
    let config = Arc::new(config);
    let _supervisor = LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events.clone());
    if matches!(case, StreamCase::UnknownAck) {
        assert!(!pm_recovery_queue_contains(&repo, id).await);
        sleep(Duration::from_millis(300)).await;
        assert_eq!(reads.load(Ordering::SeqCst), 0);
        assert_eq!(streams.load(Ordering::SeqCst), 0);
    } else {
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let done = if matches!(
                    case,
                    StreamCase::Final | StreamCase::EofComplete | StreamCase::TerminalAtRestart
                ) {
                    repo.pm_stream_context(id).await.unwrap().1
                } else if matches!(case, StreamCase::WrongReadbackPin) {
                    reads.load(Ordering::SeqCst) > 0
                } else {
                    streams.load(Ordering::SeqCst) > 0
                        && reads.load(Ordering::SeqCst)
                            >= if matches!(case, StreamCase::EofRunning) {
                                2
                            } else {
                                1
                            }
                };
                if done {
                    break;
                }
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("PM recovery/follower must settle");
    }
    if matches!(case, StreamCase::WrongReadbackPin) {
        sleep(Duration::from_millis(150)).await;
        assert_eq!(streams.load(Ordering::SeqCst), 0);
        assert_eq!(
            repo.get_pm_run(id)
                .await
                .unwrap()
                .hermes_session_ref
                .as_deref(),
            Some(EFFECTIVE),
            "foreign readback must not replace the accepted pin"
        );
    }
    let messages = repo
        .list_session_messages(reservation.session_id)
        .await
        .unwrap();
    let session_events = repo
        .list_session_events(reservation.session_id, cursor)
        .await
        .unwrap();
    assert_eq!(posts.load(Ordering::SeqCst), 0, "recovery must never POST");
    assert_eq!(unauthorized.load(Ordering::SeqCst), 0);
    let runs = repo
        .list_session_agent_runs(reservation.session_id)
        .await
        .unwrap();
    assert_eq!(runs.len(), 1, "recovery must not allocate a new run");
    assert_eq!(runs[0].id, id);
    if matches!(
        case,
        StreamCase::Final | StreamCase::EofComplete | StreamCase::TerminalAtRestart
    ) {
        assert_eq!(
            streams.load(Ordering::SeqCst),
            usize::from(!matches!(case, StreamCase::TerminalAtRestart))
        );
        let mirrors: Vec<_> = messages
            .iter()
            .filter(|m| m.message_kind == MessageKind::AssistantMessage)
            .collect();
        assert_eq!(mirrors.len(), 1);
        assert!(!mirrors[0].body.contains(&token));
        assert!(mirrors[0].body.ends_with(" reply"));
        assert_eq!(
            repo.get_session_agent_run(id).await.unwrap().state,
            SessionRunState::Completed
        );
        assert_eq!(
            repo.get_session_agent_run(id)
                .await
                .unwrap()
                .runtime_session_id,
            Some(reservation.runtime_session_id())
        );
        assert_eq!(
            repo.get_pm_run(id)
                .await
                .unwrap()
                .hermes_session_ref
                .as_deref(),
            Some(EFFECTIVE)
        );
        assert!(
            session_events
                .iter()
                .any(|e| e.event_type == "session_message_changed")
        );
        let deltas: Vec<&Value> = session_events
            .iter()
            .filter(|e| e.event_type == "session_run_delta")
            .map(|e| &e.payload)
            .collect();
        assert_eq!(
            deltas.len(),
            if matches!(case, StreamCase::TerminalAtRestart) {
                0
            } else {
                3
            }
        );
        assert!(deltas.iter().all(|p| p["run_id"] == json!(id)
            && p["text"].is_string()
            && !p.to_string().contains(&token)));
        if !matches!(case, StreamCase::TerminalAtRestart) {
            assert!(
                deltas.last().unwrap()["text"]
                    .as_str()
                    .unwrap()
                    .ends_with(" reply")
            );
        }
        let saved_cursor = repo
            .session_event_cursor(reservation.session_id)
            .await
            .unwrap();
        assert!(!pm_recovery_queue_contains(&repo, id).await);
        let _restarted = (!dispatch_enabled)
            .then(|| LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events.clone()));
        sleep(Duration::from_millis(150)).await;
        assert_eq!(
            repo.session_event_cursor(reservation.session_id)
                .await
                .unwrap(),
            saved_cursor
        );
        assert_eq!(
            repo.list_session_messages(reservation.session_id)
                .await
                .unwrap()
                .iter()
                .filter(|m| m.message_kind == MessageKind::AssistantMessage)
                .count(),
            1,
            "terminal mirror remains once across supervisor reconstruction"
        );
        assert_eq!(posts.load(Ordering::SeqCst), 0);
        assert_eq!(
            repo.list_session_agent_runs(reservation.session_id)
                .await
                .unwrap()
                .len(),
            1
        );
    } else {
        assert!(
            messages
                .iter()
                .all(|m| m.message_kind != MessageKind::AssistantMessage)
        );
        assert!(repo.get_pm_run(id).await.unwrap().terminal_status.is_none());
        let mut next = reservation.clone();
        next.session_run_id = Uuid::new_v4();
        next.dispatch_operation_key = "blocked-next".into();
        next.fence += 1;
        assert!(repo.reserve_pm_run(next).await.is_err());
        if matches!(
            case,
            StreamCase::WrongSession | StreamCase::WrongReadbackPin
        ) {
            assert!(
                session_events
                    .iter()
                    .all(|e| e.event_type != "session_run_delta")
            );
        }
    }
}

#[tokio::test]
async fn pm_http_restart_follow_cumulative_redacted_deltas_and_final_message() {
    http_stream(StreamCase::Final, true).await;
}
#[tokio::test]
async fn pm_http_eof_requires_independent_terminal_readback() {
    http_stream(StreamCase::EofComplete, true).await;
    http_stream(StreamCase::EofRunning, true).await;
}
#[tokio::test]
async fn pm_http_foreign_session_and_unknown_ack_never_release_or_redispatch() {
    http_stream(StreamCase::WrongSession, true).await;
    http_stream(StreamCase::UnknownAck, true).await;
}

#[tokio::test]
async fn pm_http_disabled_dispatch_recovers_accepted_sse_and_terminal_once() {
    http_stream(StreamCase::Final, false).await;
    http_stream(StreamCase::TerminalAtRestart, false).await;
}

#[tokio::test]
async fn pm_http_disabled_dispatch_denies_foreign_pins_and_unknown_ack() {
    http_stream(StreamCase::WrongSession, false).await;
    http_stream(StreamCase::WrongReadbackPin, false).await;
    http_stream(StreamCase::UnknownAck, false).await;
}
