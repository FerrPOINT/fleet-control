use app::{FleetRepository, RuntimeApprovalCreate, RuntimeSupervisor};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::sse::{Event, Sse},
    routing::{get, post},
};
use domain::{
    AgentKind, AgentProductRole, AgentRole, AgentStatus, CreateAgentRequest,
    CreateSessionMessageRequest, CreateSessionRequest, MessageKind, RuntimeApprovalState,
    SessionRunState,
};
use futures_util::{Stream, future::join_all, stream};
use infra::{PostgresFleetRepository, connect_database, run_migrations};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use shared::{AppConfig, DatabaseConfig};
use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{Mutex, broadcast, mpsc},
    task::JoinHandle,
    time::{sleep, timeout},
};
use uuid::Uuid;

const RUNTIME_RUN: &str = "run_approval_events_fixture";

struct FakeHermes {
    bearer: String,
    message_id: Uuid,
    events: Mutex<Option<mpsc::UnboundedReceiver<Event>>>,
    authenticated_calls: AtomicUsize,
}

impl FakeHermes {
    fn authenticate(&self, headers: &HeaderMap) -> Result<(), StatusCode> {
        if headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            != Some(self.bearer.as_str())
        {
            return Err(StatusCode::UNAUTHORIZED);
        }
        self.authenticated_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

async fn start_run(
    State(fake): State<Arc<FakeHermes>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    fake.authenticate(&headers)?;
    assert_eq!(headers["Idempotency-Key"], fake.message_id.to_string());
    assert_eq!(body["input"], "Exercise exact runtime approval requests");
    Ok(Json(json!({"run_id": RUNTIME_RUN})))
}

async fn events(
    State(fake): State<Arc<FakeHermes>>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    fake.authenticate(&headers)?;
    let receiver = fake
        .events
        .lock()
        .await
        .take()
        .ok_or(StatusCode::CONFLICT)?;
    Ok(Sse::new(stream::unfold(receiver, |mut receiver| async {
        receiver.recv().await.map(|event| (Ok(event), receiver))
    })))
}

struct TestServer(JoinHandle<()>);

impl Drop for TestServer {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn event(name: &str, payload: Value) -> Event {
    Event::default().event(name).json_data(payload).unwrap()
}

fn request(id: &str) -> Value {
    json!({"request_id": id, "command": format!("read {id} api_key=fixture-private"),
        "action": "read_file", "api_key": "fixture-private"})
}

async fn checkpoint(repo: &PostgresFleetRepository, session: Uuid, marker: &str) {
    timeout(Duration::from_secs(15), async {
        let mut after = 0;
        loop {
            for item in repo.list_session_events(session, after).await.unwrap() {
                after = item.sequence;
                if item.event_type == "session_run_delta"
                    && item.payload["text"]
                        .as_str()
                        .is_some_and(|text| text.ends_with(marker))
                {
                    return;
                }
            }
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("production SSE ingester must reach the checkpoint");
}

#[tokio::test]
#[ignore = "requires own disposable FLEET_RUNTIME_APPROVAL_EVENTS_TEST_DATABASE_URL database"]
async fn authenticated_hermes_sse_ingests_exact_requests_and_never_response_events() {
    let url = std::env::var("FLEET_RUNTIME_APPROVAL_EVENTS_TEST_DATABASE_URL")
        .expect("own PostgreSQL test database required");
    let parsed = reqwest::Url::parse(&url).unwrap();
    assert_eq!(
        parsed.path(),
        "/fleet_runtime_approval_events_test",
        "never use a shared Fleet database"
    );
    assert_eq!(parsed.username(), "fleet_approval_events_test");
    let database = DatabaseConfig {
        url,
        max_connections: 12,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    };
    run_migrations(database.clone()).await.unwrap();
    let db = connect_database(database.clone()).await.unwrap();
    let migrations = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT count(*)::bigint AS count FROM seaql_migrations".to_owned(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "count")
        .unwrap();
    assert_eq!(
        migrations, 11,
        "fixture must include accepted deployment and pending task-chat migrations"
    );
    let owner = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
         VALUES($1,$2,$3,'Approval events fixture','disabled',false,'user')",
        [owner.into(), format!("{owner}@example.test").into(), owner.to_string().into()]))
        .await.unwrap();
    let repo = Arc::new(PostgresFleetRepository::new(
        connect_database(database).await.unwrap(),
    ));
    repo.ensure_runtime_templates().await.unwrap();
    let mut config = AppConfig::default();
    config.fleet.runtime_token_secret = "runtime-approval-events-test-only".into();
    let config = Arc::new(config);
    let agent = repo
        .create_agent(
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                product_role: AgentProductRole::Executor,
                role: AgentRole::Developer,
                sdlc_role: Some(domain::SdlcRole::Developer),
                display_name: "Approval events fixture".into(),
                description: None,
                namespace_id: None,
                namespace_name: None,
                workflow_id: None,
                workflow_name: None,
                executor_ids: vec![],
            },
            &config,
        )
        .await
        .unwrap();
    repo.update_agent_status(agent.id, AgentStatus::Running)
        .await
        .unwrap();
    let session = repo
        .create_session(
            CreateSessionRequest {
                primary_agent_id: Some(agent.id),
                agent_id: None,
                title: "Exact approval ingestion".into(),
                task_key: None,
                leader_agent_id: None,
                parent_session_id: None,
                namespace_id: None,
                idempotency_key: Some(Uuid::new_v4().to_string()),
            },
            owner,
        )
        .await
        .unwrap();
    let message = repo
        .create_session_message(
            session.id,
            CreateSessionMessageRequest {
                body: "Exercise exact runtime approval requests".into(),
                author_agent_id: None,
                message_kind: Some(MessageKind::UserPrompt),
                runtime_message_id: None,
                idempotency_key: Some(Uuid::new_v4().to_string()),
            },
            owner,
        )
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET api_port=$2 WHERE id=$1",
        [agent.id.into(), i32::from(port).into()],
    ))
    .await
    .unwrap();
    let agent = repo.get_agent(agent.id).await.unwrap();
    let (sender, receiver) = mpsc::unbounded_channel();
    let fake = Arc::new(FakeHermes {
        bearer: format!(
            "Bearer {}",
            infra::agent_runtime_token(&config, agent.id).unwrap()
        ),
        message_id: message.id,
        events: Mutex::new(Some(receiver)),
        authenticated_calls: AtomicUsize::new(0),
    });
    let router = Router::new()
        .route("/v1/runs", post(start_run))
        .route(&format!("/v1/runs/{RUNTIME_RUN}/events"), get(events))
        .with_state(fake.clone());
    let _server = TestServer(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap()
    }));
    let unauthenticated = reqwest::Client::new()
        .get(format!(
            "http://127.0.0.1:{port}/v1/runs/{RUNTIME_RUN}/events"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    let (bus, _) = broadcast::channel(128);
    let runtime = infra::runtime::LocalRuntimeSupervisor::new(config, repo.clone(), bus);
    let dispatched = runtime
        .send_message(&agent, &session, &message)
        .await
        .unwrap();
    assert_eq!(dispatched.status, AgentStatus::Running);
    let run = repo
        .list_session_agent_runs(session.id)
        .await
        .unwrap()
        .into_iter()
        .find(|run| run.runtime_run_id.as_deref() == Some(RUNTIME_RUN))
        .unwrap();

    sender
        .send(event("approval.request", request("request_one")))
        .unwrap();
    sender
        .send(event("approval.requested", request("request_two")))
        .unwrap();
    for id in ["request_one", "response_only"] {
        sender
            .send(event(
                "approval.responded",
                json!({"request_id": id, "choice": "once", "resolved": 1}),
            ))
            .unwrap();
    }
    sender
        .send(event(
            "message.delta",
            json!({"delta": "requests-ingested"}),
        ))
        .unwrap();
    checkpoint(&repo, session.id, "requests-ingested").await;
    let initial = repo.list_session_approvals(session.id).await.unwrap();
    assert_eq!(
        initial.len(),
        2,
        "both request event names must create exactly two rows; responses create none"
    );
    for id in ["request_one", "request_two"] {
        let approval = initial
            .iter()
            .find(|item| item.runtime_approval_id.as_deref() == Some(id))
            .unwrap();
        assert_eq!(approval.session_run_id, run.id);
        assert_eq!(approval.runtime_run_id, RUNTIME_RUN);
        assert_eq!(approval.agent_id, agent.id);
        assert_eq!(approval.state, RuntimeApprovalState::Pending);
        assert!(!approval.prompt.contains("fixture-private"));
        assert_eq!(approval.detail["api_key"], "redacted");
    }
    assert_eq!(
        repo.get_session_agent_run(run.id).await.unwrap().state,
        SessionRunState::Waiting
    );
    assert_eq!(fake.authenticated_calls.load(Ordering::SeqCst), 2);

    for _ in 0..8 {
        sender
            .send(event("approval.requested", request("request_one")))
            .unwrap();
        sender
            .send(event("approval.request", request("request_two")))
            .unwrap();
    }
    let concurrent = join_all((0..16).map(|index| {
        let id = if index % 2 == 0 {
            "request_one"
        } else {
            "request_two"
        };
        let detail = request(id);
        repo.upsert_runtime_approval_request(RuntimeApprovalCreate {
            session_id: session.id,
            session_run_id: run.id,
            agent_id: agent.id,
            runtime_run_id: RUNTIME_RUN.into(),
            runtime_approval_id: Some(id.into()),
            prompt: detail["command"].as_str().unwrap().into(),
            detail,
        })
    }))
    .await;
    for result in concurrent {
        let approval = result.unwrap();
        assert!(initial.iter().any(|item| item.id == approval.id));
    }
    sender
        .send(event("message.delta", json!({"delta": "replays-ingested"})))
        .unwrap();
    checkpoint(&repo, session.id, "replays-ingested").await;
    let replayed = repo.list_session_approvals(session.id).await.unwrap();
    assert_eq!(replayed.len(), 2);
    assert!(
        replayed
            .iter()
            .all(|item| item.state == RuntimeApprovalState::Pending)
    );
    assert!(
        replayed
            .iter()
            .all(|item| initial.iter().any(|previous| previous.id == item.id))
    );

    sender
        .send(event(
            "run.completed",
            json!({"final_response": "Approval ingestion fixture complete"}),
        ))
        .unwrap();
    drop(sender);
    timeout(Duration::from_secs(15), async {
        while repo.get_session_agent_run(run.id).await.unwrap().state != SessionRunState::Completed
        {
            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("production event worker must finish before fixture cleanup");
    assert_eq!(
        repo.list_session_approvals(session.id).await.unwrap().len(),
        2
    );
    println!(
        "actual PostgreSQL: 10 migrations; authenticated production SSE: 2 pending request IDs; responded: 0 new rows; concurrent/replayed upserts: stable IDs"
    );
}
