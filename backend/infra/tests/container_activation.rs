use app::{FleetRepository, container_activation::*, container_runtime::*};
use domain::{Agent, AgentKind, AgentProductRole, AgentRole, AgentStatus, CreateAgentRequest};
use infra::PostgresFleetRepository;
use sea_orm::{
    ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement, TransactionTrait,
};
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

fn hash(value: &impl serde::Serialize) -> String {
    use sha2::{Digest, Sha256};
    let mut value = serde_json::to_value(value).unwrap();
    value.sort_all_objects();
    hex::encode(Sha256::digest(serde_json::to_vec(&value).unwrap()))
}

fn recovered_command(l: &ContainerLaunch) -> ContainerRecoveryCommand {
    let r = &l.prepared.container.registration;
    let m = &l.prepared.container.mapped.as_ref().unwrap().mapping;
    let mut launch = json!(l);
    launch.as_object_mut().unwrap().remove("state");
    let mut physical = m.snapshot.clone();
    physical.started_at = "2026-10-09T11:00:00Z".into();
    physical.init_pid = 101;
    ContainerRecoveryCommand {
        request: ContainerRecoveryRequest {
            id: Uuid::new_v4(),
            launch_id: r.generation,
            agent_id: l.prepared.agent_id,
            original_controller_id: l.controller_id,
            controller_id: Uuid::new_v4(),
            predecessor_id: None,
            launch_sha256: hash(&launch),
            mapping_sha256: hash(m),
            registration_sha256: hash(r),
            controller_snapshot: physical,
            agent_pid: 42,
        },
        epoch: 1,
        lease_version: 1,
        lease_expires_at: (chrono::Utc::now() + chrono::Duration::seconds(30))
            .to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
    }
}

fn observed(l: &ContainerLaunch, exited: bool) -> Value {
    let r = &l.prepared.container.registration;
    json!({"contract_version":3,"operation_id":r.operation_id,"container_id":r.container_id,"resource_id":r.resource_id,"generation":r.generation,
        "registration_sha256":hash(r),"state":"observed","observation":if exited {"namespace_exited"} else {"running"},"snapshot":l.snapshot})
}

async fn recovered_ack(
    repo: &PostgresFleetRepository,
    l: &ContainerLaunch,
    c: &ContainerRecoveryCommand,
) -> RecoveredProof {
    let observation = observed(l, l.state == "exited");
    let ack = json!({"contract_version":1,"state":"controller_recovered","request_sha256":hash(c),"recovery":c,
        "witness":{"contract_version":1,"state":"controller_restart_observed","original_mapping_sha256":c.request.mapping_sha256,
        "registration_sha256":c.request.registration_sha256,"original_controller_snapshot":l.prepared.container.mapped.as_ref().unwrap().mapping.snapshot,
        "current_controller_snapshot":c.request.controller_snapshot,"receipt":observation}});
    repo.acknowledge_container_recovery(c, ack).await.unwrap();
    repo.acknowledge_container_lease(c,json!({"ack":{"state":"controller_heartbeat","recovery_id":c.request.id,"lease_version":c.lease_version,
        "lease_expires_at":c.lease_expires_at},"receipt":observation})).await.unwrap();
    RecoveredProof {
        lease: c.clone(),
        observation,
    }
}

