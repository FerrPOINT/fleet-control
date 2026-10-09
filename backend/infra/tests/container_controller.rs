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
                    mount_mapping_sha256: None,
                },
                policy: json!({}),
                compose: "/private/compose.json".into(),
                journal: "/private/start.sqlite".into(),
                stop_journal: "/private/stop.sqlite".into(),
                source_sha256: ["1".repeat(64), "2".repeat(64), "3".repeat(64)],
                context: "protected".into(),
                mapped: None,
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
    json!({"contract_version":r.contract_version,"container_id":r.container_id,"engine":r.engine,"policy_sha256":r.policy_sha256,
        "inventory_sha256":r.running_inventory_sha256,"network_sha256":r.network_sha256,"init_pid":123,"started_at":"2026-10-09T12:00:00.123456789Z"})
}

fn hash(value: &impl serde::Serialize) -> String {
    use sha2::{Digest, Sha256};
    let mut v = serde_json::to_value(value).unwrap();
    v.sort_all_objects();
    hex::encode(Sha256::digest(serde_json::to_vec(&v).unwrap()))
}

fn mapped_launch(a: &Agent) -> ContainerLaunch {
    let mut l = launch(a);
    let mapping = ContainerMapping {
        state: "resolved".into(),
        controller: MappingController {
            container_id: "9".repeat(64),
            image_id: format!("sha256:{}", "8".repeat(64)),
            service: "fleet-backend".into(),
        },
        snapshot: ControllerSnapshot {
            container_id: "9".repeat(64),
            started_at: "2026-10-09T10:00:00Z".into(),
            init_pid: 100,
            inventory_sha256: "7".repeat(64),
        },
        engine: l.prepared.container.registration.engine.clone(),
        local_root: "/agents".into(),
        volume_name: "sdlc1_fleet_agents".into(),
        volume_sha256: "6".repeat(64),
        input_policy_sha256: "5".repeat(64),
        mounts: ["runtime", "config", "workspace", "logs"]
            .map(|s| ProjectedMount {
                mount_type: "bind".into(),
                source: format!(
                    "/var/lib/docker/volumes/sdlc1_fleet_agents/_data/{}/{s}",
                    a.name
                ),
                destination: format!("/{s}"),
                read_only: s == "runtime",
            })
            .to_vec(),
    };
    l.prepared.container.registration.contract_version = 3;
    l.prepared.container.registration.mount_mapping_sha256 = Some(hash(&mapping));
    l.prepared.container.mapped = Some(MappedContainer {
        mapping,
        mapping_file: "/private/mapping.json".into(),
        attachment_journal: "/private/attachment.sqlite".into(),
        recovery_journal: "/private/recovery.sqlite".into(),
    });
    l
}

fn recovery(l: &ContainerLaunch, seconds: i64) -> ContainerRecoveryCommand {
    let r = &l.prepared.container.registration;
    let m = &l.prepared.container.mapped.as_ref().unwrap().mapping;
    let mut identity = json!(l);
    identity.as_object_mut().unwrap().remove("state");
    let mut controller = m.snapshot.clone();
    controller.started_at = "2026-10-09T11:00:00Z".into();
    controller.init_pid = 101;
    ContainerRecoveryCommand {
        request: ContainerRecoveryRequest {
            id: Uuid::new_v4(),
            launch_id: r.generation,
            agent_id: l.prepared.agent_id,
            original_controller_id: l.controller_id,
            controller_id: Uuid::new_v4(),
            predecessor_id: None,
            launch_sha256: hash(&identity),
            mapping_sha256: hash(m),
            registration_sha256: hash(r),
            controller_snapshot: controller,
            agent_pid: 123,
        },
        epoch: 1,
        lease_version: 1,
        lease_expires_at: (chrono::Utc::now() + chrono::Duration::seconds(seconds))
            .to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
    }
}

fn observation(l: &ContainerLaunch) -> Value {
    let r = &l.prepared.container.registration;
    json!({"contract_version":3,"operation_id":r.operation_id,"container_id":r.container_id,"resource_id":r.resource_id,"generation":r.generation,
        "registration_sha256":hash(r),"state":"observed","observation":"running","snapshot":l.snapshot})
}

