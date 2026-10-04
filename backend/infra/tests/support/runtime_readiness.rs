use super::fixture;
use app::{FleetRepository, RuntimeStatePatch, RuntimeSupervisor};
use domain::{
    Agent, AgentKind, AgentProductRole, AgentRole, AgentStatus, CreateAgentRequest, DesiredState,
};
use infra::{PostgresFleetRepository, runtime::LocalRuntimeSupervisor};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::{Value, json};
use shared::{AppConfig, AppError};
use std::{path::PathBuf, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::oneshot,
    task::JoinHandle,
    time::{Duration, Instant, timeout},
};
use uuid::Uuid;

enum Reply {
    HungHeaders,
    HungBody,
    Json { body: String, chunked: bool },
}

struct TcpFixture {
    port: u16,
    requested: Option<oneshot::Receiver<()>>,
    task: JoinHandle<()>,
}

impl Drop for TcpFixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl TcpFixture {
    async fn new(reply: Reply) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (requested, receipt) = oneshot::channel();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let byte = socket.read_u8().await.unwrap();
                request.push(byte);
                assert!(request.len() <= 8192, "unexpected readiness request size");
            }
            let request = String::from_utf8(request).unwrap();
            assert!(request.starts_with("GET /actuator/health/readiness HTTP/1.1\r\n"));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("accept-encoding: identity\r\n")
            );
            match reply {
                Reply::HungHeaders => {
                    let _ = requested.send(());
                    std::future::pending::<()>().await;
                }
                Reply::HungBody => {
                    socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 256\r\nConnection: close\r\n\r\n{\"status\":").await.unwrap();
                    let _ = requested.send(());
                    std::future::pending::<()>().await;
                }
                Reply::Json { body, chunked } => {
                    let framing = if chunked {
                        "Transfer-Encoding: chunked".to_string()
                    } else {
                        format!("Content-Length: {}", body.len())
                    };
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n{framing}\r\nConnection: close\r\n\r\n"
                    );
                    socket.write_all(header.as_bytes()).await.unwrap();
                    let _ = requested.send(());
                    let body = if chunked {
                        format!("{:x}\r\n{body}\r\n0\r\n\r\n", body.len())
                    } else {
                        body
                    };
                    // An oversized Content-Length may be rejected before the fixture finishes writing.
                    let _ = socket.write_all(body.as_bytes()).await;
                }
            }
            drop(socket);
        });
        Self {
            port,
            requested: Some(receipt),
            task,
        }
    }

    async fn await_request(&mut self) {
        timeout(Duration::from_secs(2), self.requested.take().unwrap())
            .await
            .expect("readiness probe never reached the TCP fixture")
            .expect("TCP fixture rejected the readiness request");
    }
}

struct JavaFixture {
    repo: Arc<PostgresFleetRepository>,
    runtime: LocalRuntimeSupervisor,
    agent: Agent,
    root: PathBuf,
}

impl Drop for JavaFixture {
    fn drop(&mut self) {
        // This UUID path is exclusively owned by this fixture, never an accepted agents root.
        if self.root.exists() {
            std::fs::remove_dir_all(&self.root).unwrap();
        }
    }
}

impl JavaFixture {
    async fn new(port: u16, desired_state: DesiredState) -> Option<Self> {
        let (repo, _, _) = fixture().await?;
        repo.ensure_runtime_templates().await.unwrap();
        let mut config = AppConfig::default();
        let root = std::env::temp_dir().join(format!("fleet-runtime-readiness-{}", Uuid::new_v4()));
        config.fleet.agents_root = root.to_string_lossy().into_owned();
        let agent = repo
            .create_agent(
                CreateAgentRequest {
                    kind: AgentKind::JavaAgent,
                    product_role: AgentProductRole::Executor,
                    role: AgentRole::Developer,
                    sdlc_role: None,
                    display_name: "Java readiness fixture".into(),
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
        let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agents SET api_port=$2 WHERE id=$1",
            [agent.id.into(), i32::from(port).into()],
        ))
        .await
        .unwrap();
        let agent = repo
            .update_runtime_state(
                agent.id,
                RuntimeStatePatch {
                    status: AgentStatus::Running,
                    desired_state,
                    pid: Some(12345),
                    health_status: Some("previous".into()),
                    health_detail: None,
                    last_capabilities_json: Some(json!({"previous": "capabilities"})),
                    startup_command_redacted: None,
                    started_at: Some(shared::now()),
                    stopped_at: None,
                },
            )
            .await
            .unwrap();
        let repo = Arc::new(repo);
        let (events, _) = tokio::sync::broadcast::channel(32);
        let runtime = LocalRuntimeSupervisor::new(Arc::new(config), repo.clone(), events);
        Some(Self {
            repo,
            runtime,
            agent,
            root,
        })
    }