async fn recovered_step(
    repo: &PostgresFleetRepository,
    record: &mut Activation,
    proof: &RecoveredProof,
    phase: Phase,
) {
    let mut next = record.clone();
    next.phase = phase;
    repo.authorize_recovered_activation(record, proof)
        .await
        .unwrap();
    repo.advance_recovered_activation(record, &next, proof)
        .await
        .unwrap();
    *record = next;
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_ACTIVATION_TEST_DATABASE_URL"]
async fn recovered_activation_requires_original_plan_current_lease_and_exact_cas() {
    let (repo, db, a, mut record) = fixture_mode(true, true).await;
    let l = record.claim.previous.clone();
    let c = recovered_command(&l);
    repo.claim_container_recovery(&l, &c).await.unwrap();
    let missing = RecoveredProof {
        lease: c.clone(),
        observation: observed(&l, false),
    };
    assert!(
        repo.authorize_recovered_activation(&record, &missing)
            .await
            .is_err()
    );
    let proof = recovered_ack(&repo, &l, &c).await;
    repo.authorize_recovered_activation(&record, &proof)
        .await
        .unwrap();
    let mut next = record.clone();
    next.phase = Phase::StoppingPrevious;
    assert!(
        repo.advance_container_activation(&record, &next)
            .await
            .is_err()
    );
    for key in ["owner", "generation", "version", "snapshot", "plan"] {
        let mut p = proof.clone();
        let mut a = record.clone();
        match key {
            "owner" => p.lease.request.controller_id = Uuid::new_v4(),
            "generation" => p.lease.request.launch_id = Uuid::new_v4(),
            "version" => p.lease.lease_version += 1,
            "snapshot" => p.observation["snapshot"]["init_pid"] = json!(999),
            _ => a.claim.intent_sha256 = "f".repeat(64),
        }
        assert!(
            repo.authorize_recovered_activation(&a, &p).await.is_err(),
            "{key}"
        );
    }
    let actor = Uuid::new_v4();
    let session = Uuid::new_v4();
    let run = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash) VALUES($1,$2,$2,'recovered drain','!')",
        [actor.into(),format!("recovered-{actor}@test.invalid").into()])).await.unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO agent_sessions(id,agent_id,user_id,title,state) VALUES($1,$2,$3,'recovered drain','active')",
        [session.into(),a.id.into(),actor.into()])).await.unwrap();
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
        assert!(
            repo.authorize_recovered_activation(&record, &proof)
                .await
                .is_err()
        );
        assert!(
            repo.advance_recovered_activation(&record, &next, &proof)
                .await
                .is_err()
        );
    }
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE session_agent_runs SET state='completed' WHERE id=$1",
        [run.into()],
    ))
    .await
    .unwrap();
    repo.advance_recovered_activation(&record, &next, &proof)
        .await
        .unwrap();
    assert!(
        repo.advance_recovered_activation(&record, &next, &proof)
            .await
            .is_err()
    );
    record = next;
    assert_eq!(record.claim.controller_id, l.controller_id);
    assert!(
        db.execute_unprepared(
            "UPDATE runtime_container_activation_authorities SET plan_sha256=repeat('f',64)"
        )
        .await
        .is_err()
    );
    assert!(
        db.execute_unprepared("DELETE FROM runtime_container_activation_authorities")
            .await
            .is_err()
    );
    db.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_ACTIVATION_TEST_DATABASE_URL"]
async fn recovered_activation_publishes_only_after_readiness_and_renews_exited_original_anchor() {
    for rollback in [false, true] {
        let (_, db, _, _, _) = recovered_publication(rollback).await;
        db.close().await.unwrap();
    }
}

