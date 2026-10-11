use app::{FleetRepository, container_runtime::*};
use domain::{Agent, AgentKind, AgentProductRole, AgentRole, AgentStatus, CreateAgentRequest};
use infra::PostgresFleetRepository;
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement};
use sea_orm_migration::MigratorTrait;
use serde_json::{Value, json};
use uuid::Uuid;

async fn fixture() -> (
    PostgresFleetRepository,
    DatabaseConnection,
    Agent,
    ContainerPreparationClaim,
) {
    let url = std::env::var("FLEET_CONTAINER_PREPARATION_TEST_DATABASE_URL")
        .expect("own preparation database is mandatory");
    assert_eq!(
        reqwest::Url::parse(&url).unwrap().path(),
        "/fleet_container_preparation_test"
    );
    let db = Database::connect(url.clone()).await.unwrap();
    migration::Migrator::up(&db, None).await.unwrap();
    let repo = PostgresFleetRepository::new(Database::connect(url).await.unwrap());
    repo.ensure_runtime_templates().await.unwrap();
    let a = repo
        .create_agent(
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                product_role: AgentProductRole::Executor,
                role: AgentRole::Developer,
                sdlc_role: None,
                display_name: "Preparation fixture".into(),
                description: None,
                namespace_id: None,
                namespace_name: None,
                workflow_id: None,
                workflow_name: None,
                executor_ids: vec![],
            },
            &shared::AppConfig::default(),
        )
        .await
        .unwrap();
    repo.update_agent_status(a.id, AgentStatus::Ready)
        .await
        .unwrap();
    let a = repo.get_agent(a.id).await.unwrap();
    let c = ContainerPreparationClaim {
        agent_id: a.id,
        generation: Uuid::new_v4(),
        operation_id: Uuid::new_v4(),
        paths: a.paths.clone(),
        api_port: a.api_port,
        configuration_revision: None,
        configuration_sha256: None,
        intent_sha256: "a".repeat(64),
    };
    (repo, db, a, c)
}

