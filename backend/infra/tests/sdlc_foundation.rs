use app::FleetRepository;
use domain::{
    AgentKind, AgentProductRole, AgentRole, AgentStatus, CreateAgentRequest,
    CreateSessionMessageRequest, CreateSessionRequest, MessageKind, SdlcRole, SessionRunRole,
    SessionRunState, UpdateAgentConfigRequest,
};
use infra::{PostgresFleetRepository, connect_database, run_migrations};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use shared::{AppConfig, DatabaseConfig};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::time::{Duration, sleep};
use uuid::Uuid;

async fn fixture() -> Option<(PostgresFleetRepository, Uuid, Uuid)> {
    let Ok(url) = std::env::var("FLEET_TEST_DATABASE_URL") else {
        eprintln!("FLEET_TEST_DATABASE_URL not configured; PostgreSQL tests skipped");
        return None;
    };
    let config = DatabaseConfig {
        url,
        max_connections: 10,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    };
    run_migrations(config.clone()).await.unwrap();
    let db = connect_database(config).await.unwrap();
    let owner = Uuid::new_v4();
    let stranger = Uuid::new_v4();
    for id in [owner, stranger] {
        db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
             VALUES ($1,$2,$3,'SDLC test','disabled',false,'user')",
            [id.into(), format!("{id}@example.test").into(), id.to_string().into()])).await.unwrap();
    }
    Some((PostgresFleetRepository::new(db), owner, stranger))
}

async fn agent(repo: &PostgresFleetRepository) -> Uuid {
    repo.ensure_runtime_templates().await.unwrap();
    let result = repo
        .create_agent(
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                product_role: AgentProductRole::Executor,
                role: AgentRole::Developer,
                sdlc_role: Some(SdlcRole::Developer),
                display_name: "Developer".into(),
                description: None,
                namespace_id: None,
                namespace_name: None,
                workflow_id: None,
                workflow_name: None,
                executor_ids: vec![],
            },
            &AppConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(result.sdlc_role, Some(SdlcRole::Developer));
    repo.update_agent_status(result.id, AgentStatus::Running)
        .await
        .unwrap();
    result.id
}

fn chat(agent_id: Uuid, key: &str) -> CreateSessionRequest {
    CreateSessionRequest {
        primary_agent_id: Some(agent_id),
        agent_id: None,
        title: "Private task".into(),
        task_key: None,
        leader_agent_id: None,
        parent_session_id: None,
        namespace_id: None,
        idempotency_key: Some(key.into()),
    }
}

fn prompt(key: &str) -> CreateSessionMessageRequest {
    CreateSessionMessageRequest {
        body: "Implement the confirmed task".into(),
        author_agent_id: None,
        message_kind: Some(MessageKind::UserPrompt),
        runtime_message_id: None,
        idempotency_key: Some(key.into()),
    }
}

fn configuration() -> UpdateAgentConfigRequest {
    UpdateAgentConfigRequest {
        config_json: serde_json::json!({"model": "test-model"}),
        soul_md: "Follow the assigned workflow.".into(),
        env_json: serde_json::json!({}),
    }
}

async fn message_count(db: &DatabaseConnection, session_id: Uuid) -> i64 {
    db.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT count(*) AS rows FROM session_messages WHERE session_id=$1",
        [session_id.into()],
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "rows")
    .unwrap()
}