async fn recovered_publication(
    rollback: bool,
) -> (
    PostgresFleetRepository,
    DatabaseConnection,
    Agent,
    Activation,
    RecoveredProof,
) {
    let (repo, db, a, mut record) = fixture_mode(true, true).await;
    let l = record.claim.previous.clone();
    let c = recovered_command(&l);
    repo.claim_container_recovery(&l, &c).await.unwrap();
    let mut proof = recovered_ack(&repo, &l, &c).await;
    recovered_step(&repo, &mut record, &proof, Phase::StoppingPrevious).await;
    // Native original-stop ACK/exit is durable; DB still has the pre-CAS phase after restart.
    record = repo
        .get_container_activation(a.id, record.claim.revision)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(record.phase, Phase::StoppingPrevious);
    assert!(record.previous_stop.is_none());
    let mut next = record.clone();
    next.previous_stop = Some(stop(&l));
    next.phase = Phase::PreviousStopped;
    proof.observation = observed(&l, true);
    repo.advance_recovered_activation(&record, &next, &proof)
        .await
        .unwrap();
    record = next;
    recovered_step(&repo, &mut record, &proof, Phase::ApplyingCandidate).await;
    recovered_step(&repo, &mut record, &proof, Phase::PreparingCandidate).await;
    let mut child = l.clone();
    child.state = "claimed".into();
    child.snapshot = None;
    child.origin = None;
    child.stop_id = record.claim.candidate.stop_id;
    child.prepared.configuration_revision = Some(1);
    child.prepared.configuration_sha256 = Some(record.claim.configuration_sha256.clone());
    child.prepared.container.registration.generation = record.claim.candidate.generation;
    child.prepared.container.registration.operation_id = record.claim.candidate.operation_id;
    child.prepared.container.registration.container_id = "4".repeat(64);
    let mut next = record.clone();
    next.candidate = Some(child);
    next.phase = Phase::CandidatePrepared;
    repo.advance_recovered_activation(&record, &next, &proof)
        .await
        .unwrap();
    record = next;
    // Renewal/ACK is bound to the exited original, even though get_latest now returns candidate.
    let prior = repo
        .get_container_recovery(c.request.launch_id)
        .await
        .unwrap()
        .unwrap();
    let mut lease = c.clone();
    lease.lease_version += 1;
    lease.lease_expires_at = (chrono::Utc::now() + chrono::Duration::seconds(30))
        .to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    repo.claim_container_lease(&prior, &lease).await.unwrap();
    assert!(
        repo.authorize_recovered_activation(&record, &proof)
            .await
            .is_err()
    );
    repo.acknowledge_container_lease(&lease,json!({"ack":{"state":"controller_heartbeat","recovery_id":lease.request.id,
        "lease_version":lease.lease_version,"lease_expires_at":lease.lease_expires_at},"receipt":proof.observation})).await.unwrap();
    proof.lease = lease;
    recovered_step(&repo, &mut record, &proof, Phase::StartingCandidate).await;
    let mut next = record.clone();
    next.candidate = Some(running(record.candidate.clone().unwrap()));
    next.phase = Phase::CandidateRunning;
    repo.advance_recovered_activation(&record, &next, &proof)
        .await
        .unwrap();
    record = next;
    if rollback {
        let mut next = record.clone();
        next.phase = Phase::StoppingCandidate;
        next.candidate.as_mut().unwrap().state = "stopping".into();
        repo.advance_recovered_activation(&record, &next, &proof)
            .await
            .unwrap();
        record = next;
        let mut next = record.clone();
        next.phase = Phase::CandidateStopped;
        next.candidate_stop = Some(stop(record.candidate.as_ref().unwrap()));
        next.candidate.as_mut().unwrap().state = "exited".into();
        repo.advance_recovered_activation(&record, &next, &proof)
            .await
            .unwrap();
        record = next;
        recovered_step(&repo, &mut record, &proof, Phase::ApplyingRollback).await;
        recovered_step(&repo, &mut record, &proof, Phase::PreparingRollback).await;
        let mut restored = record.candidate.clone().unwrap();
        restored.state = "claimed".into();
        restored.snapshot = None;
        restored.origin = None;
        restored.stop_id = record.claim.rollback.stop_id;
        restored.prepared.configuration_revision = record.claim.previous_revision;
        restored.prepared.configuration_sha256 = record.claim.previous_configuration_sha256.clone();
        restored.prepared.container.registration.generation = record.claim.rollback.generation;
        restored.prepared.container.registration.operation_id = record.claim.rollback.operation_id;
        restored.prepared.container.registration.container_id = "5".repeat(64);
        let mut next = record.clone();
        next.phase = Phase::RollbackPrepared;
        next.rollback = Some(restored);
        repo.advance_recovered_activation(&record, &next, &proof)
            .await
            .unwrap();
        record = next;
        recovered_step(&repo, &mut record, &proof, Phase::StartingRollback).await;
        let mut next = record.clone();
        next.phase = Phase::RollbackRunning;
        next.rollback = Some(running(record.rollback.clone().unwrap()));
        repo.advance_recovered_activation(&record, &next, &proof)
            .await
            .unwrap();
        record = next;
    }
    let mut next = record.clone();
    next.phase = if rollback {
        Phase::RollbackReady
    } else {
        Phase::CandidateReady
    };
    assert!(
        repo.advance_recovered_activation(&record, &next, &proof)
            .await
            .is_err()
    );
    next.readiness = Some(Readiness {
        generation: if rollback {
            record.claim.rollback.generation
        } else {
            record.claim.candidate.generation
        },
        files_sha256: if rollback {
            record.claim.previous_files_sha256.clone()
        } else {
            record.claim.files_sha256.clone()
        },
        capabilities_sha256: "9".repeat(64),
    });
    repo.advance_recovered_activation(&record, &next, &proof)
        .await
        .unwrap();
    record = next;
    recovered_step(
        &repo,
        &mut record,
        &proof,
        if rollback {
            Phase::RolledBack
        } else {
            Phase::Committed
        },
    )
    .await;
    repo.authorize_recovered_activation(&record, &proof)
        .await
        .unwrap();
    let head = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT effective_revision,draining FROM agent_config_heads WHERE agent_id=$1",
            [a.id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        head.try_get::<Option<i64>>("", "effective_revision")
            .unwrap(),
        if rollback { None } else { Some(1) }
    );
    assert!(!head.try_get::<bool>("", "draining").unwrap());
    assert_eq!(
        record
            .claim
            .previous
            .prepared
            .container
            .registration
            .generation,
        c.request.launch_id
    );
    assert_eq!(
        record.candidate.as_ref().unwrap().controller_id,
        l.controller_id
    );
    (repo, db, a, record, proof)
}