fn receipt(c: &ContainerPreparationClaim) -> PreparedContainer {
    PreparedContainer {
        agent_id: c.agent_id,
        paths: c.paths.clone(),
        api_port: c.api_port,
        configuration_revision: c.configuration_revision,
        configuration_sha256: c.configuration_sha256.clone(),
        container: ContainerBinding {
            registration: ContainerRegistration {
                contract_version: 2,
                operation_id: c.operation_id,
                container_id: "b".repeat(64),
                resource_id: c.agent_id,
                generation: c.generation,
                engine: ContainerEngineIdentity {
                    id: "engine".into(),
                    kernel_version: "kernel".into(),
                    server_version: "29".into(),
                },
                policy_sha256: "c".repeat(64),
                inventory_sha256: "d".repeat(64),
                running_inventory_sha256: "e".repeat(64),
                compose_sha256: "f".repeat(64),
                network_sha256: Some("1".repeat(64)),
                mount_mapping_sha256: None,
            },
            policy: json!({}),
            compose: "/private/compose.json".into(),
            journal: "/private/launch.sqlite".into(),
            stop_journal: "/private/stop.sqlite".into(),
            source_sha256: ["2".repeat(64), "3".repeat(64), "4".repeat(64)],
            context: "protected".into(),
            mapped: None,
        },
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_PREPARATION_TEST_DATABASE_URL"]
async fn preparation_has_one_permit_and_original_intent_under_concurrency() {
    let (repo, db, _, c) = fixture().await;
    let record = repo.claim_container_preparation(&c).await.unwrap();
    assert!(!record.attempted && record.receipt.is_none());
    let (a, b) = tokio::join!(
        repo.claim_container_preparation_delivery(&c),
        repo.claim_container_preparation_delivery(&c)
    );
    assert_ne!(a.unwrap(), b.unwrap());
    let replay = repo.claim_container_preparation(&c).await.unwrap();
    assert!(replay.attempted && replay.receipt.is_none());
    assert!(!repo.claim_container_preparation_delivery(&c).await.unwrap());
    for field in [
        "generation",
        "operation",
        "intent",
        "credential",
        "configuration",
        "path",
    ] {
        let mut different = c.clone();
        match field {
            "generation" => different.generation = Uuid::new_v4(),
            "operation" => different.operation_id = Uuid::new_v4(),
            "intent" | "credential" => different.intent_sha256 = "f".repeat(64),
            "configuration" => different.configuration_revision = Some(99),
            _ => different.paths.config = "/foreign/config".into(),
        }
        assert!(repo.claim_container_preparation(&different).await.is_err());
    }
    db.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_PREPARATION_TEST_DATABASE_URL"]
async fn preparation_ack_is_atomic_original_and_immutable() {
    let (repo, db, _, c) = fixture().await;
    repo.claim_container_preparation(&c).await.unwrap();
    let original = receipt(&c);
    assert!(
        repo.acknowledge_container_preparation(&c, &original)
            .await
            .is_err()
    );
    assert!(repo.claim_container_preparation_delivery(&c).await.unwrap());
    let mut foreign = original.clone();
    foreign.container.registration.operation_id = Uuid::new_v4();
    assert!(
        repo.acknowledge_container_preparation(&c, &foreign)
            .await
            .is_err()
    );
    assert!(
        repo.get_container_preparation(c.agent_id)
            .await
            .unwrap()
            .unwrap()
            .receipt
            .is_none()
    );
    repo.acknowledge_container_preparation(&c, &original)
        .await
        .unwrap();
    repo.acknowledge_container_preparation(&c, &original)
        .await
        .unwrap();
    foreign = original.clone();
    foreign.container.registration.container_id = "9".repeat(64);
    assert!(
        repo.acknowledge_container_preparation(&c, &foreign)
            .await
            .is_err()
    );
    let saved = repo
        .get_container_preparation(c.agent_id)
        .await
        .unwrap()
        .unwrap()
        .receipt
        .unwrap();
    assert_eq!(
        serde_json::to_value(saved).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    for sql in [
        "DELETE FROM runtime_container_preparations WHERE agent_id=$1",
        "UPDATE runtime_container_preparations SET attempted=false WHERE agent_id=$1",
        "UPDATE runtime_container_preparations SET claim=jsonb_set(claim,'{intent_sha256}',to_jsonb(repeat('b',64))) WHERE agent_id=$1",
    ] {
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                [c.agent_id.into()]
            ))
            .await
            .is_err()
        );
    }
    db.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_PREPARATION_TEST_DATABASE_URL"]
async fn preparation_cannot_launch_without_exact_ack_or_create_replacement() {
    let (repo, db, a, c) = fixture().await;
    repo.claim_container_preparation(&c).await.unwrap();
    assert!(!repo.try_stop_unstarted_container(a.id).await.unwrap());
    assert!(repo.archive_agent(a.id).await.is_err());
    let l = ContainerLaunch {
        prepared: receipt(&c),
        controller_id: Uuid::new_v4(),
        state: "claimed".into(),
        snapshot: None,
        origin: None,
        stop_id: Uuid::new_v4(),
    };
    assert!(repo.claim_container_launch(&l).await.is_err());
    repo.claim_container_preparation_delivery(&c).await.unwrap();
    assert!(!repo.try_stop_unstarted_container(a.id).await.unwrap());
    assert!(repo.archive_agent(a.id).await.is_err());
    repo.acknowledge_container_preparation(&c, &l.prepared)
        .await
        .unwrap();
    assert!(repo.archive_agent(a.id).await.is_err());
    let mut changed = l.clone();
    changed.prepared.container.context = "foreign".into();
    assert!(repo.claim_container_launch(&changed).await.is_err());
    repo.claim_container_launch(&l).await.unwrap();
    assert!(repo.claim_container_preparation(&c).await.is_err());
    assert!(repo.claim_container_preparation_delivery(&c).await.is_err());
    let user = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash) VALUES($1,$2,$3,'Fixture','!')",
        [user.into(),format!("{user}@example.test").into(),user.to_string().into()])).await.unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO agent_config_revisions(agent_id,revision,state,snapshot,created_by_user_id) VALUES($1,1,'draft',$2,$3)",
        [c.agent_id.into(),json!({"config":{"config_json":{},"soul_md":"","env_json":{}},"skills":[]}).into(),user.into()])).await.unwrap();
    // Drafting remains possible; publishing a different effective revision does not.
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO agent_config_heads(agent_id,desired_revision) VALUES($1,1)",
        [c.agent_id.into()],
    ))
    .await
    .unwrap();
    for sql in [
        "UPDATE agent_config_heads SET effective_revision=1 WHERE agent_id=$1",
        "DELETE FROM agent_config_heads WHERE agent_id=$1",
    ] {
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                [c.agent_id.into()]
            ))
            .await
            .is_err()
        );
    }
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT claim,receipt FROM runtime_container_preparations WHERE agent_id=$1",
            [a.id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(
        !row.try_get::<Value>("", "claim")
            .unwrap()
            .to_string()
            .contains("API_SERVER_KEY")
    );
    let r = &l.prepared.container.registration;
    let snapshot = json!({"contract_version":2,"container_id":r.container_id,
        "engine":r.engine,"policy_sha256":r.policy_sha256,
        "inventory_sha256":r.running_inventory_sha256,"network_sha256":r.network_sha256,
        "init_pid":123,"started_at":"2026-10-09T12:00:00.123456789Z"});
    let origin = format!("http://172.18.0.2:{}", a.api_port.unwrap());
    let mut l = l;
    for state in ["running", "stopping", "exited"] {
        repo.advance_container_launch(&l, state, Some(snapshot.clone()), Some(origin.clone()))
            .await
            .unwrap();
        l.state = state.into();
        l.snapshot = Some(snapshot.clone());
        l.origin = Some(origin.clone());
    }
    assert!(!repo.try_stop_unstarted_container(a.id).await.unwrap());
    let before = repo.get_container_preparation(a.id).await.unwrap().unwrap();
    let before =
        json!({"claim":before.claim,"attempted":before.attempted,"receipt":before.receipt});
    assert_eq!(
        repo.archive_agent(a.id).await.unwrap().status,
        AgentStatus::Archived
    );
    let after = repo.get_container_preparation(a.id).await.unwrap().unwrap();
    assert_eq!(
        json!({"claim":after.claim,"attempted":after.attempted,"receipt":after.receipt}),
        before
    );
    db.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_PREPARATION_TEST_DATABASE_URL"]
async fn unstarted_stop_and_archive_serialize_preparation_claims() {
    use sea_orm::TransactionTrait;
    use std::sync::Arc;

    let (repo, db, a, c) = fixture().await;
    let repo = Arc::new(repo);
    assert!(repo.try_stop_unstarted_container(a.id).await.unwrap());
    assert_eq!(
        repo.get_agent(a.id)
            .await
            .unwrap()
            .runtime
            .health_status
            .as_deref(),
        Some("not_started")
    );
    let tx = db.begin().await.unwrap();
    tx.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id=$1 FOR UPDATE",
        [a.id.into()],
    ))
    .await
    .unwrap()
    .unwrap();
    let blocker: i32 = tx
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT pg_backend_pid() AS pid".to_string(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    let waiting = {
        let repo = repo.clone();
        let id = a.id;
        tokio::spawn(async move { repo.try_stop_unstarted_container(id).await })
    };
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let blocked: bool = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid))
                    AND query LIKE 'SELECT kind,status,archived_at,%') AS blocked",
                [blocker.into()])).await.unwrap().unwrap().try_get("", "blocked").unwrap();
            if blocked { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.expect("the Stop CAS must wait for the actual agent-row lock");
    // The same valid immutable claim that the production preparation transaction inserts.
    tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_container_preparations(agent_id,generation,operation_id,claim) VALUES($1,$2,$3,$4)",
        [c.agent_id.into(),c.generation.into(),c.operation_id.into(),serde_json::to_value(&c).unwrap().into()]))
        .await.unwrap();
    tx.commit().await.unwrap();
    assert!(!waiting.await.unwrap().unwrap());
    let before = repo.get_container_preparation(a.id).await.unwrap().unwrap();
    let before =
        json!({"claim":before.claim,"attempted":before.attempted,"receipt":before.receipt});
    assert!(repo.archive_agent(a.id).await.is_err());
    let after = repo.get_container_preparation(a.id).await.unwrap().unwrap();
    assert_eq!(
        json!({"claim":after.claim,"attempted":after.attempted,"receipt":after.receipt}),
        before
    );
    assert!(repo.claim_container_preparation_delivery(&c).await.unwrap());
    assert!(repo.archive_agent(a.id).await.is_err());
    drop(repo);
    db.close().await.unwrap();

    let (repo, db, a, c) = fixture().await;
    assert!(repo.try_stop_unstarted_container(a.id).await.unwrap());
    repo.archive_agent(a.id).await.unwrap();
    assert!(repo.claim_container_preparation(&c).await.is_err());
    assert!(repo.claim_container_preparation_delivery(&c).await.is_err());
    assert!(
        repo.get_container_preparation(a.id)
            .await
            .unwrap()
            .is_none()
    );
    db.close().await.unwrap();
}
