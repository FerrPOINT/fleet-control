use super::{configuration, fixture};
use app::{AgentProvisioner, FleetRepository, RuntimeStatePatch};
use domain::{
    AgentKind, AgentProductRole, AgentRole, AgentStatus, CreateAgentRequest, DesiredState,
    SystemRole,
};
use infra::{FilesystemProvisioner, runtime::LocalRuntimeSupervisor};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde_json::json;
use shared::AppConfig;
use std::{path::PathBuf, sync::Arc};
use tokio::{task::JoinHandle, time::Duration};
use uuid::Uuid;

struct OwnedResources {
    root: PathBuf,
    server: Option<JoinHandle<()>>,
}

impl Drop for OwnedResources {
    fn drop(&mut self) {
        if let Some(server) = &self.server {
            server.abort();
        }
        // Only this fixture's UUID directory, never the configured runtime root.
        if self.root.exists() {
            std::fs::remove_dir_all(&self.root).unwrap();
        }
    }
}

async fn rejected_purge_preserves_runtime_and_files(draining: bool) {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET system_role='operator' WHERE id=$1",
        [owner.into()],
    ))
    .await
    .unwrap();
    let mut resources = OwnedResources {
        root: std::env::temp_dir().join(format!("fleet-runtime-purge-{}", Uuid::new_v4())),
        server: None,
    };
    let mut config = AppConfig::default();
    config.fleet.agents_root = resources.root.to_string_lossy().into_owned();
    repo.ensure_runtime_templates().await.unwrap();
    let created = repo
        .create_agent(
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                product_role: AgentProductRole::Executor,
                role: AgentRole::Developer,
                sdlc_role: None,
                display_name: "Purge preservation fixture".into(),
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
    if draining {
        repo.create_config_revision(created.id, configuration(), owner)
            .await
            .unwrap();
        // No activation is queued, so a background activator cannot claim this fixture.
        let updated = db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE agent_config_heads SET draining=true WHERE agent_id=$1",
                [created.id.into()],
            ))
            .await
            .unwrap();
        assert_eq!(updated.rows_affected(), 1);
    }
    repo.archive_agent(created.id).await.unwrap();
    let agent = repo
        .update_runtime_state(
            created.id,
            RuntimeStatePatch {
                status: AgentStatus::Archived,
                desired_state: DesiredState::Running,
                pid: Some(12345),
                health_status: Some("untracked".into()),
                health_detail: Some("test-only unconfirmed runtime".into()),
                last_capabilities_json: Some(json!({"previous": "capabilities"})),
                startup_command_redacted: None,
                started_at: Some(shared::now()),
                stopped_at: None,
            },
        )
        .await
        .unwrap();
    let agent_root = resources.root.join(&agent.name);
    let retained = agent_root.join("workspace/retained.txt");
    tokio::fs::create_dir_all(retained.parent().unwrap())
        .await
        .unwrap();
    let marker_path = agent_root.join(".fleet-agent.json");
    let marker = json!({"id": agent.id, "name": agent.name}).to_string();
    tokio::fs::write(&marker_path, &marker).await.unwrap();
    tokio::fs::write(&retained, b"must survive rejected purge")
        .await
        .unwrap();
    let storage = FilesystemProvisioner
        .storage_report(&agent, &config)
        .await
        .unwrap();
    assert!(storage.marker_verified);
    assert!(storage.retention.purge_eligible);
    assert_eq!(repo.agent_is_draining(agent.id).await.unwrap(), draining);

    let repo = Arc::new(repo);
    let config = Arc::new(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = Arc::new(LocalRuntimeSupervisor::new(
        config.clone(),
        repo.clone(),
        events.clone(),
    ));
    let (restart_tx, _) = tokio::sync::mpsc::channel(1);
    let ctx = Arc::new(app::AppContext::new(
        config,
        repo.clone(),
        Arc::new(FilesystemProvisioner),
        runtime,
        events,
        restart_tx,
    ));
    // Existing test auth boundary: a persisted operator injected via Extension.
    // Stop uses the real supervisor and PostgreSQL state, not a runtime mock.
    let router = axum::Router::new()
        .route(
            "/api/v1/agents/{agent_id}/purge-files",
            axum::routing::post(api::routes::agents::purge_agent_files),
        )
        .layer(axum::Extension(api::middleware::CurrentUser {
            id: owner,
            role: SystemRole::Operator,
            central_write: None,
            is_system_admin: false,
        }))
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "http://{}/api/v1/agents/{}/purge-files",
        listener.local_addr().unwrap(),
        agent.id
    );
    resources.server = Some(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .no_proxy()
        .build()
        .unwrap()
        .post(url)
        .json(&json!({"confirmation": agent.name}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        if draining {
            reqwest::StatusCode::CONFLICT
        } else {
            reqwest::StatusCode::SERVICE_UNAVAILABLE
        }
    );

    let observed = repo.get_agent(agent.id).await.unwrap();
    assert_eq!(observed.status, AgentStatus::Archived);
    assert_eq!(observed.updated_at, agent.updated_at);
    assert_eq!(observed.runtime.pid, Some(12345));
    assert_eq!(observed.runtime.desired_state, DesiredState::Running);
    assert_eq!(observed.runtime.started_at, agent.runtime.started_at);
    assert_eq!(observed.runtime.stopped_at, agent.runtime.stopped_at);
    assert_eq!(observed.runtime.health_status, agent.runtime.health_status);
    assert_eq!(
        observed.runtime.last_capabilities_json,
        agent.runtime.last_capabilities_json
    );
    assert_eq!(repo.agent_is_draining(agent.id).await.unwrap(), draining);
    assert!(agent_root.is_dir());
    assert_eq!(
        tokio::fs::read_to_string(marker_path).await.unwrap(),
        marker
    );
    assert_eq!(
        tokio::fs::read(retained).await.unwrap(),
        b"must survive rejected purge"
    );
    let successes = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT
                (SELECT count(*) FROM agent_events
                 WHERE agent_id=$1 AND event_type='agent.files_purged') AS events,
                (SELECT count(*) FROM audit_log
                 WHERE entity_type='agent' AND entity_id=$2
                   AND action='agent.files_purge') AS audits",
            [agent.id.into(), agent.id.to_string().into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(successes.try_get::<i64>("", "events").unwrap(), 0);
    assert_eq!(successes.try_get::<i64>("", "audits").unwrap(), 0);
}

#[tokio::test]
async fn runtime_purge_http_untracked_stop_preserves_archived_runtime_files_and_success_ledgers() {
    rejected_purge_preserves_runtime_and_files(false).await;
}

#[tokio::test]
async fn runtime_purge_http_draining_preserves_archived_runtime_files_and_success_ledgers() {
    rejected_purge_preserves_runtime_and_files(true).await;
}