async fn origin_live(db: &impl ConnectionTrait, launch: &ContainerLaunch) -> bool {
    db.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT fleet_container_origin($1,$2,$3,$4) AS live",
        [
            launch.prepared.agent_id.into(),
            launch.origin.clone().unwrap().into(),
            launch.prepared.api_port.into(),
            json!({"fleet_container_generation":launch.prepared.container.registration.generation})
                .into(),
        ],
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "live")
    .unwrap()
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_ACTIVATION_TEST_DATABASE_URL"]
async fn recovered_terminal_draft_keeps_historical_custody_and_fenced_regular_stop_both_outcomes() {
    for rollback in [false, true] {
        let (repo, db, a, record, proof) = recovered_publication(rollback).await;
        let published = if rollback {
            record.rollback.clone()
        } else {
            record.candidate.clone()
        }
        .unwrap();
        assert!(origin_live(&db, &published).await);
        let actor:Uuid=db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT created_by_user_id FROM agent_config_revisions WHERE agent_id=$1 AND revision=1",[a.id.into()]))
            .await.unwrap().unwrap().try_get("","created_by_user_id").unwrap();
        let draft = repo
            .create_config_revision(
                a.id,
                domain::UpdateAgentConfigRequest {
                    config_json: json!({}),
                    soul_md: "next draft".into(),
                    env_json: json!({}),
                },
                actor,
            )
            .await
            .unwrap();
        assert_eq!(draft.revision, 2);
        assert!(origin_live(&db, &published).await);
        assert_eq!(
            repo.open_container_activation(a.id)
                .await
                .unwrap()
                .unwrap()
                .claim
                .id,
            record.claim.id
        );
        assert_eq!(
            repo.container_activation_for_launch(&published)
                .await
                .unwrap()
                .unwrap()
                .claim
                .id,
            record.claim.id
        );
        repo.authorize_recovered_activation(&record, &proof)
            .await
            .unwrap();
        // Requesting the next revision must not manufacture a new claim or child custody anchor.
        repo.validate_config_revision(a.id, 2, vec![])
            .await
            .unwrap();
        repo.request_config_activation(a.id, 2, actor)
            .await
            .unwrap();
        let claimed = repo.claim_config_activation().await.unwrap().unwrap();
        assert_eq!((claimed.agent_id, claimed.revision), (a.id, 2));
        let mut claim = record.claim.clone();
        claim.id = Uuid::new_v4();
        claim.revision = 2;
        claim.previous = published.clone();
        claim.configuration_sha256 = hash(&draft.snapshot);
        claim.previous_files_sha256 = record.readiness.as_ref().unwrap().files_sha256.clone();
        claim.previous_revision = published.prepared.configuration_revision;
        claim.previous_configuration_sha256 = published.prepared.configuration_sha256.clone();
        claim.candidate = generation();
        claim.rollback = generation();
        assert!(repo.claim_container_activation(&claim).await.is_err());
        assert!(
            repo.get_container_activation(a.id, 2)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            repo.container_activation_for_launch(&published)
                .await
                .unwrap()
                .unwrap()
                .claim
                .id,
            record.claim.id
        );
        assert!(
            repo.claim_container_recovery(&published, &recovered_command(&published))
                .await
                .is_err()
        );
        repo.authorize_recovered_activation(&record, &proof)
            .await
            .unwrap();
        // A stale/native-unproved caller cannot mutate the child even without child recovery rows.
        assert!(
            repo.advance_container_launch(
                &published,
                "stopping",
                published.snapshot.clone(),
                published.origin.clone()
            )
            .await
            .is_err()
        );
        repo.advance_recovered_container(&published, &proof.lease, "stopping")
            .await
            .unwrap();
        let mut stopping = published.clone();
        stopping.state = "stopping".into();
        assert_eq!(
            repo.container_activation_for_launch(&stopping)
                .await
                .unwrap()
                .unwrap()
                .claim
                .id,
            record.claim.id
        );
        repo.authorize_recovered_activation(&record, &proof)
            .await
            .unwrap();
        repo.advance_recovered_container(&stopping, &proof.lease, "exited")
            .await
            .unwrap();
        assert!(!origin_live(&db, &published).await);
        let head = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT effective_revision,draining FROM agent_config_heads WHERE agent_id=$1",
                [a.id.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            head.try_get::<Option<i64>>("", "effective_revision")
                .unwrap(),
            published.prepared.configuration_revision
        );
        assert!(head.try_get::<bool>("", "draining").unwrap());
        db.close().await.unwrap();
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_ACTIVATION_TEST_DATABASE_URL"]
async fn recovered_child_sql_origin_serializes_heartbeat_then_holds_unknown_expired_and_new_epoch()
{
    let (repo, db, a, record, proof) = recovered_publication(false).await;
    let child = record.candidate.clone().unwrap();
    let prior = repo
        .get_container_recovery(proof.lease.request.launch_id)
        .await
        .unwrap()
        .unwrap();
    let mut next = prior.lease.clone();
    next.lease_version += 1;
    next.lease_expires_at = (chrono::Utc::now() + chrono::Duration::seconds(30))
        .to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    let tx = db.begin().await.unwrap();
    assert!(origin_live(&tx, &child).await);
    let writer = PostgresFleetRepository::new(db.clone());
    let pending = next.clone();
    let (started, seen) = tokio::sync::oneshot::channel();
    let mut task = tokio::spawn(async move {
        started.send(()).unwrap();
        writer.claim_container_lease(&prior, &pending).await
    });
    seen.await.unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), &mut task)
            .await
            .is_err()
    );
    tx.commit().await.unwrap();
    task.await.unwrap().unwrap();
    assert!(!origin_live(&db, &child).await);
    assert!(
        repo.advance_recovered_container(&child, &proof.lease, "stopping")
            .await
            .is_err()
    );
    let mutation = db.begin().await.unwrap();
    mutation
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT set_config('fleet.container_recovery_id',$1,true)",
            [next.request.id.to_string().into()],
        ))
        .await
        .unwrap();
    assert!(
        mutation
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE runtime_container_launches SET state='stopping' WHERE generation=$1",
                [child.prepared.container.registration.generation.into()]
            ))
            .await
            .is_err()
    );
    mutation.rollback().await.unwrap();
    repo.acknowledge_container_lease(&next,json!({"ack":{"state":"controller_heartbeat","recovery_id":next.request.id,
        "lease_version":next.lease_version,"lease_expires_at":next.lease_expires_at},"receipt":proof.observation})).await.unwrap();
    assert!(origin_live(&db, &child).await);
    let mut successor = next.clone();
    successor.request.id = Uuid::new_v4();
    successor.request.controller_id = Uuid::new_v4();
    successor.request.predecessor_id = Some(next.request.id);
    successor.request.controller_snapshot.started_at = "2026-10-09T12:00:00Z".into();
    successor.request.controller_snapshot.init_pid += 1;
    successor.epoch += 1;
    successor.lease_version = 1;
    assert!(
        repo.claim_container_recovery(&record.claim.previous, &successor)
            .await
            .is_err()
    );
    let deadline = chrono::DateTime::parse_from_rfc3339(&next.lease_expires_at).unwrap();
    let wait = (deadline - chrono::Utc::now()).to_std().unwrap_or_default()
        + std::time::Duration::from_millis(10);
    tokio::time::sleep(wait).await;
    assert!(!origin_live(&db, &child).await);
    successor.lease_expires_at = (chrono::Utc::now() + chrono::Duration::seconds(30))
        .to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    repo.claim_container_recovery(&record.claim.previous, &successor)
        .await
        .unwrap();
    assert!(!origin_live(&db, &child).await);
    assert!(
        repo.authorize_recovered_activation(&record, &proof)
            .await
            .is_err()
    );
    let mut exited = record.claim.previous.clone();
    exited.state = "exited".into();
    let fresh = recovered_ack(&repo, &exited, &successor).await;
    assert!(!origin_live(&db, &child).await);
    repo.authorize_recovered_activation(&record, &fresh)
        .await
        .unwrap();
    assert!(origin_live(&db, &child).await);
    // Origin's outer SQL snapshot can predate a concurrent stop; recheck child state after its agent lock.
    let stopping = db.begin().await.unwrap();
    stopping
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM agents WHERE id=$1 FOR UPDATE",
            [a.id.into()],
        ))
        .await
        .unwrap();
    stopping
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT set_config('fleet.container_recovery_id',$1,true)",
            [successor.request.id.to_string().into()],
        ))
        .await
        .unwrap();
    stopping
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE runtime_container_launches SET state='stopping' WHERE generation=$1",
            [child.prepared.container.registration.generation.into()],
        ))
        .await
        .unwrap();
    let reader = db.clone();
    let original = child.clone();
    let (started, seen) = tokio::sync::oneshot::channel();
    let mut reading = tokio::spawn(async move {
        started.send(()).unwrap();
        origin_live(&reader, &original).await
    });
    seen.await.unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), &mut reading)
            .await
            .is_err()
    );
    stopping.commit().await.unwrap();
    assert!(!reading.await.unwrap());
    assert_eq!(
        repo.open_container_activation(a.id)
            .await
            .unwrap()
            .unwrap()
            .claim
            .id,
        record.claim.id
    );
    db.close().await.unwrap();
}