    async fn assert_preserved(&self, health: &str, capabilities: Value) {
        let observed = self.repo.get_agent(self.agent.id).await.unwrap();
        assert_eq!(observed.status, AgentStatus::Degraded);
        assert_eq!(observed.runtime.pid, self.agent.runtime.pid);
        assert_eq!(
            observed.runtime.desired_state,
            self.agent.runtime.desired_state
        );
        assert_eq!(observed.runtime.started_at, self.agent.runtime.started_at);
        assert_eq!(observed.runtime.stopped_at, self.agent.runtime.stopped_at);
        assert_eq!(observed.runtime.health_status.as_deref(), Some(health));
        assert_eq!(observed.runtime.last_capabilities_json, capabilities);
    }
}

async fn hung_probe_releases_stop(reply: Reply, desired_state: DesiredState) {
    let mut tcp = TcpFixture::new(reply).await;
    let Some(f) = JavaFixture::new(tcp.port, desired_state).await else {
        return;
    };
    let started = Instant::now();
    let health = {
        let runtime = f.runtime.clone();
        let agent = f.agent.clone();
        tokio::spawn(async move { runtime.health(&agent).await })
    };
    tcp.await_request().await;
    // health already holds the lifecycle lock when stop starts waiting for it.
    let stopped = timeout(Duration::from_secs(6), f.runtime.stop(&f.agent))
        .await
        .expect("hung Java health deadlocked concurrent stop");
    assert!(matches!(stopped, Err(AppError::Unavailable(_))));
    let health = timeout(Duration::from_secs(2), health)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(health.status, AgentStatus::Degraded);
    assert!(started.elapsed() < Duration::from_secs(8));
    f.assert_preserved("unhealthy", json!({"previous": "capabilities"}))
        .await;
}

#[tokio::test]
async fn runtime_readiness_java_hung_headers_release_concurrent_stop() {
    hung_probe_releases_stop(Reply::HungHeaders, DesiredState::Running).await;
}

#[tokio::test]
async fn runtime_readiness_java_hung_body_release_concurrent_stop_without_reviving_intent() {
    hung_probe_releases_stop(Reply::HungBody, DesiredState::Stopped).await;
}

async fn json_probe(body: String, chunked: bool, healthy: bool) {
    let mut tcp = TcpFixture::new(Reply::Json {
        body: body.clone(),
        chunked,
    })
    .await;
    let Some(f) = JavaFixture::new(tcp.port, DesiredState::Stopped).await else {
        return;
    };
    let health = timeout(Duration::from_secs(6), f.runtime.health(&f.agent))
        .await
        .expect("Java JSON readiness probe exceeded its budget")
        .unwrap();
    tcp.await_request().await;
    assert_eq!(
        health.status,
        AgentStatus::Degraded,
        "HTTP UP cannot attest process ownership"
    );
    assert!(matches!(
        timeout(Duration::from_secs(2), f.runtime.stop(&f.agent))
            .await
            .unwrap(),
        Err(AppError::Unavailable(_))
    ));
    let capabilities = if healthy {
        serde_json::from_str(&body).unwrap()
    } else {
        json!({"previous": "capabilities"})
    };
    f.assert_preserved(if healthy { "healthy" } else { "unhealthy" }, capabilities)
        .await;
}

#[tokio::test]
async fn runtime_readiness_java_rejects_oversized_known_and_streamed_up_bodies() {
    let body = json!({"status": "UP", "padding": "x".repeat(16_384)}).to_string();
    assert!(body.len() > 16_384);
    for chunked in [false, true] {
        json_probe(body.clone(), chunked, false).await;
    }
}

#[tokio::test]
async fn runtime_readiness_java_rejects_malformed_and_non_up_json() {
    for body in ["{", r#"{"status":true}"#, r#"{"status":"DOWN"}"#] {
        json_probe(body.into(), false, false).await;
    }
}

#[tokio::test]
async fn runtime_readiness_java_accepts_normal_up_without_process_or_intent_claims() {
    json_probe(r#"{"status":"UP"}"#.into(), false, true).await;
}
