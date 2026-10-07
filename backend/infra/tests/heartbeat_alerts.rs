use app::{AuditLogFilter, FleetRepository, HEARTBEAT_STALE_ALERT_KIND, RepositoryAlertService};
use chrono::{DateTime, Duration, Utc};
use domain::{AgentKind, AgentProductRole, AgentRole, AgentStatus, CreateAgentRequest, FleetAlert};
use infra::{PostgresFleetRepository, connect_database, run_migrations};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use shared::{AppConfig, DatabaseConfig};
use std::sync::Arc;
use uuid::Uuid;

struct Fixture {
    repo: Arc<PostgresFleetRepository>,
    db: DatabaseConnection,
    agent_id: Uuid,
    user_id: Uuid,
}

impl Fixture {
    async fn new() -> Option<Self> {
        let Ok(url) = std::env::var("FLEET_TEST_DATABASE_URL") else {
            eprintln!("FLEET_TEST_DATABASE_URL not configured; PostgreSQL heartbeat tests skipped");
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
        let db = connect_database(config.clone()).await.unwrap();
        let user_id = Uuid::new_v4();
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
             VALUES ($1,$2,$3,'Heartbeat test','disabled',false,'operator')",
            [user_id.into(), format!("{user_id}@example.test").into(), user_id.to_string().into()],
        )).await.unwrap();
        let repo = Arc::new(PostgresFleetRepository::new(
            connect_database(config).await.unwrap(),
        ));
        repo.ensure_runtime_templates().await.unwrap();
        let agent = repo
            .create_agent(
                CreateAgentRequest {
                    kind: AgentKind::Hermes,
                    product_role: AgentProductRole::Executor,
                    role: AgentRole::Developer,
                    sdlc_role: None,
                    display_name: "Heartbeat test agent".into(),
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
        repo.update_agent_status(agent.id, AgentStatus::Running)
            .await
            .unwrap();
        Some(Self {
            repo,
            db,
            agent_id: agent.id,
            user_id,
        })
    }

    fn service(&self) -> RepositoryAlertService {
        RepositoryAlertService {
            repository: self.repo.clone(),
        }
    }

    async fn heartbeat(&self, value: Option<DateTime<Utc>>) {
        self.db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE agent_runtime SET last_health_at=$2 WHERE agent_id=$1",
                [
                    self.agent_id.into(),
                    value.map(|ts| ts.fixed_offset()).into(),
                ],
            ))
            .await
            .unwrap();
    }

    fn alert(&self, kind: &str) -> FleetAlert {
        FleetAlert {
            id: Uuid::new_v4(),
            agent_id: Some(self.agent_id),
            kind: kind.into(),
            severity: "warning".into(),
            detail: serde_json::json!({}),
            state: "open".into(),
            opened_at: Utc::now().to_rfc3339(),
            resolved_at: None,
            acknowledged_at: None,
            acknowledged_by_user_id: None,
        }
    }

    async fn alerts(&self) -> Vec<FleetAlert> {
        self.repo
            .list_fleet_alerts(None)
            .await
            .unwrap()
            .into_iter()
            .filter(|alert| alert.agent_id == Some(self.agent_id))
            .collect()
    }

    async fn resolution_audits(&self) -> Vec<domain::AuditLogEntry> {
        self.repo
            .list_audit_log(AuditLogFilter {
                action: Some("fleet_alert.resolved".into()),
                entity_id: Some(self.agent_id.to_string()),
                ..AuditLogFilter::default()
            })
            .await
            .unwrap()
    }
}

#[tokio::test]
#[serial_test::serial]
async fn stale_incident_persists_deduplicates_and_recovers_without_status_transition() {
    let Some(fixture) = Fixture::new().await else {
        return;
    };
    fixture
        .heartbeat(Some(Utc::now() - Duration::minutes(30)))
        .await;
    fixture
        .service()
        .record_heartbeat_freshness()
        .await
        .unwrap();
    let incident = fixture.alerts().await.pop().unwrap();
    assert_eq!(incident.kind, HEARTBEAT_STALE_ALERT_KIND);
    assert_eq!(incident.state, "open");
    fixture
        .service()
        .record_heartbeat_freshness()
        .await
        .unwrap();
    assert_eq!(fixture.alerts().await.len(), 1);
    fixture
        .repo
        .acknowledge_fleet_alert(incident.id, fixture.user_id)
        .await
        .unwrap();
    fixture
        .service()
        .record_heartbeat_freshness()
        .await
        .unwrap();
    assert_eq!(fixture.alerts().await.len(), 1);
    assert_eq!(fixture.alerts().await[0].state, "acknowledged");

    let unrelated = fixture
        .repo
        .insert_fleet_alert(fixture.alert("agent_down"))
        .await
        .unwrap();
    fixture
        .heartbeat(Some(Utc::now() - Duration::seconds(1)))
        .await;
    fixture
        .service()
        .record_heartbeat_freshness()
        .await
        .unwrap();
    fixture
        .service()
        .record_heartbeat_freshness()
        .await
        .unwrap();
    let rows = fixture.alerts().await;
    let resolved = rows.iter().find(|row| row.id == incident.id).unwrap();
    assert_eq!(resolved.state, "resolved");
    assert!(resolved.resolved_at.is_some());
    assert_eq!(
        rows.iter()
            .find(|row| row.id == unrelated.id)
            .unwrap()
            .state,
        "open"
    );
    assert_eq!(fixture.resolution_audits().await.len(), 1);

    fixture
        .heartbeat(Some(Utc::now() - Duration::minutes(30)))
        .await;
    fixture
        .service()
        .record_heartbeat_freshness()
        .await
        .unwrap();
    let rows = fixture.alerts().await;
    assert_eq!(
        rows.iter()
            .filter(|row| row.kind == HEARTBEAT_STALE_ALERT_KIND)
            .count(),
        2
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.kind == HEARTBEAT_STALE_ALERT_KIND && row.state == "open")
            .count(),
        1
    );
}

