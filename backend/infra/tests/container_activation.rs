use app::{FleetRepository, container_activation::*, container_runtime::*};
use domain::{Agent, AgentKind, AgentProductRole, AgentRole, AgentStatus, CreateAgentRequest};
use infra::PostgresFleetRepository;
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement};
use sea_orm_migration::MigratorTrait;
use serde_json::{Value, json};
use uuid::Uuid;

fn launch(
    a: &Agent,
    identity: &Generation,
    owner: Uuid,
    revision: Option<i64>,
    hash: Option<String>,
) -> ContainerLaunch {
    serde_json::from_value(json!({"controller_id":owner,"state":"claimed","stop_id":identity.stop_id,"snapshot":null,"origin":null,
        "prepared":{"agent_id":a.id,"paths":a.paths,"api_port":a.api_port,"configuration_revision":revision,"configuration_sha256":hash,
        "container":{"registration":{"contract_version":2,"operation_id":identity.operation_id,"container_id":identity.generation.simple().to_string().repeat(2),
            "resource_id":a.id,"generation":identity.generation,"engine":{"ID":"engine","KernelVersion":"kernel","ServerVersion":"29"},
            "policy_sha256":"b".repeat(64),"inventory_sha256":"c".repeat(64),"running_inventory_sha256":"d".repeat(64),
            "compose_sha256":"e".repeat(64),"network_sha256":"f".repeat(64)},"policy":{"image_id":"sha256:immutable"},
            "compose":"/private/compose.json","journal":"/private/launch.sqlite","stop_journal":"/private/stop.sqlite",
            "source_sha256":["1".repeat(64),"2".repeat(64),"3".repeat(64)],"context":"protected"}}})).unwrap()
}

fn generation() -> Generation {
    Generation {
        generation: Uuid::new_v4(),
        operation_id: Uuid::new_v4(),
        stop_id: Uuid::new_v4(),
    }
}

fn running(mut l: ContainerLaunch) -> ContainerLaunch {
    let r = &l.prepared.container.registration;
    l.state = "running".into();
    l.origin = Some(format!(
        "http://172.18.0.2:{}",
        l.prepared.api_port.unwrap()
    ));
    l.snapshot = Some(
        json!({"contract_version":2,"container_id":r.container_id,"engine":r.engine,
        "policy_sha256":r.policy_sha256,"inventory_sha256":r.running_inventory_sha256,"network_sha256":r.network_sha256,
        "init_pid":42,"started_at":"2026-10-09T10:00:00Z"}),
    );
    l
}

fn stop(l: &ContainerLaunch) -> Value {
    // Fixtures contain only ASCII; canonical compact object order matches original Base.
    use sha2::{Digest, Sha256};
    let hash = hex::encode(Sha256::digest(
        serde_json::to_vec(l.snapshot.as_ref().unwrap()).unwrap(),
    ));
    let r = &l.prepared.container.registration;
    json!({"contract_version":2,"operation_id":l.stop_id,"container_id":r.container_id,"resource_id":r.resource_id,
        "generation":r.generation,"snapshot_sha256":hash,"state":"observed","observation":"namespace_exited"})
}

