use super::*;
use domain::{
    ClarificationAnswerRequest, ClarificationCommandActor, ClarificationDeliveryOutcome,
    ClarificationDeliveryState, TrackerAnswer,
};

struct HttpServer(tokio::task::JoinHandle<()>);
impl Drop for HttpServer {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn serve(router: axum::Router) -> (String, HttpServer) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    (
        base,
        HttpServer(tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap()
        })),
    )
}

async fn setup() -> (PostgresFleetRepository, ClarificationCommandActor, Uuid) {
    std::env::var("FLEET_TEST_DATABASE_URL")
        .expect("isolated PostgreSQL is required for clarification custody");
    let (repo, _, _) = fixture().await.expect("configured PostgreSQL fixture");
    let subject = Uuid::new_v4().to_string();
    let owner = repo
        .find_or_create_central_user(
            &subject,
            &format!("{subject}@example.test"),
            "Custody owner",
        )
        .await
        .unwrap()
        .id;
    let agent_id = agent(&repo).await;
    let session = repo
        .create_session(chat(agent_id, &Uuid::new_v4().to_string()), owner)
        .await
        .unwrap();
    let task = Uuid::new_v4();
    let binding = domain::TaskChatBinding {
        tracker_instance_id: "custody-fixture".into(),
        project_id: Uuid::new_v4(),
        task_id: task,
        root_task_id: task,
        agent_id,
        owner_subject: subject.clone(),
    };
    repo.bind_task_chat(session.id, binding.clone(), Uuid::new_v4().to_string())
        .await
        .unwrap();
    (
        repo,
        ClarificationCommandActor {
            session_id: session.id,
            user_id: owner,
            subject,
            binding,
        },
        Uuid::new_v4(),
    )
}