#[tokio::test]
#[serial_test::serial]
async fn unknown_future_and_nonrunning_health_do_not_resolve_an_incident() {
    let Some(fixture) = Fixture::new().await else {
        return;
    };
    let incident = fixture
        .repo
        .insert_fleet_alert(fixture.alert(HEARTBEAT_STALE_ALERT_KIND))
        .await
        .unwrap();
    fixture
        .repo
        .acknowledge_fleet_alert(incident.id, fixture.user_id)
        .await
        .unwrap();
    for value in [None, Some(Utc::now() + Duration::hours(1))] {
        fixture.heartbeat(value).await;
        fixture
            .service()
            .record_heartbeat_freshness()
            .await
            .unwrap();
        assert_eq!(fixture.alerts().await[0].state, "acknowledged");
    }
    fixture
        .heartbeat(Some(Utc::now() - Duration::seconds(1)))
        .await;
    for status in [
        AgentStatus::Stopped,
        AgentStatus::Degraded,
        AgentStatus::Failed,
    ] {
        fixture
            .repo
            .update_agent_status(fixture.agent_id, status)
            .await
            .unwrap();
        fixture
            .service()
            .record_heartbeat_freshness()
            .await
            .unwrap();
        assert_eq!(fixture.alerts().await[0].state, "acknowledged");
    }
    assert!(fixture.resolution_audits().await.is_empty());
}

#[tokio::test]
#[serial_test::serial]
async fn concurrent_incident_creation_returns_one_persisted_active_identity() {
    let Some(fixture) = Fixture::new().await else {
        return;
    };
    let mut identity = None;
    for acknowledged in [false, true] {
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let repo = fixture.repo.clone();
            let incident = fixture.alert(HEARTBEAT_STALE_ALERT_KIND);
            tasks.push(tokio::spawn(async move {
                repo.insert_fleet_alert(incident).await.unwrap()
            }));
        }
        for task in tasks {
            let result = task.await.unwrap();
            assert_eq!(*identity.get_or_insert(result.id), result.id);
            assert_eq!(
                result.state,
                if acknowledged { "acknowledged" } else { "open" }
            );
        }
        assert_eq!(fixture.alerts().await.len(), 1);
        if !acknowledged {
            fixture
                .repo
                .acknowledge_fleet_alert(identity.unwrap(), fixture.user_id)
                .await
                .unwrap();
        }
    }
}

#[tokio::test]
#[serial_test::serial]
async fn explicit_health_recovery_resolves_the_canonical_acknowledged_incident() {
    let Some(fixture) = Fixture::new().await else {
        return;
    };
    let incident = fixture
        .repo
        .insert_fleet_alert(fixture.alert(HEARTBEAT_STALE_ALERT_KIND))
        .await
        .unwrap();
    fixture
        .repo
        .acknowledge_fleet_alert(incident.id, fixture.user_id)
        .await
        .unwrap();
    fixture
        .service()
        .record_health_transition(fixture.agent_id, Some("failed".into()), "running")
        .await
        .unwrap();
    fixture
        .service()
        .record_health_transition(fixture.agent_id, Some("failed".into()), "running")
        .await
        .unwrap();
    assert_eq!(fixture.alerts().await[0].state, "resolved");
    assert_eq!(fixture.resolution_audits().await.len(), 1);
}

#[tokio::test]
#[serial_test::serial]
async fn failed_resolution_audit_rolls_back_state_and_retry_redacts_payload() {
    let Some(fixture) = Fixture::new().await else {
        return;
    };
    fixture
        .repo
        .insert_fleet_alert(fixture.alert(HEARTBEAT_STALE_ALERT_KIND))
        .await
        .unwrap();
    let name = format!("heartbeat_audit_{}", Uuid::new_v4().simple());
    let function = format!(
        "CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
         IF NEW.action = 'fleet_alert.resolved' AND NEW.entity_id = '{}' THEN
         RAISE EXCEPTION 'intentional task-owned audit failure'; END IF; RETURN NEW; END; $$",
        fixture.agent_id
    );
    fixture
        .db
        .execute(Statement::from_string(DatabaseBackend::Postgres, function))
        .await
        .unwrap();
    fixture.db.execute(Statement::from_string(DatabaseBackend::Postgres, format!(
        "CREATE TRIGGER {name} BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION {name}()"
    ))).await.unwrap();
    let payload = serde_json::json!({"api_key": "test-only-sensitive-value"});
    assert!(
        fixture
            .repo
            .resolve_active_alerts_of_kind(
                fixture.agent_id,
                HEARTBEAT_STALE_ALERT_KIND,
                payload.clone()
            )
            .await
            .is_err()
    );
    assert_eq!(fixture.alerts().await[0].state, "open");
    assert!(fixture.resolution_audits().await.is_empty());
    fixture
        .db
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("DROP TRIGGER {name} ON audit_log"),
        ))
        .await
        .unwrap();
    fixture
        .db
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("DROP FUNCTION {name}()"),
        ))
        .await
        .unwrap();
    assert_eq!(
        fixture
            .repo
            .resolve_active_alerts_of_kind(fixture.agent_id, HEARTBEAT_STALE_ALERT_KIND, payload)
            .await
            .unwrap(),
        1
    );
    assert_eq!(fixture.alerts().await[0].state, "resolved");
    let audits = fixture.resolution_audits().await;
    assert_eq!(audits.len(), 1);
    assert!(
        !serde_json::to_string(&audits[0])
            .unwrap()
            .contains("test-only-sensitive-value")
    );
}