#[tokio::test]
async fn message_receipts_and_replay_are_independent_of_history_limit() {
    let (repo, owner, stranger) = fixture()
        .await
        .expect("FLEET_TEST_DATABASE_URL is required for the receipt regression");
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    for history_size in [499_i32, 500] {
        let agent_id = agent(&repo).await;
        let session = repo
            .create_session(chat(agent_id, &format!("history-{history_size}")), owner)
            .await
            .unwrap();
        let seed_size = history_size - i32::try_from(message_count(&db, session.id).await).unwrap();
        assert_eq!(session.pending_delivery, Some(false));
        let initial_runs = repo.list_session_agent_runs(session.id).await.unwrap();
        assert_eq!(initial_runs.len(), 1);
        assert_eq!(initial_runs[0].state, SessionRunState::Pending);
        assert!(initial_runs[0].runtime_session_id.is_none());
        assert!(initial_runs[0].runtime_run_id.is_none());
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO session_messages
             (id,session_id,author_type,author_user_id,body,message_kind,delivery_state,created_at)
             SELECT gen_random_uuid(),$1,'user',$2,'Earlier message '||n,'user_prompt','mirrored',
                    timestamptz '2000-01-01' + n * interval '1 second'
             FROM generate_series(1,$3) n",
            [session.id.into(), owner.into(), seed_size.into()],
        ))
        .await
        .unwrap();
        assert_eq!(
            message_count(&db, session.id).await,
            i64::from(history_size)
        );
        let mut request = prompt("00000000-0000-4000-8000-000000000399");
        request.body = "Discuss password=fixture-value".into();
        request.message_kind = None;
        let result = repo
            .create_session_message(session.id, request.clone(), owner)
            .await;
        let count = message_count(&db, session.id).await;
        assert_eq!(count, i64::from(history_size) + 1);
        let created =
            result.expect("A committed message must return its receipt beyond the history page");
        assert_eq!(
            repo.get_session(session.id).await.unwrap().pending_delivery,
            Some(true)
        );
        assert_eq!(created.session_id, session.id);
        assert_eq!(created.author_user_id, Some(owner));
        assert_eq!(created.body, "Discuss password=redacted");
        assert_eq!(
            created.request_payload_hash.as_deref(),
            Some("ae34a8dd992f505a7b8e0f5ad8248284d857471f3a8067a041aa8632eff34087",)
        );
        assert!(!created.replayed);
        let replay = repo
            .create_session_message(session.id, request.clone(), owner)
            .await
            .unwrap();
        assert_eq!(replay.id, created.id);
        assert!(replay.replayed);
        assert_eq!(replay.body, created.body);
        assert_eq!(replay.request_payload_hash, created.request_payload_hash);
        let mut changed = request.clone();
        changed.body = "Different payload".into();
        assert!(
            repo.create_session_message(session.id, changed, owner)
                .await
                .is_err()
        );
        assert!(
            repo.create_session_message(session.id, request.clone(), stranger)
                .await
                .is_err()
        );
        let listed = repo.list_session_messages(session.id).await.unwrap();
        assert_eq!(listed.len(), 500);
        assert!(
            listed
                .iter()
                .all(|message| message.request_payload_hash.is_none())
        );
        assert_eq!(
            listed.iter().any(|message| message.id == created.id),
            history_size == 499
        );
        assert_eq!(message_count(&db, session.id).await, count);
        // Select this fixture ahead of other tests' pending outbox rows.
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE message_dispatch_outbox SET created_at=timestamptz '2000-01-01' WHERE message_id=$1",
            [created.id.into()],
        )).await.unwrap();
        let claimed = repo.claim_message_dispatch().await.unwrap().unwrap();
        assert_eq!(claimed.id, created.id);
        assert_eq!(claimed.body, request.body);
        assert!(claimed.request_payload_hash.is_none());
        repo.finish_message_dispatch(
            claimed.id,
            true,
            Some("Fixture retains unknown runtime acceptance".into()),
        )
        .await
        .unwrap();
        let runtime_id = format!("mirror-{history_size}");
        let mirror = repo
            .insert_session_message_mirror(
                session.id,
                Some(agent_id),
                "Confirmed reply".into(),
                MessageKind::AssistantMessage,
                Some(runtime_id.clone()),
            )
            .await
            .unwrap();
        let replayed_mirror = repo
            .insert_session_message_mirror(
                session.id,
                Some(agent_id),
                "Confirmed reply".into(),
                MessageKind::AssistantMessage,
                Some(runtime_id),
            )
            .await
            .unwrap();
        assert_eq!(mirror.session_id, session.id);
        assert_eq!(mirror.body, "Confirmed reply");
        assert_eq!(mirror.id, replayed_mirror.id);
        assert!(mirror.request_payload_hash.is_none());
        assert_eq!(message_count(&db, session.id).await, count + 1);
        assert_eq!(
            repo.list_session_messages(session.id).await.unwrap().len(),
            500
        );
    }
}