fn request() -> ClarificationAnswerRequest {
    ClarificationAnswerRequest {
        expected_question_version: 1,
        requirement_revision: 2,
        selected_option_ids: vec![],
        text: Some("Synthetic original answer".into()),
        comment: None,
        idempotency_key: Uuid::new_v4().to_string(),
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn concurrent_store_and_reload_preserve_one_original_body_and_key() {
    let (repo, actor, question) = setup().await;
    let original = request();
    let (a, b) = tokio::join!(
        repo.store_clarification_command(&actor, question, original.clone()),
        repo.store_clarification_command(&actor, question, original.clone())
    );
    let a = a.unwrap();
    assert_eq!(a.id, b.unwrap().id);
    let restored = repo
        .list_pending_clarification_commands(&actor)
        .await
        .unwrap();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].id, a.id);
    assert_eq!(
        restored[0].request.idempotency_key,
        original.idempotency_key
    );
    assert_eq!(restored[0].request.text, original.text);
    let mut changed = original.clone();
    changed.text = Some("Changed".into());
    assert!(
        repo.store_clarification_command(&actor, question, changed)
            .await
            .is_err()
    );
    assert!(
        repo.store_clarification_command(&actor, question, request())
            .await
            .is_err()
    );
    let other_agent = agent(&repo).await;
    let other_session = repo
        .create_session(
            chat(other_agent, &Uuid::new_v4().to_string()),
            actor.user_id,
        )
        .await
        .unwrap();
    let mut other = actor.clone();
    other.session_id = other_session.id;
    other.binding.agent_id = other_agent;
    repo.bind_task_chat(
        other.session_id,
        other.binding.clone(),
        Uuid::new_v4().to_string(),
    )
    .await
    .unwrap();
    assert!(matches!(
        repo.store_clarification_command(&other, question, original.clone())
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    assert!(
        repo.store_clarification_command(&other, question, request())
            .await
            .is_err()
    );
    let mut foreign = actor.clone();
    foreign.user_id = Uuid::new_v4();
    assert!(
        repo.get_clarification_command(&foreign, a.id)
            .await
            .is_err()
    );
    assert!(
        repo.claim_clarification_delivery(&foreign, a.id)
            .await
            .is_err()
    );
    let mut drift = actor.clone();
    drift.binding.project_id = Uuid::new_v4();
    assert!(
        repo.list_pending_clarification_commands(&drift)
            .await
            .is_err()
    );
    drift = actor.clone();
    drift.subject = Uuid::new_v4().to_string();
    assert!(repo.get_clarification_command(&drift, a.id).await.is_err());
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn unknown_attempt_then_rejection_keeps_original_hold_until_exact_answer() {
    let (repo, actor, question) = setup().await;
    let command = repo
        .store_clarification_command(&actor, question, request())
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        repo.claim_clarification_delivery(&actor, command.id),
        repo.claim_clarification_delivery(&actor, command.id)
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_ne!(a.attempt_id.is_some(), b.attempt_id.is_some());
    let first = a.attempt_id.or(b.attempt_id).unwrap();
    let unknown = repo
        .finish_clarification_delivery(
            &actor,
            command.id,
            first,
            ClarificationDeliveryOutcome::Uncertain,
        )
        .await
        .unwrap();
    assert_eq!(unknown.state, ClarificationDeliveryState::Uncertain);
    assert!(
        repo.store_clarification_command(&actor, question, request())
            .await
            .is_err()
    );
    let retry = repo
        .claim_clarification_delivery(&actor, command.id)
        .await
        .unwrap();
    assert_eq!(retry.command.payload_sha256, command.payload_sha256);
    assert_eq!(
        retry.command.request.idempotency_key,
        command.request.idempotency_key
    );
    let stale = repo
        .finish_clarification_delivery(
            &actor,
            command.id,
            first,
            ClarificationDeliveryOutcome::Rejected(409),
        )
        .await
        .unwrap();
    assert_eq!(stale.state, ClarificationDeliveryState::Delivering);
    let held = repo
        .finish_clarification_delivery(
            &actor,
            command.id,
            retry.attempt_id.unwrap(),
            ClarificationDeliveryOutcome::Rejected(409),
        )
        .await
        .unwrap();
    assert_eq!(held.state, ClarificationDeliveryState::Uncertain);
    let retry = repo
        .claim_clarification_delivery(&actor, command.id)
        .await
        .unwrap();
    let answer = TrackerAnswer {
        id: Uuid::new_v4(),
        question_id: question,
        question_version: 1,
        requirement_revision: 2,
        selected_option_ids: vec![],
        text: command.request.text.clone(),
        comment: None,
        author_subject: actor.subject.clone(),
        created_at: chrono::Utc::now(),
    };
    let done = repo
        .finish_clarification_delivery(
            &actor,
            command.id,
            retry.attempt_id.unwrap(),
            ClarificationDeliveryOutcome::Delivered(answer.clone()),
        )
        .await
        .unwrap();
    assert_eq!(done.state, ClarificationDeliveryState::Delivered);
    assert_eq!(done.answer.unwrap().id, answer.id);
    assert!(
        repo.claim_clarification_delivery(&actor, command.id)
            .await
            .unwrap()
            .attempt_id
            .is_none()
    );
    assert!(
        repo.list_pending_clarification_commands(&actor)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        repo.store_clarification_command(&actor, question, command.request)
            .await
            .unwrap()
            .id,
        command.id
    );
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn expired_attempt_recovers_same_body_without_runtime_dispatch_or_identity_mutation() {
    let (repo, actor, question) = setup().await;
    let command = repo
        .store_clarification_command(&actor, question, request())
        .await
        .unwrap();
    let permit = repo
        .claim_clarification_delivery(&actor, command.id)
        .await
        .unwrap();
    assert!(permit.attempt_id.is_some());
    // Exercise the real expiry, never rewrite lease/authority with fixture SQL.
    sleep(Duration::from_secs(31)).await;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    assert!(db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE clarification_answer_commands SET attempt_id=$2,lease_until=clock_timestamp()+interval '30 seconds' WHERE id=$1",
        [command.id.into(),Uuid::new_v4().into()])).await.is_err());
    let recovered = repo
        .claim_clarification_delivery(&actor, command.id)
        .await
        .unwrap();
    assert_ne!(recovered.attempt_id, permit.attempt_id);
    assert_eq!(recovered.command.payload_sha256, command.payload_sha256);
    let held = repo
        .finish_clarification_delivery(
            &actor,
            command.id,
            recovered.attempt_id.unwrap(),
            ClarificationDeliveryOutcome::Rejected(403),
        )
        .await
        .unwrap();
    assert_eq!(held.state, ClarificationDeliveryState::Uncertain);
    assert!(
        repo.list_session_agent_runs(actor.session_id)
            .await
            .unwrap()
            .is_empty()
    );
    for sql in [
        "DELETE FROM clarification_answer_commands WHERE id=$1",
        "UPDATE clarification_answer_commands SET idempotency_key='new' WHERE id=$1",
        "UPDATE clarification_answer_commands SET request_body='{}' WHERE id=$1",
        "UPDATE clarification_answer_commands SET ever_uncertain=false WHERE id=$1",
    ] {
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                [command.id.into()]
            ))
            .await
            .is_err()
        );
    }
    db.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn http_reload_replays_exact_original_post_after_current_project_and_owner_checks() {
    use axum::{
        Extension, Json, Router,
        http::StatusCode,
        routing::{get, post},
    };
    use serde_json::{Value, json};
    let (repo, actor, question) = setup().await;
    let original = request();
    let answer = TrackerAnswer {
        id: Uuid::new_v4(),
        question_id: question,
        question_version: 1,
        requirement_revision: 2,
        selected_option_ids: vec![],
        text: original.text.clone(),
        comment: None,
        author_subject: actor.subject.clone(),
        created_at: chrono::Utc::now(),
    };
    let posts = Arc::new(AtomicUsize::new(0));
    let context_calls = Arc::new(AtomicUsize::new(0));
    let allowed = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let bodies = Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
    let binding = actor.binding.clone();
    let tracker = Router::new()
        .route("/api/v1/issues/{task}/sdlc/context",get({
            let allowed = allowed.clone(); let calls = context_calls.clone();
            move || { let binding=binding.clone(); let allowed=allowed.clone(); let calls=calls.clone(); async move {
                calls.fetch_add(1,Ordering::SeqCst);
                if !allowed.load(Ordering::SeqCst) { return (StatusCode::FORBIDDEN,Json(json!({"error":"revoked"}))); }
                (StatusCode::OK,Json(json!({"contract_version":1,"tracker_instance_id":binding.tracker_instance_id,
                    "project_id":binding.project_id,"task_id":binding.task_id,"root_task_id":binding.root_task_id,
                    "owner_subject":binding.owner_subject,"stage":"Clarification","requirement_revision":2,"waiting_reason":null,
                    "permissions":{"can_answer":false,"can_confirm":false},"assignment":{"assignment_id":Uuid::new_v4(),
                    "execution_id":Uuid::new_v4(),"agent_id":binding.agent_id,"version":1,"machine_subject":"synthetic"}})))
            }}
        }))
        .route("/api/v1/issues/{task}/sdlc/clarifications/{question}/answers",post({
            let posts=posts.clone(); let bodies=bodies.clone(); let answer=answer.clone();
            move |Json(body): Json<Value>| { let posts=posts.clone(); let bodies=bodies.clone(); let answer=answer.clone(); async move {
                bodies.lock().unwrap().push(body);
                if posts.fetch_add(1,Ordering::SeqCst)==0 {
                    (StatusCode::SERVICE_UNAVAILABLE,Json(json!({"error":"synthetic lost acknowledgement"})))
                } else { (StatusCode::OK,Json(serde_json::to_value(answer).unwrap())) }
            }}
        }));
    let (tracker_url, _tracker) = serve(tracker).await;
    let config = Arc::new(AppConfig {
        tracker: shared::config::TrackerConfig {
            url: tracker_url,
            instance_id: actor.binding.tracker_instance_id.clone(),
            ..Default::default()
        },
        ..Default::default()
    });
    let repo = Arc::new(repo);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = Arc::new(infra::runtime::LocalRuntimeSupervisor::new(
        config.clone(),
        repo.clone(),
        events.clone(),
    ));
    let (restart, _) = tokio::sync::mpsc::channel(1);
    let ctx = Arc::new(app::AppContext::new(
        config,
        repo.clone(),
        Arc::new(infra::FilesystemProvisioner),
        runtime,
        events,
        restart,
    ));
    let routes = || {
        Router::new()
            .route(
                "/api/v1/sessions/{session_id}/clarifications/{question_id}/answers",
                post(api::routes::task_chats::answer),
            )
            .route(
                "/api/v1/sessions/{session_id}/clarifications/{question_id}/answer-commands",
                post(api::routes::clarification_commands::store),
            )
            .route(
                "/api/v1/sessions/{session_id}/clarification-answer-commands",
                get(api::routes::clarification_commands::pending),
            )
            .route(
                "/api/v1/sessions/{session_id}/clarification-answer-commands/{command_id}",
                get(api::routes::clarification_commands::get),
            )
            .route(
                "/api/v1/sessions/{session_id}/clarification-answer-commands/{command_id}/delivery",
                post(api::routes::clarification_commands::delivery),
            )
    };
    // Fixture extensions isolate route custody, not Central Auth/JWKS acceptance.
    let user = api::middleware::CurrentUser {
        id: actor.user_id,
        role: domain::SystemRole::User,
        is_system_admin: false,
        central_write: Some(true),
    };
    // Production gives sessionless central tokens BOTH identity extensions.
    // Only the trusted human marker is absent; a request header cannot supply it.
    let (machine_url, _machine) = serve(
        routes()
            .layer(Extension(user.clone()))
            .layer(Extension(api::middleware::VerifiedCentralSubject(
                actor.subject.clone(),
            )))
            .with_state(ctx.clone()),
    )
    .await;
    let machine_base = format!("{machine_url}/api/v1/sessions/{}", actor.session_id);
    let (url, _fleet) = serve(
        routes()
            .layer(Extension(user.clone()))
            .layer(Extension(api::middleware::VerifiedHumanSession))
            .layer(Extension(api::middleware::VerifiedCentralSubject(
                actor.subject.clone(),
            )))
            .with_state(ctx.clone()),
    )
    .await;
    let client = reqwest::Client::new();
    let base = format!("{url}/api/v1/sessions/{}", actor.session_id);
    let stored = client
        .post(format!("{base}/clarifications/{question}/answer-commands"))
        .bearer_auth("synthetic-human")
        .json(&original)
        .send()
        .await
        .unwrap();
    assert_eq!(stored.status(), StatusCode::OK);
    assert_eq!(stored.headers().get("cache-control").unwrap(), "no-store");
    let stored: domain::ClarificationAnswerCommand = stored.json().await.unwrap();
    assert_eq!(posts.load(Ordering::SeqCst), 0);
    let delivery = format!(
        "{base}/clarification-answer-commands/{}/delivery",
        stored.id
    );
    let unknown: domain::ClarificationAnswerCommand = client
        .post(&delivery)
        .bearer_auth("synthetic-human")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(unknown.state, ClarificationDeliveryState::Uncertain);
    let restored: Vec<domain::ClarificationAnswerCommand> = client
        .get(format!("{base}/clarification-answer-commands"))
        .bearer_auth("synthetic-human")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(restored.len(), 1);
    assert_eq!(
        restored[0].request.idempotency_key,
        original.idempotency_key
    );
    let unknown_before = repo
        .get_clarification_command(&actor, stored.id)
        .await
        .unwrap();
    let context_before = context_calls.load(Ordering::SeqCst);
    for (method, suffix) in [
        (
            reqwest::Method::GET,
            format!("clarification-answer-commands/{}", stored.id),
        ),
        (
            reqwest::Method::POST,
            format!("clarification-answer-commands/{}/delivery", stored.id),
        ),
    ] {
        assert_eq!(
            client
                .request(method, format!("{machine_base}/{suffix}"))
                .bearer_auth("synthetic-sessionless-owner")
                .header("X-Verified-Human-Session", "true")
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(context_calls.load(Ordering::SeqCst), context_before);
    assert_eq!(posts.load(Ordering::SeqCst), 1);
    assert_eq!(
        serde_json::to_value(
            repo.get_clarification_command(&actor, stored.id)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(unknown_before).unwrap()
    );
    allowed.store(false, Ordering::SeqCst);
    assert_eq!(
        client
            .get(format!("{base}/clarification-answer-commands"))
            .bearer_auth("synthetic-human")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .post(&delivery)
            .bearer_auth("synthetic-human")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(posts.load(Ordering::SeqCst), 1);
    allowed.store(true, Ordering::SeqCst);
    let (reloaded_url, _reloaded) = serve(
        routes()
            .layer(Extension(user.clone()))
            .layer(Extension(api::middleware::VerifiedHumanSession))
            .layer(Extension(api::middleware::VerifiedCentralSubject(
                actor.subject.clone(),
            )))
            .with_state(ctx.clone()),
    )
    .await;
    let done: domain::ClarificationAnswerCommand = client
        .post(format!(
            "{reloaded_url}/api/v1/sessions/{}/clarification-answer-commands/{}/delivery",
            actor.session_id, stored.id
        ))
        .bearer_auth("synthetic-human")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(done.state, ClarificationDeliveryState::Delivered);
    assert_eq!(done.answer.unwrap().id, answer.id);
    let terminal = client
        .get(format!(
            "{base}/clarification-answer-commands/{}",
            stored.id
        ))
        .bearer_auth("synthetic-human")
        .send()
        .await
        .unwrap();
    assert_eq!(terminal.headers().get("cache-control").unwrap(), "no-store");
    let terminal: domain::ClarificationAnswerCommand = terminal.json().await.unwrap();
    assert_eq!(terminal.id, stored.id);
    assert_eq!(terminal.state, ClarificationDeliveryState::Delivered);
    assert_eq!(terminal.answer.unwrap().id, answer.id);
    assert_eq!(posts.load(Ordering::SeqCst), 2);
    let captured = bodies.lock().unwrap().clone();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0], captured[1]);
    let before = context_calls.load(Ordering::SeqCst);
    let (foreign_url, _foreign) = serve(
        routes()
            .layer(Extension(api::middleware::CurrentUser {
                id: Uuid::new_v4(),
                ..user.clone()
            }))
            .layer(Extension(api::middleware::VerifiedHumanSession))
            .layer(Extension(api::middleware::VerifiedCentralSubject(
                actor.subject.clone(),
            )))
            .with_state(ctx.clone()),
    )
    .await;
    assert_eq!(
        client
            .get(format!(
                "{foreign_url}/api/v1/sessions/{}/clarification-answer-commands",
                actor.session_id
            ))
            .bearer_auth("synthetic-human")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let pending_before = repo
        .list_pending_clarification_commands(&actor)
        .await
        .unwrap();
    let mut sessionless_request = original.clone();
    sessionless_request.idempotency_key = Uuid::new_v4().to_string();
    for (method, suffix) in [
        (reqwest::Method::GET, "clarification-answer-commands".into()),
        (
            reqwest::Method::GET,
            format!("clarification-answer-commands/{}", stored.id),
        ),
        (
            reqwest::Method::POST,
            format!("clarifications/{question}/answer-commands"),
        ),
        (
            reqwest::Method::POST,
            format!("clarification-answer-commands/{}/delivery", stored.id),
        ),
        (
            reqwest::Method::POST,
            format!("clarifications/{question}/answers"),
        ),
    ] {
        let response = client
            .request(method.clone(), format!("{machine_base}/{suffix}"))
            .bearer_auth("synthetic-sessionless-owner")
            .header("X-Verified-Human-Session", "true")
            .json(&sessionless_request)
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {suffix}"
        );
        assert_eq!(context_calls.load(Ordering::SeqCst), before);
        assert_eq!(posts.load(Ordering::SeqCst), 2);
    }
    assert_eq!(
        serde_json::to_value(
            repo.list_pending_clarification_commands(&actor)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(pending_before).unwrap()
    );
    assert_eq!(context_calls.load(Ordering::SeqCst), before);
    assert_eq!(posts.load(Ordering::SeqCst), 2);
    assert!(
        repo.list_session_agent_runs(actor.session_id)
            .await
            .unwrap()
            .is_empty()
    );
}
