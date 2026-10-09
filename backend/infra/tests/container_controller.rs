use app::{FleetRepository, container_runtime::*};
use domain::{Agent, AgentKind, AgentProductRole, AgentRole, AgentStatus, CreateAgentRequest};
use infra::PostgresFleetRepository;
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement};
use sea_orm_migration::MigratorTrait;
use serde_json::{Value, json};
use uuid::Uuid;

async fn fixture() -> (PostgresFleetRepository, DatabaseConnection, Agent) {
    let url = std::env::var("FLEET_CONTAINER_CONTROLLER_TEST_DATABASE_URL")
        .expect("own container controller PG database is mandatory");
    assert_eq!(
        reqwest::Url::parse(&url).unwrap().path(),
        "/fleet_container_controller_test"
    );
    let db = Database::connect(url).await.unwrap();
    migration::Migrator::up(&db, None).await.unwrap();
    let repo = PostgresFleetRepository::new(db.clone());
    repo.ensure_runtime_templates().await.unwrap();
    let a = repo
        .create_agent(
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                product_role: AgentProductRole::Executor,
                role: AgentRole::Developer,
                sdlc_role: None,
                display_name: "Container fixture".into(),
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
    (repo, db, a)
}

fn launch(a: &Agent) -> ContainerLaunch {
    ContainerLaunch {
        prepared: PreparedContainer {
            agent_id: a.id,
            paths: a.paths.clone(),
            api_port: a.api_port,
            configuration_revision: None,
            configuration_sha256: None,
            container: ContainerBinding {
                registration: ContainerRegistration {
                    contract_version: 2,
                    operation_id: Uuid::new_v4(),
                    container_id: "a".repeat(64),
                    resource_id: a.id,
                    generation: Uuid::new_v4(),
                    engine: ContainerEngineIdentity {
                        id: "original-engine".into(),
                        kernel_version: "original-kernel".into(),
                        server_version: "29".into(),
                    },
                    policy_sha256: "b".repeat(64),
                    inventory_sha256: "c".repeat(64),
                    running_inventory_sha256: "d".repeat(64),
                    compose_sha256: "e".repeat(64),
                    network_sha256: Some("f".repeat(64)),
                },
                policy: json!({}),
                compose: "/private/compose.json".into(),
                journal: "/private/start.sqlite".into(),
                stop_journal: "/private/stop.sqlite".into(),
                source_sha256: ["1".repeat(64), "2".repeat(64), "3".repeat(64)],
                context: "protected".into(),
            },
        },
        controller_id: Uuid::new_v4(),
        state: "claimed".into(),
        snapshot: None,
        origin: None,
        stop_id: Uuid::new_v4(),
    }
}

fn snapshot(l: &ContainerLaunch) -> Value {
    let r = &l.prepared.container.registration;
    json!({"contract_version":2,"container_id":r.container_id,"engine":r.engine,"policy_sha256":r.policy_sha256,
        "inventory_sha256":r.running_inventory_sha256,"network_sha256":r.network_sha256,"init_pid":123,"started_at":"2026-10-09T12:00:00.123456789Z"})
}

async fn valid(
    db: &DatabaseConnection,
    l: &ContainerLaunch,
    origin: &str,
    generation: Uuid,
) -> bool {
    db.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT fleet_container_origin($1,$2,$3,$4) AS valid",
        [
            l.prepared.agent_id.into(),
            origin.into(),
            l.prepared.api_port.into(),
            json!({"fleet_container_generation":generation}).into(),
        ],
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "valid")
    .unwrap()
}