async fn fixture() -> (
    PostgresFleetRepository,
    DatabaseConnection,
    Agent,
    Activation,
) {
    let url = std::env::var("FLEET_CONTAINER_ACTIVATION_TEST_DATABASE_URL")
        .expect("own activation database required");
    assert_eq!(
        reqwest::Url::parse(&url).unwrap().path(),
        "/fleet_container_activation_test"
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
                display_name: "Activation fixture".into(),
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
    let owner = Uuid::new_v4();
    let initial = generation();
    let l = launch(&a, &initial, owner, None, None);
    let p = ContainerPreparationClaim {
        agent_id: a.id,
        generation: initial.generation,
        operation_id: initial.operation_id,
        paths: a.paths.clone(),
        api_port: a.api_port,
        configuration_revision: None,
        configuration_sha256: None,
        intent_sha256: "0".repeat(64),
    };
    repo.claim_container_preparation(&p).await.unwrap();
    repo.claim_container_preparation_delivery(&p).await.unwrap();
    repo.acknowledge_container_preparation(&p, &l.prepared)
        .await
        .unwrap();
    repo.claim_container_launch(&l).await.unwrap();
    let previous = running(l.clone());
    repo.advance_container_launch(
        &l,
        "running",
        previous.snapshot.clone(),
        previous.origin.clone(),
    )
    .await
    .unwrap();
    repo.update_agent_status(a.id, AgentStatus::Running)
        .await
        .unwrap();
    let actor = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,system_role,is_active) VALUES($1,$2,$2,'activation','!','operator',false)",
        [actor.into(),format!("activation-{actor}@test.invalid").into()])).await.unwrap();
    let snapshot = json!({"config":{"config_json":{},"soul_md":"\u{43f}\u{440}\u{438}\u{432}\u{435}\u{442} \u{1f600}","env_json":{}},"skills":[]});
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO agent_config_revisions(agent_id,revision,state,snapshot,created_by_user_id,claimed_at) VALUES($1,1,'activating',$2,$3,now())",
        [a.id.into(),snapshot.clone().into(),actor.into()])).await.unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO agent_config_heads(agent_id,desired_revision,draining) VALUES($1,1,true)",
        [a.id.into()],
    ))
    .await
    .unwrap();
    // Independent Base json.dumps(sort_keys=True, separators=(',', ':'), ensure_ascii=True).
    let configuration_sha256 =
        "a5de7dfacd6c2771ef639bb9cbbfe24b3f38b4eeb5170ad7f6c7a6c6b2404e69".into();
    let claim = Claim {
        id: Uuid::new_v4(),
        agent_id: a.id,
        controller_id: owner,
        revision: 1,
        previous_revision: None,
        configuration_sha256,
        previous_configuration_sha256: None,
        files_sha256: "a".repeat(64),
        previous_files_sha256: "b".repeat(64),
        intent_sha256: "c".repeat(64),
        candidate_intent_sha256: "d".repeat(64),
        rollback_intent_sha256: "e".repeat(64),
        previous,
        candidate: generation(),
        rollback: generation(),
    };
    let record = repo.claim_container_activation(&claim).await.unwrap();
    (repo, db, a, record)
}

async fn step(repo: &PostgresFleetRepository, old: &mut Activation, phase: Phase) {
    let mut next = old.clone();
    next.phase = phase;
    repo.advance_container_activation(old, &next).await.unwrap();
    *old = next;
}

async fn stopped_previous(repo: &PostgresFleetRepository, record: &mut Activation) {
    step(repo, record, Phase::StoppingPrevious).await;
    let mut next = record.clone();
    next.previous_stop = Some(stop(&record.claim.previous));
    next.phase = Phase::PreviousStopped;
    repo.advance_container_activation(record, &next)
        .await
        .unwrap();
    *record = next;
}