fn running(mut l: ContainerLaunch) -> ContainerLaunch {
    let r = &l.prepared.container.registration;
    l.state = "running".into();
    l.origin = Some(format!(
        "http://172.18.0.2:{}",
        l.prepared.api_port.unwrap()
    ));
    l.snapshot = Some(
        json!({"contract_version":r.contract_version,"container_id":r.container_id,"engine":r.engine,
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
    json!({"contract_version":r.contract_version,"operation_id":l.stop_id,"container_id":r.container_id,"resource_id":r.resource_id,
        "generation":r.generation,"snapshot_sha256":hash,"state":"observed","observation":"namespace_exited"})
}

async fn fixture() -> (
    PostgresFleetRepository,
    DatabaseConnection,
    Agent,
    Activation,
) {
    fixture_with_record(true).await
}

async fn fixture_with_record(
    recorded: bool,
) -> (
    PostgresFleetRepository,
    DatabaseConnection,
    Agent,
    Activation,
) {
    fixture_mode(recorded, false).await
}

async fn fixture_mode(
    recorded: bool,
    mapped: bool,
) -> (
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
    let mut l = launch(&a, &initial, owner, None, None);
    if mapped {
        let mapping:ContainerMapping=serde_json::from_value(json!({"state":"resolved",
            "controller":{"container_id":"9".repeat(64),"image_id":format!("sha256:{}","8".repeat(64)),"service":"fleet-backend"},
            "snapshot":{"container_id":"9".repeat(64),"inventory_sha256":"7".repeat(64),"init_pid":100,"started_at":"2026-10-09T10:00:00Z"},
            "engine":l.prepared.container.registration.engine,"local_root":"/agents","volume_name":"sdlc1_fleet_agents",
            "volume_sha256":"6".repeat(64),"input_policy_sha256":"5".repeat(64),"mounts":[]})).unwrap();
        l.prepared.container.registration.contract_version = 3;
        l.prepared.container.registration.mount_mapping_sha256 = Some(hash(&mapping));
        l.prepared.container.mapped = Some(MappedContainer {
            mapping,
            mapping_file: "/private/mapping.json".into(),
            attachment_journal: "/private/attachment.sqlite".into(),
            recovery_journal: "/private/recovery.sqlite".into(),
        });
    }
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
    let record = if recorded {
        repo.claim_container_activation(&claim).await.unwrap()
    } else {
        Activation::planned(claim)
    };
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
    let pending = repo.pending_container_activations(None).await.unwrap();
    assert!(pending.iter().any(|r| r.agent_id == a.id));
    assert!(
        repo.pending_container_activations(Some(a.id))
            .await
            .unwrap()
            .iter()
            .all(|r| r.agent_id > a.id)
    );
    db.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires own activation PostgreSQL database"]
async fn claimed_preplan_survives_restart_as_audited_hold_without_reclaim_or_drain_release() {
    let (repo, db, a, _) = fixture_with_record(false).await;
    let pending = repo.pending_container_activations(None).await.unwrap();
    let revision = pending.iter().find(|r| r.agent_id == a.id).unwrap();
    assert!(
        repo.get_container_activation(a.id, 1)
            .await
            .unwrap()
            .is_none()
    );
    let before = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT claimed_at FROM agent_config_revisions WHERE agent_id=$1 AND revision=1",
            [a.id.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<shared::Timestamp>("", "claimed_at")
        .unwrap();
    let launch = repo.get_container_launch(a.id).await.unwrap().unwrap();
    let hold = RecoveryHold::new(Uuid::new_v4(), &launch, None);
    assert_ne!(hold.controller_id, launch.controller_id);
    let (one, two) = tokio::join!(
        repo.hold_container_activation(revision, None, &launch, &hold),
        repo.hold_container_activation(revision, None, &launch, &hold)
    );
    one.unwrap();
    two.unwrap();
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT claimed_at,last_error,state FROM agent_config_revisions WHERE agent_id=$1 AND revision=1", [a.id.into()])).await.unwrap().unwrap();
    assert_eq!(
        row.try_get::<shared::Timestamp>("", "claimed_at").unwrap(),
        before
    );
    assert_eq!(row.try_get::<String>("", "state").unwrap(), "activating");
    assert_eq!(
        serde_json::from_str::<RecoveryHold>(&row.try_get::<String>("", "last_error").unwrap())
            .unwrap(),
        hold
    );
    let audits = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT count(*) AS n FROM audit_log WHERE entity_id=$1 AND action='agent_config.recovery_required'",
        [a.id.to_string().into()])).await.unwrap().unwrap();
    assert_eq!(audits.try_get::<i64>("", "n").unwrap(), 1);
    let mut foreign = hold.clone();
    foreign.generation = Uuid::new_v4();
    assert!(
        repo.hold_container_activation(revision, None, &launch, &foreign)
            .await
            .is_err()
    );
    assert!(repo.agent_is_draining(a.id).await.unwrap());
    assert!(
        repo.get_container_configuration(a.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.get_container_activation(a.id, 1)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        repo.get_container_launch(a.id)
            .await
            .unwrap()
            .unwrap()
            .controller_id,
        launch.controller_id
    );
    db.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires own activation PostgreSQL database"]
async fn recorded_unknown_restart_audits_original_command_and_rejects_stale_progress() {
    let (repo, db, a, mut activation) = fixture().await;
    stopped_previous(&repo, &mut activation).await;
    let stale = activation.clone();
    step(&repo, &mut activation, Phase::ApplyingCandidate).await;
    step(&repo, &mut activation, Phase::PreparingCandidate).await;
    let revision = repo
        .pending_container_activations(None)
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.agent_id == a.id)
        .unwrap();
    let launch = repo.get_container_launch(a.id).await.unwrap().unwrap();
    let hold = RecoveryHold::new(Uuid::new_v4(), &launch, Some(&activation));
    assert_eq!(hold.generation, activation.claim.candidate.generation);
    assert_eq!(hold.operation_id, activation.claim.candidate.operation_id);
    assert_eq!(hold.reason, RecoveryReason::UnknownOriginalEffect);
    repo.hold_container_activation(&revision, Some(&activation), &launch, &hold)
        .await
        .unwrap();
    let stale_hold = RecoveryHold::new(hold.controller_id, &launch, Some(&stale));
    assert!(
        repo.hold_container_activation(&revision, Some(&stale), &launch, &stale_hold)
            .await
            .is_err()
    );
    let mut changed = revision.clone();
    changed.snapshot.config.soul_md.push_str("changed");
    assert!(
        repo.hold_container_activation(&changed, Some(&activation), &launch, &hold)
            .await
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(
            repo.get_container_activation(a.id, 1)
                .await
                .unwrap()
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(&activation).unwrap()
    );
    assert!(repo.agent_is_draining(a.id).await.unwrap());
    assert!(
        repo.get_container_configuration(a.id)
            .await
            .unwrap()
            .is_none()
    );
    db.close().await.unwrap();
}