#[tokio::test]
#[ignore = "requires own FLEET_CONTAINER_CONTROLLER_TEST_DATABASE_URL"]
async fn original_claim_ack_origin_and_stop_are_fenced() {
    let (repo, db, a) = fixture().await;
    let l = launch(&a);
    repo.claim_container_launch(&l).await.unwrap();
    assert!(repo.claim_container_launch(&launch(&a)).await.is_err());
    let origin = format!("http://172.20.0.2:{}", a.api_port.unwrap());
    let s = snapshot(&l);
    let mut foreign = l.clone();
    foreign.controller_id = Uuid::new_v4();
    assert!(
        repo.advance_container_launch(&foreign, "running", Some(s.clone()), Some(origin.clone()))
            .await
            .is_err()
    );
    let mut wrong = s.clone();
    wrong["inventory_sha256"] = json!("0".repeat(64));
    assert!(
        repo.advance_container_launch(&l, "running", Some(wrong), Some(origin.clone()))
            .await
            .is_err()
    );
    repo.advance_container_launch(&l, "running", Some(s.clone()), Some(origin.clone()))
        .await
        .unwrap();
    let running = repo.get_container_launch(a.id).await.unwrap().unwrap();
    assert!(
        valid(
            &db,
            &running,
            &origin,
            l.prepared.container.registration.generation
        )
        .await
    );
    assert!(!valid(&db, &running, &origin, Uuid::new_v4()).await);
    assert!(
        !valid(
            &db,
            &running,
            &origin.replace("172.20.0.2", "172.20.0.3"),
            l.prepared.container.registration.generation
        )
        .await
    );
    assert!(
        !valid(
            &db,
            &running,
            &origin.replace("172.20.0.2", "127.0.0.1"),
            l.prepared.container.registration.generation
        )
        .await
    );
    assert!(
        repo.advance_container_launch(&l, "running", Some(s.clone()), Some(origin.clone()))
            .await
            .is_err()
    );
    repo.advance_container_launch(&running, "stopping", Some(s.clone()), Some(origin.clone()))
        .await
        .unwrap();
    let stopping = repo.get_container_launch(a.id).await.unwrap().unwrap();
    assert_eq!(stopping.stop_id, l.stop_id);
    assert!(
        !valid(
            &db,
            &stopping,
            &origin,
            l.prepared.container.registration.generation
        )
        .await
    );
    repo.advance_container_launch(&stopping, "exited", Some(s), Some(origin))
        .await
        .unwrap();
    assert!(repo.claim_container_launch(&l).await.is_err());
    let next = launch(&a);
    repo.claim_container_launch(&next).await.unwrap();
    assert_eq!(
        repo.get_container_launch(a.id)
            .await
            .unwrap()
            .unwrap()
            .prepared
            .container
            .registration
            .generation,
        next.prepared.container.registration.generation
    );
    assert!(
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "DELETE FROM runtime_container_launches WHERE generation=$1",
            [l.prepared.container.registration.generation.into()]
        ))
        .await
        .is_err()
    );
}

#[tokio::test]
#[ignore = "requires own FLEET_CONTAINER_CONTROLLER_TEST_DATABASE_URL"]
async fn unresolved_run_holds_stop_and_new_claim() {
    let (repo, db, a) = fixture().await;
    let l = launch(&a);
    repo.claim_container_launch(&l).await.unwrap();
    let origin = format!("http://172.20.0.2:{}", a.api_port.unwrap());
    let s = snapshot(&l);
    repo.advance_container_launch(&l, "running", Some(s.clone()), Some(origin.clone()))
        .await
        .unwrap();
    let running = repo.get_container_launch(a.id).await.unwrap().unwrap();
    let owner = Uuid::new_v4();
    let session = Uuid::new_v4();
    let run = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash) VALUES($1,$2,$3,'Owner','disabled')",
        [owner.into(),format!("{owner}@example.test").into(),owner.to_string().into()])).await.unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO agent_sessions(id,agent_id,user_id,title,state) VALUES($1,$2,$3,'Unknown acceptance','active')",
        [session.into(),a.id.into(),owner.into()])).await.unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO session_agent_runs(id,session_id,agent_id,run_role,state,runtime_session_id) VALUES($1,$2,$3,'primary','pending',$4)",
        [run.into(),session.into(),a.id.into(),format!("fleet:{session}:{}",a.id).into()])).await.unwrap();
    assert!(
        repo.advance_container_launch(&running, "stopping", Some(s), Some(origin))
            .await
            .is_err()
    );
    assert_eq!(
        repo.get_container_launch(a.id)
            .await
            .unwrap()
            .unwrap()
            .state,
        "running"
    );
    let other = launch(&a);
    assert!(repo.claim_container_launch(&other).await.is_err());
    let state: String = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT state FROM session_agent_runs WHERE id=$1",
            [run.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "state")
        .unwrap();
    assert_eq!(state, "pending");
}