async fn candidate_running(repo: &PostgresFleetRepository, a: &Agent, record: &mut Activation) {
    stopped_previous(repo, record).await;
    step(repo, record, Phase::ApplyingCandidate).await;
    step(repo, record, Phase::PreparingCandidate).await;
    let mut next = record.clone();
    next.phase = Phase::CandidatePrepared;
    next.candidate = Some(launch(
        a,
        &record.claim.candidate,
        record.claim.controller_id,
        Some(record.claim.revision),
        Some(record.claim.configuration_sha256.clone()),
    ));
    repo.advance_container_activation(record, &next)
        .await
        .unwrap();
    *record = next;
    step(repo, record, Phase::StartingCandidate).await;
    let mut next = record.clone();
    next.phase = Phase::CandidateRunning;
    next.candidate = Some(running(record.candidate.clone().unwrap()));
    repo.advance_container_activation(record, &next)
        .await
        .unwrap();
    *record = next;
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_ACTIVATION_TEST_DATABASE_URL"]
async fn activation_cas_stop_proof_and_readiness_gate_effective_revision() {
    let (repo, db, a, mut record) = fixture().await;
    assert!(
        repo.finish_config_activation(a.id, 1, None, true)
            .await
            .is_err()
    );
    for sql in [
        "UPDATE agent_config_heads SET effective_revision=1,draining=false WHERE agent_id=$1",
        "UPDATE agent_config_heads SET draining=false WHERE agent_id=$1",
        "DELETE FROM runtime_container_activations WHERE agent_id=$1",
    ] {
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                [a.id.into()]
            ))
            .await
            .is_err()
        );
    }
    let mut next = record.clone();
    next.phase = Phase::StoppingPrevious;
    let (x, y) = tokio::join!(
        repo.advance_container_activation(&record, &next),
        repo.advance_container_activation(&record, &next)
    );
    assert_ne!(x.is_ok(), y.is_ok());
    record = next;
    let mut forged = record.clone();
    forged.phase = Phase::PreviousStopped;
    let mut receipt = stop(&record.claim.previous);
    receipt["operation_id"] = json!(Uuid::new_v4());
    forged.previous_stop = Some(receipt);
    assert!(
        repo.advance_container_activation(&record, &forged)
            .await
            .is_err()
    );
    let mut next = record.clone();
    next.phase = Phase::PreviousStopped;
    next.previous_stop = Some(stop(&record.claim.previous));
    repo.advance_container_activation(&record, &next)
        .await
        .unwrap();
    record = next;
    step(&repo, &mut record, Phase::ApplyingCandidate).await;
    step(&repo, &mut record, Phase::PreparingCandidate).await;
    let l = launch(
        &a,
        &record.claim.candidate,
        record.claim.controller_id,
        Some(1),
        Some(record.claim.configuration_sha256.clone()),
    );
    assert!(repo.claim_container_launch(&l).await.is_err());
    let mut next = record.clone();
    next.candidate = Some(l);
    next.phase = Phase::CandidatePrepared;
    repo.advance_container_activation(&record, &next)
        .await
        .unwrap();
    record = next;
    step(&repo, &mut record, Phase::StartingCandidate).await;
    let mut next = record.clone();
    next.candidate = Some(running(record.candidate.clone().unwrap()));
    next.phase = Phase::CandidateRunning;
    repo.advance_container_activation(&record, &next)
        .await
        .unwrap();
    record = next;
    assert!(
        repo.get_container_configuration(a.id)
            .await
            .unwrap()
            .is_none()
    );
    let mut next = record.clone();
    next.phase = Phase::CandidateReady;
    assert!(
        repo.advance_container_activation(&record, &next)
            .await
            .is_err()
    );
    next.readiness = Some(Readiness {
        generation: record.claim.candidate.generation,
        files_sha256: record.claim.files_sha256.clone(),
        capabilities_sha256: "f".repeat(64),
    });
    repo.advance_container_activation(&record, &next)
        .await
        .unwrap();
    record = next;
    step(&repo, &mut record, Phase::Committed).await;
    let effective = repo
        .get_container_configuration(a.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(effective.revision, 1);
    assert!(!effective.draining);
    db.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_ACTIVATION_TEST_DATABASE_URL"]
async fn rollback_requires_candidate_exit_and_preserves_previous_effective() {
    let (repo, db, a, mut record) = fixture().await;
    candidate_running(&repo, &a, &mut record).await;
    let mut ready = record.clone();
    ready.phase = Phase::CandidateReady;
    ready.readiness = Some(Readiness {
        generation: record.claim.candidate.generation,
        files_sha256: record.claim.files_sha256.clone(),
        capabilities_sha256: "f".repeat(64),
    });
    repo.advance_container_activation(&record, &ready)
        .await
        .unwrap();
    record = ready;
    step(&repo, &mut record, Phase::Committed).await;
    let snapshot =
        json!({"config":{"config_json":{},"soul_md":"second candidate","env_json":{}},"skills":[]});
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO agent_config_revisions(agent_id,revision,state,snapshot,created_by_user_id,claimed_at)
         SELECT agent_id,2,'activating',$2,created_by_user_id,now() FROM agent_config_revisions WHERE agent_id=$1 AND revision=1",
        [a.id.into(),snapshot.clone().into()])).await.unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agent_config_heads SET desired_revision=2,draining=true WHERE agent_id=$1",
        [a.id.into()],
    ))
    .await
    .unwrap();
    use sha2::{Digest, Sha256};
    let mut claim = record.claim.clone();
    claim.id = Uuid::new_v4();
    claim.revision = 2;
    claim.previous_revision = Some(1);
    claim.previous_configuration_sha256 = Some(claim.configuration_sha256.clone());
    claim.previous_files_sha256 = claim.files_sha256.clone();
    claim.files_sha256 = "9".repeat(64);
    claim.configuration_sha256 =
        hex::encode(Sha256::digest(serde_json::to_vec(&snapshot).unwrap()));
    claim.previous = record.candidate.clone().unwrap();
    claim.candidate = generation();
    claim.rollback = generation();
    record = repo.claim_container_activation(&claim).await.unwrap();
    candidate_running(&repo, &a, &mut record).await;
    let mut next = record.clone();
    next.phase = Phase::ApplyingRollback;
    assert!(
        repo.advance_container_activation(&record, &next)
            .await
            .is_err()
    );
    let mut next = record.clone();
    next.phase = Phase::StoppingCandidate;
    next.candidate.as_mut().unwrap().state = "stopping".into();
    repo.advance_container_activation(&record, &next)
        .await
        .unwrap();
    record = next;
    let mut next = record.clone();
    next.phase = Phase::CandidateStopped;
    next.candidate_stop = Some(stop(record.candidate.as_ref().unwrap()));
    next.candidate.as_mut().unwrap().state = "exited".into();
    repo.advance_container_activation(&record, &next)
        .await
        .unwrap();
    record = next;
    step(&repo, &mut record, Phase::ApplyingRollback).await;
    step(&repo, &mut record, Phase::PreparingRollback).await;
    let mut next = record.clone();
    next.phase = Phase::RollbackPrepared;
    next.rollback = Some(launch(
        &a,
        &record.claim.rollback,
        record.claim.controller_id,
        record.claim.previous_revision,
        record.claim.previous_configuration_sha256.clone(),
    ));
    repo.advance_container_activation(&record, &next)
        .await
        .unwrap();
    record = next;
    step(&repo, &mut record, Phase::StartingRollback).await;
    let mut next = record.clone();
    next.phase = Phase::RollbackRunning;
    next.rollback = Some(running(record.rollback.clone().unwrap()));
    repo.advance_container_activation(&record, &next)
        .await
        .unwrap();
    record = next;
    let mut next = record.clone();
    next.phase = Phase::RollbackReady;
    next.readiness = Some(Readiness {
        generation: record.claim.rollback.generation,
        files_sha256: record.claim.previous_files_sha256.clone(),
        capabilities_sha256: "f".repeat(64),
    });
    repo.advance_container_activation(&record, &next)
        .await
        .unwrap();
    record = next;
    step(&repo, &mut record, Phase::RolledBack).await;
    assert_eq!(
        repo.get_container_configuration(a.id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        1
    );
    assert!(!repo.agent_is_draining(a.id).await.unwrap());
    assert_eq!(
        repo.list_config_revisions(a.id).await.unwrap()[0].state,
        "failed"
    );
    db.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_ACTIVATION_TEST_DATABASE_URL"]
async fn unknown_preparation_and_foreign_claim_cannot_release_drain() {
    let (repo, db, a, mut record) = fixture().await;
    let owner = Uuid::new_v4();
    let session = Uuid::new_v4();
    let run = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash) VALUES($1,$2,$2,'run owner','!')",
        [owner.into(),format!("run-{owner}@test.invalid").into()])).await.unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO agent_sessions(id,agent_id,user_id,title,state) VALUES($1,$2,$3,'drain fence','active')",
        [session.into(),a.id.into(),owner.into()])).await.unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO session_agent_runs(id,session_id,agent_id,run_role,state,runtime_session_id) VALUES($1,$2,$3,'primary','pending',$4)",
        [run.into(),session.into(),a.id.into(),format!("fleet:{session}:{}",a.id).into()])).await.unwrap();
    for state in ["pending", "running", "waiting", "stopping"] {
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE session_agent_runs SET state=$2 WHERE id=$1",
            [run.into(), state.into()],
        ))
        .await
        .unwrap();
        let mut next = record.clone();
        next.phase = Phase::StoppingPrevious;
        assert!(
            repo.advance_container_activation(&record, &next)
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
    }
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE session_agent_runs SET state='completed' WHERE id=$1",
        [run.into()],
    ))
    .await
    .unwrap();
    let mut foreign = record.claim.clone();
    foreign.controller_id = Uuid::new_v4();
    assert!(repo.claim_container_activation(&foreign).await.is_err());
    stopped_previous(&repo, &mut record).await;
    step(&repo, &mut record, Phase::ApplyingCandidate).await;
    step(&repo, &mut record, Phase::PreparingCandidate).await;
    for phase in [
        Phase::ApplyingRollback,
        Phase::PreparingRollback,
        Phase::Committed,
        Phase::PreparingCandidate,
    ] {
        let mut next = record.clone();
        next.phase = phase;
        assert!(
            repo.advance_container_activation(&record, &next)
                .await
                .is_err()
        );
    }
    assert!(repo.agent_is_draining(a.id).await.unwrap());
    assert!(
        repo.get_container_configuration(a.id)
            .await
            .unwrap()
            .is_none()
    );
    let pending = repo
        .pending_container_activations(record.claim.controller_id)
        .await
        .unwrap();
    assert!(pending.iter().any(|r| r.agent_id == a.id));
    assert!(
        repo.pending_container_activations(Uuid::new_v4())
            .await
            .unwrap()
            .is_empty()
    );
    db.close().await.unwrap();
}