#[tokio::test]
async fn concurrent_idempotency_and_private_message_authorization() {
    let Some((repo, owner, stranger)) = fixture().await else {
        return;
    };
    let agent_id = agent(&repo).await;
    let request = chat(agent_id, "create-once");
    let (a, b) = tokio::join!(
        repo.create_session(request.clone(), owner),
        repo.create_session(request.clone(), owner)
    );
    let first = a.unwrap();
    assert_eq!(first.id, b.unwrap().id);
    assert!(first.leader_agent_id.is_none());
    let mut changed = request;
    changed.title = "Different payload".into();
    assert!(
        repo.create_session(changed, owner)
            .await
            .unwrap_err()
            .to_string()
            .contains("different session payload")
    );
    assert!(
        repo.create_session_message(first.id, prompt("stranger"), stranger)
            .await
            .is_err()
    );
    let (a, b) = tokio::join!(
        repo.create_session_message(first.id, prompt("once"), owner),
        repo.create_session_message(first.id, prompt("once"), owner)
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert_eq!(a.id, b.id);
    assert_ne!(a.replayed, b.replayed);
    let mut changed = prompt("once");
    changed.body = "Another message".into();
    assert!(
        repo.create_session_message(first.id, changed, owner)
            .await
            .unwrap_err()
            .to_string()
            .contains("different message payload")
    );
    assert_eq!(
        repo.list_session_messages(first.id)
            .await
            .unwrap()
            .iter()
            .filter(|message| message.message_kind == MessageKind::UserPrompt)
            .count(),
        1
    );
    let events = repo.list_session_events(first.id, 0).await.unwrap();
    assert!(events.len() >= 4);
    assert!(
        events
            .windows(2)
            .all(|pair| pair[0].sequence < pair[1].sequence)
    );
    let last = events.last().unwrap().sequence;
    assert_eq!(repo.session_event_cursor(first.id).await.unwrap(), last);
    assert!(
        repo.list_session_events(first.id, last)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn unknown_dispatch_holds_agent_capacity_and_terminal_mirror_is_deduplicated() {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let agent_id = agent(&repo).await;
    let first = repo
        .create_session(chat(agent_id, "first"), owner)
        .await
        .unwrap();
    let second = repo
        .create_session(chat(agent_id, "second"), owner)
        .await
        .unwrap();
    let message = repo
        .create_session_message(first.id, prompt("run-first"), owner)
        .await
        .unwrap();
    repo.create_session_message(second.id, prompt("run-second"), owner)
        .await
        .unwrap();
    // Other tests may leave pending messages; dispatch only this fixture's agent.
    let claimed = loop {
        let Some(candidate) = repo.claim_message_dispatch().await.unwrap() else {
            panic!("missing dispatch")
        };
        if candidate.id == message.id {
            break candidate;
        }
        repo.finish_message_dispatch(candidate.id, true, Some("test isolation hold".into()))
            .await
            .unwrap();
    };
    repo.finish_message_dispatch(claimed.id, true, Some("unknown runtime acceptance".into()))
        .await
        .unwrap();
    while let Some(candidate) = repo.claim_message_dispatch().await.unwrap() {
        assert_ne!(candidate.session_id, second.id);
        repo.finish_message_dispatch(candidate.id, true, Some("test isolation hold".into()))
            .await
            .unwrap();
    }
    let run = repo
        .prepare_session_agent_run(
            first.id,
            agent_id,
            SessionRunRole::Primary,
            "fleet:test".into(),
        )
        .await
        .unwrap();
    assert!(
        repo.prepare_session_agent_run(
            second.id,
            agent_id,
            SessionRunRole::Primary,
            "fleet:other".into()
        )
        .await
        .is_err()
    );
    repo.update_session_agent_run_dispatch(
        run.id,
        Some("runtime-run".into()),
        SessionRunState::Completed,
        None,
    )
    .await
    .unwrap();
    let a = repo
        .insert_session_message_mirror(
            first.id,
            Some(agent_id),
            "Done".into(),
            MessageKind::AssistantMessage,
            Some("runtime-run".into()),
        )
        .await
        .unwrap();
    let b = repo
        .insert_session_message_mirror(
            first.id,
            Some(agent_id),
            "Done".into(),
            MessageKind::AssistantMessage,
            Some("runtime-run".into()),
        )
        .await
        .unwrap();
    assert_eq!(a.id, b.id);
}

#[tokio::test]
async fn config_revision_drains_runs_and_failed_rollback_stays_blocked() {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let agent_id = agent(&repo).await;
    let session = repo
        .create_session(chat(agent_id, "config-drain"), owner)
        .await
        .unwrap();
    let run = repo
        .prepare_session_agent_run(
            session.id,
            agent_id,
            SessionRunRole::Primary,
            "fleet:config".into(),
        )
        .await
        .unwrap();
    let draft = repo
        .create_config_revision(agent_id, configuration(), owner)
        .await
        .unwrap();
    assert_eq!(draft.state, "draft");
    assert!(!draft.is_effective);
    assert!(
        repo.request_config_activation(agent_id, draft.revision, owner)
            .await
            .is_err()
    );
    repo.validate_config_revision(agent_id, draft.revision, vec![])
        .await
        .unwrap();
    repo.request_config_activation(agent_id, draft.revision, owner)
        .await
        .unwrap();
    assert!(repo.agent_is_draining(agent_id).await.unwrap());
    assert!(repo.claim_config_activation().await.unwrap().is_none());
    assert!(
        repo.create_config_revision(agent_id, configuration(), owner)
            .await
            .is_err()
    );
    repo.update_session_agent_run_dispatch(
        run.id,
        Some("config-run".into()),
        SessionRunState::Completed,
        None,
    )
    .await
    .unwrap();
    let claimed = repo.claim_config_activation().await.unwrap().unwrap();
    assert_eq!(claimed.agent_id, agent_id);
    repo.finish_config_activation(agent_id, draft.revision, None, true)
        .await
        .unwrap();
    assert!(!repo.agent_is_draining(agent_id).await.unwrap());
    assert!(repo.list_config_revisions(agent_id).await.unwrap()[0].is_effective);
    let next = repo
        .create_config_revision(agent_id, configuration(), owner)
        .await
        .unwrap();
    repo.validate_config_revision(agent_id, next.revision, vec![])
        .await
        .unwrap();
    repo.request_config_activation(agent_id, next.revision, owner)
        .await
        .unwrap();
    repo.claim_config_activation().await.unwrap().unwrap();
    repo.finish_config_activation(
        agent_id,
        next.revision,
        Some("rollback unverified".into()),
        false,
    )
    .await
    .unwrap();
    let revisions = repo.list_config_revisions(agent_id).await.unwrap();
    assert!(revisions[0].draining);
    assert_eq!(revisions[0].state, "failed");
    assert!(revisions[1].is_effective);
}

async fn runtime_http_fixture(runtime_status: &'static str) {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let agent_id = agent(&repo).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port = $2 WHERE id = $1",
        [agent_id.into(), i32::from(port).into()],
    ))
    .await
    .unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let calls = count.clone();
    let router = axum::Router::new()
        .route("/v1/runs", axum::routing::post(move |headers: axum::http::HeaderMap, axum::Json(body): axum::Json<serde_json::Value>| {
            let calls = calls.clone();
            async move {
                assert!(headers.get("authorization").unwrap().to_str().unwrap().starts_with("Bearer fc_"));
                assert!(headers.contains_key("idempotency-key"));
                assert!(body["session_id"].as_str().unwrap().starts_with("fleet:"));
                calls.fetch_add(1, Ordering::SeqCst);
                axum::Json(serde_json::json!({"run_id": "fixture-run"}))
            }
        }))
        .route("/v1/runs/fixture-run/events", axum::routing::get(|| async {
            ([(axum::http::header::CONTENT_TYPE, "text/event-stream")], "event: response.delta\ndata: {\"delta\":\"partial response\"}\n\n")
        }))
        .route("/v1/runs/fixture-run", axum::routing::get(move || async move {
            axum::Json(serde_json::json!({"status": runtime_status, "final_response": "verified response"}))
        }));
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let repo = Arc::new(repo);
    let session = repo
        .create_session(chat(agent_id, "http-chat"), owner)
        .await
        .unwrap();
    let mut config = AppConfig::default();
    config.fleet.runtime_token_secret = "isolated-test-runtime-secret".into();
    let (events, _) = tokio::sync::broadcast::channel(32);
    let _runtime =
        infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), repo.clone(), events);
    repo.create_session_message(session.id, prompt("http-once"), owner)
        .await
        .unwrap();
    let expected = match runtime_status {
        "completed" => SessionRunState::Completed,
        "interrupted" => SessionRunState::Failed,
        _ => SessionRunState::Waiting,
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let runs = repo.list_session_agent_runs(session.id).await.unwrap();
            if runs.iter().any(|run| run.state == expected) {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("runtime event/status reconciliation completed");
    repo.create_session_message(session.id, prompt("http-once"), owner)
        .await
        .unwrap();
    sleep(Duration::from_millis(350)).await;
    assert_eq!(count.load(Ordering::SeqCst), 1);
    let replies = repo
        .list_session_messages(session.id)
        .await
        .unwrap()
        .into_iter()
        .filter(|message| message.message_kind == MessageKind::AssistantMessage)
        .collect::<Vec<_>>();
    if runtime_status == "completed" {
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].body, "verified response");
    } else {
        assert!(
            replies.is_empty(),
            "EOF must not fabricate a completed answer"
        );
    }
    server.abort();
}

#[tokio::test]
async fn runtime_http_eof_without_terminal_status_keeps_run_waiting() {
    runtime_http_fixture("running").await;
}

#[tokio::test]
async fn runtime_http_terminal_readback_persists_one_answer() {
    runtime_http_fixture("completed").await;
}

#[tokio::test]
async fn runtime_http_interrupted_is_failed_without_fabricated_reply() {
    runtime_http_fixture("interrupted").await;
}