fn recovery_ack(l: &ContainerLaunch, c: &ContainerRecoveryCommand) -> Value {
    json!({"contract_version":1,"state":"controller_recovered","request_sha256":hash(c),"recovery":c,
        "witness":{"contract_version":1,"state":"controller_restart_observed","original_mapping_sha256":c.request.mapping_sha256,
            "registration_sha256":c.request.registration_sha256,"original_controller_snapshot":l.prepared.container.mapped.as_ref().unwrap().mapping.snapshot,
            "current_controller_snapshot":c.request.controller_snapshot,"receipt":observation(l)}})
}

fn lease_ack(l: &ContainerLaunch, c: &ContainerRecoveryCommand) -> Value {
    json!({"ack":{"state":"controller_heartbeat","recovery_id":c.request.id,"lease_version":c.lease_version,"lease_expires_at":c.lease_expires_at},"receipt":observation(l)})
}

async fn running_mapped(repo: &PostgresFleetRepository, a: &Agent) -> ContainerLaunch {
    let mut l = mapped_launch(a);
    repo.claim_container_launch(&l).await.unwrap();
    let s = snapshot(&l);
    let origin = format!("http://172.20.0.2:{}", a.api_port.unwrap());
    repo.advance_container_launch(&l, "running", Some(s.clone()), Some(origin.clone()))
        .await
        .unwrap();
    l.state = "running".into();
    l.snapshot = Some(s);
    l.origin = Some(origin);
    l
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_CONTROLLER_TEST_DATABASE_URL"]
async fn mapped_recovery_claim_fences_original_owner_until_both_native_receipts() {
    let (repo, db, a) = fixture().await;
    let l = running_mapped(&repo, &a).await;
    let c = recovery(&l, 30);
    let before = json!(repo.get_container_launch(a.id).await.unwrap().unwrap());
    repo.claim_container_recovery(&l, &c).await.unwrap();
    assert!(
        repo.claim_container_recovery(&l, &recovery(&l, 30))
            .await
            .is_err()
    );
    assert!(!valid(&db, &l, l.origin.as_ref().unwrap(), c.request.launch_id).await);
    assert!(
        repo.advance_container_launch(&l, "stopping", l.snapshot.clone(), l.origin.clone())
            .await
            .is_err()
    );
    assert!(
        repo.advance_recovered_container(&l, &c, "stopping")
            .await
            .is_err()
    );
    let mut foreign = recovery_ack(&l, &c);
    foreign["witness"]["receipt"]["snapshot"]["init_pid"] = json!(999);
    assert!(
        repo.acknowledge_container_recovery(&c, foreign)
            .await
            .is_err()
    );
    let ack = recovery_ack(&l, &c);
    repo.acknowledge_container_recovery(&c, ack.clone())
        .await
        .unwrap();
    repo.acknowledge_container_recovery(&c, ack).await.unwrap();
    assert!(!valid(&db, &l, l.origin.as_ref().unwrap(), c.request.launch_id).await);
    repo.acknowledge_container_lease(&c, lease_ack(&l, &c))
        .await
        .unwrap();
    assert!(valid(&db, &l, l.origin.as_ref().unwrap(), c.request.launch_id).await);
    let mut wrong = c.clone();
    wrong.request.controller_id = Uuid::new_v4();
    assert!(
        repo.advance_recovered_container(&l, &wrong, "stopping")
            .await
            .is_err()
    );
    assert_eq!(
        json!(repo.get_container_launch(a.id).await.unwrap().unwrap()),
        before
    );
    repo.advance_recovered_container(&l, &c, "stopping")
        .await
        .unwrap();
    let mut stopped = l.clone();
    stopped.state = "stopping".into();
    repo.advance_recovered_container(&stopped, &c, "exited")
        .await
        .unwrap();
    assert!(!valid(&db, &l, l.origin.as_ref().unwrap(), c.request.launch_id).await);
    assert!(
        db.execute_unprepared(&format!(
            "DELETE FROM runtime_container_recoveries WHERE id='{}'",
            c.request.id
        ))
        .await
        .is_err()
    );
    db.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_CONTROLLER_TEST_DATABASE_URL"]
async fn mapped_heartbeat_is_durable_single_step_and_unknown_delivery_holds_origin() {
    let (repo, db, a) = fixture().await;
    let l = running_mapped(&repo, &a).await;
    let c = recovery(&l, 30);
    repo.claim_container_recovery(&l, &c).await.unwrap();
    repo.acknowledge_container_recovery(&c, recovery_ack(&l, &c))
        .await
        .unwrap();
    repo.acknowledge_container_lease(&c, lease_ack(&l, &c))
        .await
        .unwrap();
    let previous = repo
        .get_container_recovery(c.request.launch_id)
        .await
        .unwrap()
        .unwrap();
    let mut next = c.clone();
    next.lease_version = 3;
    assert!(repo.claim_container_lease(&previous, &next).await.is_err());
    next.lease_version = 2;
    next.lease_expires_at = (chrono::Utc::now() + chrono::Duration::seconds(30))
        .to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    repo.claim_container_lease(&previous, &next).await.unwrap();
    assert!(!valid(&db, &l, l.origin.as_ref().unwrap(), c.request.launch_id).await);
    assert!(
        repo.acknowledge_container_lease(&c, lease_ack(&l, &c))
            .await
            .is_err()
    );
    assert!(
        repo.claim_container_recovery(&l, &recovery(&l, 30))
            .await
            .is_err()
    );
    repo.acknowledge_container_lease(&next, lease_ack(&l, &next))
        .await
        .unwrap();
    assert!(valid(&db, &l, l.origin.as_ref().unwrap(), c.request.launch_id).await);
    assert!(
        repo.advance_recovered_container(&l, &c, "stopping")
            .await
            .is_err()
    );
    assert!(repo.claim_container_lease(&previous, &next).await.is_err());
    db.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_CONTROLLER_TEST_DATABASE_URL"]
async fn mapped_expired_owner_requires_exact_predecessor_and_new_physical_restart() {
    let (repo, db, a) = fixture().await;
    let l = running_mapped(&repo, &a).await;
    let c = recovery(&l, 3);
    repo.claim_container_recovery(&l, &c).await.unwrap();
    repo.acknowledge_container_recovery(&c, recovery_ack(&l, &c))
        .await
        .unwrap();
    repo.acknowledge_container_lease(&c, lease_ack(&l, &c))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(3100)).await;
    assert!(!valid(&db, &l, l.origin.as_ref().unwrap(), c.request.launch_id).await);
    assert!(
        repo.advance_recovered_container(&l, &c, "stopping")
            .await
            .is_err()
    );
    let mut next = recovery(&l, 30);
    next.epoch = 2;
    next.request.predecessor_id = Some(c.request.id);
    assert!(repo.claim_container_recovery(&l, &next).await.is_err());
    next.request.controller_snapshot.started_at = "2026-10-09T12:00:00Z".into();
    next.request.controller_snapshot.init_pid = 102;
    let mut foreign = next.clone();
    foreign.request.predecessor_id = Some(Uuid::new_v4());
    assert!(repo.claim_container_recovery(&l, &foreign).await.is_err());
    repo.claim_container_recovery(&l, &next).await.unwrap();
    assert!(
        repo.acknowledge_container_lease(&c, lease_ack(&l, &c))
            .await
            .is_err()
    );
    repo.acknowledge_container_recovery(&next, recovery_ack(&l, &next))
        .await
        .unwrap();
    repo.acknowledge_container_lease(&next, lease_ack(&l, &next))
        .await
        .unwrap();
    assert!(valid(&db, &l, l.origin.as_ref().unwrap(), c.request.launch_id).await);
    assert_eq!(
        repo.get_container_launch(a.id)
            .await
            .unwrap()
            .unwrap()
            .controller_id,
        l.controller_id
    );
    let owner = Uuid::new_v4();
    let session = Uuid::new_v4();
    let run = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash) VALUES($1,$2,$3,'Owner','disabled')",
        [owner.into(),format!("{owner}@example.test").into(),owner.to_string().into()])).await.unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO agent_sessions(id,agent_id,user_id,title,state) VALUES($1,$2,$3,'Recovered unknown acceptance','active')",
        [session.into(),a.id.into(),owner.into()])).await.unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO session_agent_runs(id,session_id,agent_id,run_role,state,runtime_session_id) VALUES($1,$2,$3,'primary','pending',$4)",
        [run.into(),session.into(),a.id.into(),format!("fleet:{session}:{}",a.id).into()])).await.unwrap();
    assert!(
        repo.advance_recovered_container(&l, &next, "stopping")
            .await
            .is_err()
    );
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
    db.close().await.unwrap();
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
