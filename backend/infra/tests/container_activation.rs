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
    use std::fmt::Write;

    let mut value = serde_json::to_value(value).unwrap();
    value.sort_all_objects();
    // Base receipts use compact sorted JSON with ensure_ascii=True.
    let json = serde_json::to_string(&value).unwrap();
    let mut ascii = String::with_capacity(json.len());
    for c in json.chars() {
        if c < '\u{7f}' {
            ascii.push(c);
        } else {
            for unit in c.encode_utf16(&mut [0; 2]) {
                write!(ascii, "\\u{unit:04x}").unwrap();
            }
        }
    }
    hex::encode(Sha256::digest(ascii.as_bytes()))
}

#[test]
fn activation_probe_hash_matches_base_unicode_snapshot() {
    use sha2::{Digest, Sha256};

    let value = json!({"config":{"config_json":{},"soul_md":"\u{43f}\u{440}\u{438}\u{432}\u{435}\u{442} \u{1f600}","env_json":{}},"skills":[]});
    // Independent Python json.dumps(sort_keys=True, separators=(',', ':'), ensure_ascii=True).
    assert_eq!(
        hash(&value),
        "a5de7dfacd6c2771ef639bb9cbbfe24b3f38b4eeb5170ad7f6c7a6c6b2404e69"
    );
    assert_ne!(
        hash(&value),
        hex::encode(Sha256::digest(serde_json::to_vec(&value).unwrap()))
    );
    let controls = json!({"z":{"\\":"\"\\\n\t\u{7f}","\u{1f600}":"/srv/\u{430}\u{433}\u{435}\u{43d}\u{442}\u{44b}"},"\u{e9}":"\u{2028}\u{2029}"});
    assert_eq!(
        hash(&controls),
        "9f8e9d6aa4038a3f5ee775ae03966955cbb132701da1266358b48d5af159b8a6"
    );
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

async fn assert_recovered_preconditions(
    repo: &PostgresFleetRepository,
    record: &Activation,
    proof: &RecoveredProof,
) {
    let anchor = record.claim.anchor();
    assert!(
        proof.lease.request.agent_id == record.claim.agent_id
            && proof.lease.request.launch_id == anchor.prepared.container.registration.generation
            && proof.lease.request.original_controller_id == record.claim.controller_id
            && proof.lease.request.controller_id != record.claim.controller_id,
        "activation_probe_request_scope_exact"
    );
    assert!(
        matches!(anchor.snapshot.as_ref(), Some(snapshot)
            if !snapshot.is_null() && proof.observation.get("snapshot") == Some(snapshot)),
        "activation_probe_observation_snapshot_exact"
    );
    let stored = repo
        .get_container_activation(record.claim.agent_id, record.claim.revision)
        .await;
    assert!(
        matches!(stored.as_ref(), Ok(Some(actual)) if json!(actual) == json!(record)),
        "activation_probe_stored_record_exact"
    );
    let url = std::env::var("FLEET_CONTAINER_ACTIVATION_TEST_DATABASE_URL")
        .unwrap_or_else(|_| panic!("activation_probe_database_configured"));
    assert!(
        reqwest::Url::parse(&url).is_ok_and(|url| url.path() == "/fleet_container_activation_test"),
        "activation_probe_owned_database"
    );
    let connection = Database::connect(url).await;
    assert!(connection.is_ok(), "activation_probe_database_connected");
    let db = connection.unwrap_or_else(|_| panic!("activation_probe_database_connected"));
    // One read-only snapshot diagnoses predicates; authorize still rechecks under locks.
    let snapshot = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres, r#"
WITH activation AS (
 SELECT record FROM runtime_container_activations WHERE id=$1 AND agent_id=$2
), recovery AS (
 SELECT id,command,lease,receipt,lease_receipt,expires_at FROM runtime_container_recoveries
 WHERE generation=$3 ORDER BY epoch DESC LIMIT 1
), configuration AS (
 SELECT h.desired_revision,h.effective_revision,h.draining,r.state,r.claimed_at,r.validation_errors,r.snapshot
 FROM agent_config_heads h JOIN agent_config_revisions r
 ON r.agent_id=h.agent_id AND r.revision=h.desired_revision WHERE h.agent_id=$2
), probe_clock AS (SELECT clock_timestamp() AS observed_at)
SELECT
 (SELECT snapshot FROM configuration) AS configuration_snapshot,
 EXISTS(SELECT 1 FROM activation WHERE record=$4::jsonb) AS activation_probe_record_snapshot_exact,
 EXISTS(SELECT 1 FROM recovery WHERE id=$5 AND lease=$6::jsonb) AS activation_probe_stored_lease_exact,
 EXISTS(SELECT 1 FROM recovery WHERE id=$5 AND command->'request'=$6::jsonb->'request'
   AND command->>'epoch'=$6::jsonb->>'epoch') AS activation_probe_stored_request_exact,
 EXISTS(SELECT 1 FROM recovery WHERE receipt IS NOT NULL) AS activation_probe_recovery_ack_present,
 EXISTS(SELECT 1 FROM recovery WHERE lease_receipt IS NOT NULL) AS activation_probe_lease_ack_present,
 EXISTS(SELECT 1 FROM recovery WHERE receipt->'recovery'=command
   AND lease_receipt#>>'{ack,recovery_id}'=id::text
   AND lease_receipt#>>'{ack,lease_version}'=lease->>'lease_version'
   AND lease_receipt#>>'{ack,lease_expires_at}'=lease->>'lease_expires_at') AS activation_probe_ack_binding_exact,
 EXISTS(SELECT 1 FROM recovery,probe_clock WHERE expires_at>observed_at
   AND expires_at=($6::jsonb->>'lease_expires_at')::timestamptz) AS activation_probe_live_expiry,
 EXISTS(SELECT 1 FROM runtime_container_launches WHERE generation=$3 AND agent_id=$2
   AND controller_id::text=$8::jsonb->>'controller_id' AND prepared=$8::jsonb->'prepared'
   AND snapshot=$8::jsonb->'snapshot' AND origin=$8::jsonb->>'origin'
   AND stop_id::text=$8::jsonb->>'stop_id') AS activation_probe_anchor_custody_exact,
 fleet_activation_anchor($3) AS activation_probe_anchor_valid,
 EXISTS(SELECT 1 FROM configuration WHERE desired_revision=$7 AND draining) AS activation_probe_desired_draining,
 EXISTS(SELECT 1 FROM configuration WHERE state='activating' AND claimed_at IS NOT NULL
   AND validation_errors='[]'::jsonb
   AND effective_revision::text IS NOT DISTINCT FROM $4::jsonb#>>'{claim,previous_revision}') AS activation_probe_configuration_valid,
 NOT EXISTS(SELECT 1 FROM session_agent_runs WHERE agent_id=$2
   AND state IN ('pending','running','waiting','stopping')) AS activation_probe_no_active_runs,
 NOT EXISTS(SELECT 1 FROM hermes_dispatch_journal WHERE agent_id=$2
   AND state IN ('prepared','submitted')) AS activation_probe_no_unfinished_dispatch,
 NOT EXISTS(SELECT 1 FROM message_dispatch_outbox WHERE agent_id=$2
   AND state IN ('dispatching','uncertain')) AS activation_probe_no_unknown_delivery,
 NOT EXISTS(SELECT 1 FROM runtime_container_recoveries r JOIN runtime_container_launches l USING(generation)
   WHERE l.agent_id=$2 AND l.generation<>$3) AS activation_probe_no_other_recovery
"#, [record.claim.id.into(), record.claim.agent_id.into(),
        anchor.prepared.container.registration.generation.into(), json!(record).into(),
        proof.lease.request.id.into(), json!(proof.lease).into(), record.claim.revision.into(), json!(anchor).into()])).await;
    let claim = &record.claim;
    // Diagnostic copies preserve production binds but take no row locks or authority.
    let checked = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT h.effective_revision,r.snapshot FROM agent_config_heads h JOIN agent_config_revisions r
            ON r.agent_id=h.agent_id AND r.revision=h.desired_revision
         WHERE h.agent_id=$1 AND h.desired_revision=$2 AND h.draining AND r.state='activating'
            AND r.claimed_at IS NOT NULL AND r.validation_errors='[]'::jsonb
            AND NOT EXISTS(SELECT 1 FROM session_agent_runs WHERE agent_id=$1 AND state IN ('pending','running','waiting','stopping'))
            AND NOT EXISTS(SELECT 1 FROM hermes_dispatch_journal WHERE agent_id=$1 AND state IN ('prepared','submitted'))
            AND NOT EXISTS(SELECT 1 FROM message_dispatch_outbox WHERE agent_id=$1 AND state IN ('dispatching','uncertain'))
            AND (NOT EXISTS(SELECT 1 FROM runtime_container_recoveries x JOIN runtime_container_launches l USING(generation) WHERE l.agent_id=$1)
                OR ($3::uuid IS NOT NULL AND fleet_activation_anchor($3)
                  AND NOT EXISTS(SELECT 1 FROM runtime_container_recoveries x JOIN runtime_container_launches l USING(generation) WHERE l.agent_id=$1 AND l.generation<>$3)))",
        [claim.agent_id.into(),claim.revision.into(),Some(anchor.prepared.container.registration.generation).into()])).await;
    let current = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT r.id FROM runtime_container_recoveries r
             WHERE r.generation=$3 AND r.lease=$4 AND
               (EXISTS(SELECT 1 FROM runtime_container_activations a WHERE a.id=$1 AND a.record=$2)
               OR ($5 AND NOT EXISTS(SELECT 1 FROM runtime_container_activations a WHERE a.agent_id=$6 AND a.revision=$7)
                   AND fleet_activation_lineage($8)))
             AND r.receipt IS NOT NULL AND r.lease_receipt IS NOT NULL AND r.expires_at>clock_timestamp()
             AND NOT EXISTS(SELECT 1 FROM runtime_container_recoveries n WHERE n.generation=r.generation AND n.epoch>r.epoch)",
        [claim.id.into(),json!(record).into(),anchor.prepared.container.registration.generation.into(),json!(proof.lease).into(),
            (record.phase==Phase::Planned && claim.lineage.is_some()).into(),claim.agent_id.into(),claim.revision.into(),json!(claim).into()])).await;
    let authority = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres, r#"
WITH proposed AS (
 SELECT $1::uuid AS activation_id,$2::uuid AS recovery_id,$3::uuid AS controller_id,
        $4::text AS plan_sha256,$5::jsonb AS claim
)
SELECT EXISTS(SELECT 1 FROM runtime_container_activations a JOIN runtime_container_recoveries r
    ON r.id=proposed.recovery_id WHERE a.id=proposed.activation_id AND a.record->'claim'=proposed.claim
    AND a.record#>>'{claim,intent_sha256}'=proposed.plan_sha256
    AND r.generation=fleet_activation_scope(jsonb_build_object('claim',proposed.claim))
    AND r.command#>>'{request,controller_id}'=proposed.controller_id::text
    AND r.receipt IS NOT NULL AND r.lease_receipt IS NOT NULL AND r.expires_at>clock_timestamp()
    AND NOT EXISTS(SELECT 1 FROM runtime_container_recoveries n WHERE n.generation=r.generation AND n.epoch>r.epoch)
    AND fleet_activation_anchor(r.generation)
    AND (a.record->>'phase' NOT IN ('committed','rolled_back') OR EXISTS(
       SELECT 1 FROM runtime_container_activation_authorities old WHERE old.activation_id=a.id AND old.claim=proposed.claim AND old.plan_sha256=proposed.plan_sha256)))
 AS allowed FROM proposed
"#, [claim.id.into(),proof.lease.request.id.into(),proof.lease.request.controller_id.into(),
        claim.intent_sha256.clone().into(),json!(claim).into()])).await;
    let closed = db.close().await;
    assert!(closed.is_ok(), "activation_probe_database_closed");
    assert!(snapshot.is_ok(), "activation_probe_select_succeeded");
    let row = snapshot
        .ok()
        .flatten()
        .expect("activation_probe_snapshot_present");
    assert!(
        row.try_get::<Value>("", "configuration_snapshot")
            .is_ok_and(|snapshot| hash(&snapshot) == record.claim.configuration_sha256),
        "activation_probe_configuration_snapshot_exact"
    );
    for predicate in [
        "activation_probe_record_snapshot_exact",
        "activation_probe_stored_lease_exact",
        "activation_probe_stored_request_exact",
        "activation_probe_recovery_ack_present",
        "activation_probe_lease_ack_present",
        "activation_probe_ack_binding_exact",
        "activation_probe_live_expiry",
        "activation_probe_anchor_custody_exact",
        "activation_probe_anchor_valid",
        "activation_probe_desired_draining",
        "activation_probe_configuration_valid",
        "activation_probe_no_active_runs",
        "activation_probe_no_unfinished_dispatch",
        "activation_probe_no_unknown_delivery",
        "activation_probe_no_other_recovery",
    ] {
        assert!(
            matches!(row.try_get::<bool>("", predicate), Ok(true)),
            "{predicate}"
        );
    }
    assert!(checked.is_ok(), "activation_probe_checked_query_ok");
    let checked = checked.unwrap_or_else(|_| panic!("activation_probe_checked_query_ok"));
    assert!(checked.is_some(), "activation_probe_checked_row_present");
    let checked = checked.unwrap_or_else(|| panic!("activation_probe_checked_row_present"));
    let revision = checked.try_get::<Option<i64>>("", "effective_revision");
    assert!(revision.is_ok(), "activation_probe_checked_revision_decode");
    assert!(
        matches!(revision, Ok(value) if value == claim.previous_revision),
        "activation_probe_checked_revision_predicate"
    );
    let configuration = checked.try_get::<Value>("", "snapshot");
    assert!(
        configuration.is_ok(),
        "activation_probe_checked_snapshot_decode"
    );
    assert!(
        configuration.is_ok_and(|value| hash(&value) == claim.configuration_sha256),
        "activation_probe_checked_snapshot_predicate"
    );
    assert!(current.is_ok(), "activation_probe_current_query_ok");
    assert!(
        matches!(current, Ok(Some(_))),
        "activation_probe_current_row_present"
    );
    assert!(authority.is_ok(), "activation_probe_authority_query_ok");
    let authority = authority.unwrap_or_else(|_| panic!("activation_probe_authority_query_ok"));
    assert!(
        authority.is_some(),
        "activation_probe_authority_row_present"
    );
    let authority = authority.unwrap_or_else(|| panic!("activation_probe_authority_row_present"));
    let allowed = authority.try_get::<bool>("", "allowed");
    assert!(
        allowed.is_ok(),
        "activation_probe_authority_predicate_decode"
    );
    assert!(
        matches!(allowed, Ok(true)),
        "activation_probe_authority_predicate"
    );
}

async fn recovered_step(
    repo: &PostgresFleetRepository,
    record: &mut Activation,
    proof: &RecoveredProof,
    phase: Phase,
) {
    let mut next = record.clone();
    next.phase = phase;
    assert_recovered_preconditions(repo, record, proof).await;
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
async fn recovered_authority_alias_repair_preserves_trigger_oid_and_exact_custody() {
    let (repo, db, _, record) = fixture_mode(true, true).await;
    let launch = &record.claim.previous;
    let command = recovered_command(launch);
    repo.claim_container_recovery(launch, &command)
        .await
        .unwrap();
    let proof = recovered_ack(&repo, launch, &command).await;
    assert_recovered_preconditions(&repo, &record, &proof).await;
    let before_ledger = migration::Migrator::get_migration_models(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.version, row.applied_at))
        .collect::<Vec<_>>();
    let query = || {
        Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_get_functiondef(to_regprocedure($1)) AS body,
         to_regprocedure($1)::oid::bigint AS oid",
            ["fleet_guard_activation_authority()".into()],
        )
    };
    let before = db.query_one(query()).await.unwrap().unwrap();
    let body: String = before.try_get("", "body").unwrap();
    let oid: i64 = before.try_get("", "oid").unwrap();
    assert_eq!(body.matches("prior_authority").count(), 4);

    // Reproduce migration019 in a transaction; rollback restores the repaired function.
    let tx = db.begin().await.unwrap();
    tx.execute_unprepared(&body.replace("prior_authority", "old"))
        .await
        .unwrap();
    let attempt = tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_container_activation_authorities(activation_id,recovery_id,controller_id,plan_sha256,claim)
         VALUES($1,$2,$3,$4,$5) ON CONFLICT(activation_id,recovery_id) DO NOTHING",
        [record.claim.id.into(),proof.lease.request.id.into(),proof.lease.request.controller_id.into(),
            record.claim.intent_sha256.clone().into(),json!(record.claim).into()])).await;
    let ambiguous = attempt.as_ref().err().is_some_and(|error| {
        let message = error.to_string();
        message.contains("old.activation_id") && message.contains("ambiguous")
    });
    tx.rollback().await.unwrap();
    assert!(ambiguous, "authority_alias_original_trigger_ambiguous");
    let after = db.query_one(query()).await.unwrap().unwrap();
    assert_eq!(after.try_get::<i64>("", "oid").unwrap(), oid);
    assert!(
        after
            .try_get::<String>("", "body")
            .is_ok_and(|actual| actual == body),
        "authority_alias_definition_restored"
    );
    for _ in 0..2 {
        repo.authorize_recovered_activation(&record, &proof)
            .await
            .unwrap();
    }
    let authority = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT count(*) AS count FROM runtime_container_activation_authorities
         WHERE activation_id=$1 AND recovery_id=$2 AND controller_id=$3 AND plan_sha256=$4 AND claim=$5",
        [record.claim.id.into(),proof.lease.request.id.into(),proof.lease.request.controller_id.into(),
            record.claim.intent_sha256.clone().into(),json!(record.claim).into()])).await.unwrap().unwrap();
    assert_eq!(authority.try_get::<i64>("", "count").unwrap(), 1);
    let repair = migration::Migrator::migrations()
        .into_iter()
        .find(|item| item.name() == "m20261010_000021_activation_authority_alias")
        .unwrap();
    let tx = db.begin().await.unwrap();
    let downgrade = repair
        .down(&sea_orm_migration::SchemaManager::new(&tx))
        .await;
    let refused = downgrade.as_ref().err().is_some_and(|error| {
        error
            .to_string()
            .contains("Recovered activation history prevents alias repair downgrade")
    });
    tx.rollback().await.unwrap();
    assert!(refused, "authority_alias_history_blocks_downgrade");
    let after_ledger = migration::Migrator::get_migration_models(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.version, row.applied_at))
        .collect::<Vec<_>>();
    assert_eq!(after_ledger, before_ledger);
    db.close().await.unwrap();
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
    assert_recovered_preconditions(&repo, &record, &proof).await;
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

async fn next_claim(
    repo: &PostgresFleetRepository,
    db: &DatabaseConnection,
    a: &Agent,
    parent: &Activation,
) -> Claim {
    let actor:Uuid=db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT created_by_user_id FROM agent_config_revisions WHERE agent_id=$1 AND revision=$2",
        [a.id.into(),parent.claim.revision.into()])).await.unwrap().unwrap().try_get("","created_by_user_id").unwrap();
    let draft = repo
        .create_config_revision(
            a.id,
            domain::UpdateAgentConfigRequest {
                config_json: json!({}),
                soul_md: format!("next after {}", parent.claim.revision),
                env_json: json!({}),
            },
            actor,
        )
        .await
        .unwrap();
    repo.validate_config_revision(a.id, draft.revision, vec![])
        .await
        .unwrap();
    repo.request_config_activation(a.id, draft.revision, actor)
        .await
        .unwrap();
    let claimed = repo.claim_config_activation().await.unwrap().unwrap();
    assert_eq!((claimed.agent_id, claimed.revision), (a.id, draft.revision));
    let published = if parent.phase == Phase::Committed {
        parent.candidate.as_ref()
    } else {
        parent.rollback.as_ref()
    }
    .unwrap();
    let mut c = parent.claim.clone();
    c.lineage = Some(Lineage {
        anchor: parent.claim.anchor().clone(),
        family_id: parent
            .claim
            .lineage
            .as_ref()
            .map_or(parent.claim.id, |l| l.family_id),
        predecessor_activation_id: parent.claim.id,
        predecessor_intent_sha256: parent.claim.intent_sha256.clone(),
    });
    c.id = Uuid::new_v4();
    c.revision = draft.revision;
    c.previous = published.clone();
    c.previous_revision = published.prepared.configuration_revision;
    c.previous_configuration_sha256 = published.prepared.configuration_sha256.clone();
    c.previous_files_sha256 = parent.readiness.as_ref().unwrap().files_sha256.clone();
    c.configuration_sha256 = hash(&draft.snapshot);
    c.intent_sha256 = hash(&json!({"next":c.id}));
    c.candidate = generation();
    c.rollback = generation();
    c
}

async fn publish_next(
    repo: &PostgresFleetRepository,
    a: &Agent,
    mut record: Activation,
    proof: &RecoveredProof,
    rollback: bool,
) -> Activation {
    recovered_step(repo, &mut record, proof, Phase::StoppingPrevious).await;
    // A restarted worker reads the durable original child stop; no new stop ID/permit.
    record = repo
        .get_container_activation(a.id, record.claim.revision)
        .await
        .unwrap()
        .unwrap();
    let mut next = record.clone();
    next.phase = Phase::PreviousStopped;
    next.previous_stop = Some(stop(&record.claim.previous));
    repo.advance_recovered_activation(&record, &next, proof)
        .await
        .unwrap();
    record = next;
    recovered_step(repo, &mut record, proof, Phase::ApplyingCandidate).await;
    recovered_step(repo, &mut record, proof, Phase::PreparingCandidate).await;
    let mut child = record.claim.previous.clone();
    child.state = "claimed".into();
    child.snapshot = None;
    child.origin = None;
    child.stop_id = record.claim.candidate.stop_id;
    child.prepared.configuration_revision = Some(record.claim.revision);
    child.prepared.configuration_sha256 = Some(record.claim.configuration_sha256.clone());
    child.prepared.container.registration.generation = record.claim.candidate.generation;
    child.prepared.container.registration.operation_id = record.claim.candidate.operation_id;
    child.prepared.container.registration.container_id = record
        .claim
        .candidate
        .generation
        .simple()
        .to_string()
        .repeat(2);
    let mut next = record.clone();
    next.phase = Phase::CandidatePrepared;
    next.candidate = Some(child);
    repo.advance_recovered_activation(&record, &next, proof)
        .await
        .unwrap();
    record = next;
    recovered_step(repo, &mut record, proof, Phase::StartingCandidate).await;
    let mut next = record.clone();
    next.phase = Phase::CandidateRunning;
    next.candidate = Some(running(record.candidate.clone().unwrap()));
    repo.advance_recovered_activation(&record, &next, proof)
        .await
        .unwrap();
    record = next;
    if rollback {
        let mut next = record.clone();
        next.phase = Phase::StoppingCandidate;
        next.candidate.as_mut().unwrap().state = "stopping".into();
        repo.advance_recovered_activation(&record, &next, proof)
            .await
            .unwrap();
        record = next;
        let mut next = record.clone();
        next.phase = Phase::CandidateStopped;
        next.candidate_stop = Some(stop(record.candidate.as_ref().unwrap()));
        next.candidate.as_mut().unwrap().state = "exited".into();
        repo.advance_recovered_activation(&record, &next, proof)
            .await
            .unwrap();
        record = next;
        recovered_step(repo, &mut record, proof, Phase::ApplyingRollback).await;
        recovered_step(repo, &mut record, proof, Phase::PreparingRollback).await;
        let mut restored = record.claim.previous.clone();
        restored.state = "claimed".into();
        restored.snapshot = None;
        restored.origin = None;
        restored.stop_id = record.claim.rollback.stop_id;
        restored.prepared.container.registration.generation = record.claim.rollback.generation;
        restored.prepared.container.registration.operation_id = record.claim.rollback.operation_id;
        restored.prepared.container.registration.container_id = record
            .claim
            .rollback
            .generation
            .simple()
            .to_string()
            .repeat(2);
        let mut next = record.clone();
        next.phase = Phase::RollbackPrepared;
        next.rollback = Some(restored);
        repo.advance_recovered_activation(&record, &next, proof)
            .await
            .unwrap();
        record = next;
        recovered_step(repo, &mut record, proof, Phase::StartingRollback).await;
        let mut next = record.clone();
        next.phase = Phase::RollbackRunning;
        next.rollback = Some(running(record.rollback.clone().unwrap()));
        repo.advance_recovered_activation(&record, &next, proof)
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
        repo.advance_recovered_activation(&record, &next, proof)
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
        capabilities_sha256: "8".repeat(64),
    });
    repo.advance_recovered_activation(&record, &next, proof)
        .await
        .unwrap();
    record = next;
    recovered_step(
        repo,
        &mut record,
        proof,
        if rollback {
            Phase::RolledBack
        } else {
            Phase::Committed
        },
    )
    .await;
    record
}

async fn sequential_publication(previous_rollback: bool) {
    for rollback in [false, true] {
        let (repo, db, a, parent, proof) = recovered_publication(previous_rollback).await;
        let frozen = json!(parent);
        let claim = next_claim(&repo, &db, &a, &parent).await;
        repo.authorize_recovered_activation(&parent, &proof)
            .await
            .unwrap();
        assert!(repo.claim_container_activation(&claim).await.is_err());
        let record = repo
            .claim_recovered_container_activation(&claim, &proof)
            .await
            .unwrap();
        let current = publish_next(&repo, &a, record, &proof, rollback).await;
        let published = if rollback {
            current.rollback.as_ref()
        } else {
            current.candidate.as_ref()
        }
        .unwrap();
        assert_eq!(published.controller_id, parent.claim.controller_id);
        assert_eq!(
            current
                .claim
                .anchor()
                .prepared
                .container
                .registration
                .generation,
            proof.lease.request.launch_id
        );
        assert_ne!(
            current
                .claim
                .previous
                .prepared
                .container
                .registration
                .generation,
            proof.lease.request.launch_id
        );
        assert_eq!(
            published.prepared.configuration_revision,
            if rollback {
                claim.previous_revision
            } else {
                Some(claim.revision)
            }
        );
        assert_eq!(
            published.prepared.configuration_sha256,
            if rollback {
                claim.previous_configuration_sha256.clone()
            } else {
                Some(claim.configuration_sha256.clone())
            }
        );
        assert!(origin_live(&db, published).await);
        assert!(!origin_live(&db, &claim.previous).await);
        assert_eq!(
            json!(
                repo.get_container_activation(a.id, parent.claim.revision)
                    .await
                    .unwrap()
                    .unwrap()
            ),
            frozen
        );
        assert!(
            repo.claim_container_recovery(published, &recovered_command(published))
                .await
                .is_err()
        );
        // A third requested revision also inherits the root, not this newly effective child.
        let third = next_claim(&repo, &db, &a, &current).await;
        repo.authorize_recovered_activation(&current, &proof)
            .await
            .unwrap();
        let planned = repo
            .claim_recovered_container_activation(&third, &proof)
            .await
            .unwrap();
        assert_eq!(
            planned
                .claim
                .anchor()
                .prepared
                .container
                .registration
                .generation,
            proof.lease.request.launch_id
        );
        assert_eq!(
            planned.claim.lineage.as_ref().unwrap().family_id,
            parent.claim.id
        );
        db.close().await.unwrap();
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_ACTIVATION_TEST_DATABASE_URL"]
async fn recovered_committed_child_next_activation_and_failure_rollback_keep_original_anchor() {
    sequential_publication(false).await;
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_ACTIVATION_TEST_DATABASE_URL"]
async fn recovered_rolled_back_child_next_activation_and_failure_rollback_keep_original_anchor() {
    sequential_publication(true).await;
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_ACTIVATION_TEST_DATABASE_URL"]
async fn recovered_next_claim_rejects_foreign_lineage_unknown_lease_and_duplicate_plan() {
    let (repo, db, a, parent, proof) = recovered_publication(false).await;
    let claim = next_claim(&repo, &db, &a, &parent).await;
    let revision = repo
        .list_config_revisions(a.id)
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.revision == claim.revision)
        .unwrap();
    let mut preplan = RecoveryHold::new(proof.lease.request.controller_id, &claim.previous, None);
    preplan.bind_custody(parent.claim.anchor());
    repo.hold_container_activation(&revision, None, &claim.previous, &preplan)
        .await
        .unwrap();
    assert!(
        repo.get_container_activation(a.id, claim.revision)
            .await
            .unwrap()
            .is_none()
    );
    let actor:Uuid=db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT created_by_user_id FROM agent_config_revisions WHERE agent_id=$1 AND revision=1",[a.id.into()]))
        .await.unwrap().unwrap().try_get("","created_by_user_id").unwrap();
    let session = Uuid::new_v4();
    let run = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO agent_sessions(id,agent_id,user_id,title,state) VALUES($1,$2,$3,'next drain','active')",
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
            repo.claim_recovered_container_activation(&claim, &proof)
                .await
                .is_err(),
            "{state}"
        );
    }
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE session_agent_runs SET state='completed' WHERE id=$1",
        [run.into()],
    ))
    .await
    .unwrap();
    for key in [
        "anchor",
        "family",
        "predecessor",
        "hash",
        "snapshot",
        "stop",
        "configuration",
    ] {
        let mut c = claim.clone();
        match key {
            "anchor" => c.lineage.as_mut().unwrap().anchor.controller_id = Uuid::new_v4(),
            "family" => c.lineage.as_mut().unwrap().family_id = Uuid::new_v4(),
            "predecessor" => c.lineage.as_mut().unwrap().predecessor_activation_id = Uuid::new_v4(),
            "hash" => c.lineage.as_mut().unwrap().predecessor_intent_sha256 = "f".repeat(64),
            "snapshot" => c.previous.snapshot.as_mut().unwrap()["init_pid"] = json!(987),
            "stop" => c.previous.stop_id = Uuid::new_v4(),
            _ => c.previous_revision = Some(999),
        }
        assert!(
            repo.claim_recovered_container_activation(&c, &proof)
                .await
                .is_err(),
            "{key}"
        );
    }
    for key in ["owner", "epoch", "version", "observation", "root_running"] {
        let mut p = proof.clone();
        match key {
            "owner" => p.lease.request.controller_id = Uuid::new_v4(),
            "epoch" => p.lease.epoch += 1,
            "version" => p.lease.lease_version += 1,
            "root_running" => p.observation["observation"] = json!("running"),
            _ => p.observation["snapshot"]["init_pid"] = json!(999),
        }
        assert!(
            repo.claim_recovered_container_activation(&claim, &p)
                .await
                .is_err(),
            "{key}"
        );
    }
    let prior = repo
        .get_container_recovery(proof.lease.request.launch_id)
        .await
        .unwrap()
        .unwrap();
    let mut lease = proof.lease.clone();
    lease.lease_version += 1;
    lease.lease_expires_at = (chrono::Utc::now() + chrono::Duration::seconds(30))
        .to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    repo.claim_container_lease(&prior, &lease).await.unwrap();
    assert!(
        repo.claim_recovered_container_activation(&claim, &proof)
            .await
            .is_err()
    );
    let mut fresh = proof.clone();
    fresh.lease = lease.clone();
    assert!(
        repo.claim_recovered_container_activation(&claim, &fresh)
            .await
            .is_err()
    );
    assert!(
        repo.get_container_activation(a.id, claim.revision)
            .await
            .unwrap()
            .is_none()
    );
    repo.acknowledge_container_lease(&lease,json!({"ack":{"state":"controller_heartbeat","recovery_id":lease.request.id,
        "lease_version":lease.lease_version,"lease_expires_at":lease.lease_expires_at},"receipt":proof.observation})).await.unwrap();
    repo.authorize_recovered_activation(&parent, &fresh)
        .await
        .unwrap();
    let planned = repo
        .claim_recovered_container_activation(&claim, &fresh)
        .await
        .unwrap();
    let revision = repo
        .list_config_revisions(a.id)
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.revision == claim.revision)
        .unwrap();
    let hold = RecoveryHold::new(
        fresh.lease.request.controller_id,
        &claim.previous,
        Some(&planned),
    );
    assert_eq!(hold.custody_generation, fresh.lease.request.launch_id);
    assert_eq!(
        hold.generation,
        claim.previous.prepared.container.registration.generation
    );
    repo.hold_container_activation(&revision, Some(&planned), &claim.previous, &hold)
        .await
        .unwrap();
    let (first, second) = tokio::join!(
        repo.claim_recovered_container_activation(&claim, &fresh),
        repo.claim_recovered_container_activation(&claim, &fresh)
    );
    assert_eq!(json!(first.unwrap()), json!(planned));
    assert_eq!(json!(second.unwrap()), json!(planned));
    let mut raw = planned.clone();
    raw.phase = Phase::StoppingPrevious;
    assert!(
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE runtime_container_activations SET record=$2 WHERE id=$1",
            [planned.claim.id.into(), json!(raw).into()]
        ))
        .await
        .is_err()
    );
    assert_eq!(
        json!(
            repo.claim_recovered_container_activation(&claim, &fresh)
                .await
                .unwrap()
        ),
        json!(planned)
    );
    let mut conflict = claim.clone();
    conflict.id = Uuid::new_v4();
    conflict.candidate = generation();
    conflict.intent_sha256 = "e".repeat(64);
    assert!(
        repo.claim_recovered_container_activation(&conflict, &fresh)
            .await
            .is_err()
    );
    assert_eq!(
        repo.get_container_activation(a.id, claim.revision)
            .await
            .unwrap()
            .unwrap()
            .claim
            .id,
        planned.claim.id
    );
    db.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_CONTAINER_ACTIVATION_TEST_DATABASE_URL"]
async fn recovered_next_claim_expiry_and_new_epoch_require_latest_original_authority() {
    let (repo, db, a, parent, proof) = recovered_publication(false).await;
    let claim = next_claim(&repo, &db, &a, &parent).await;
    let deadline = chrono::DateTime::parse_from_rfc3339(&proof.lease.lease_expires_at).unwrap();
    tokio::time::sleep(
        deadline
            .signed_duration_since(chrono::Utc::now())
            .to_std()
            .unwrap_or_default()
            + std::time::Duration::from_millis(10),
    )
    .await;
    assert!(
        repo.claim_recovered_container_activation(&claim, &proof)
            .await
            .is_err()
    );
    assert!(
        repo.get_container_activation(a.id, claim.revision)
            .await
            .unwrap()
            .is_none()
    );
    let mut next = proof.lease.clone();
    next.request.id = Uuid::new_v4();
    next.request.controller_id = Uuid::new_v4();
    next.request.predecessor_id = Some(proof.lease.request.id);
    next.request.controller_snapshot.started_at = "2026-10-09T13:00:00Z".into();
    next.request.controller_snapshot.init_pid += 1;
    next.epoch += 1;
    next.lease_version = 1;
    next.lease_expires_at = (chrono::Utc::now() + chrono::Duration::seconds(30))
        .to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    repo.claim_container_recovery(parent.claim.anchor(), &next)
        .await
        .unwrap();
    assert!(
        repo.claim_recovered_container_activation(&claim, &proof)
            .await
            .is_err()
    );
    let mut exited = parent.claim.anchor().clone();
    exited.state = "exited".into();
    let fresh = recovered_ack(&repo, &exited, &next).await;
    // Native ACK alone cannot silently transfer the predecessor's activation authority.
    assert!(
        repo.claim_recovered_container_activation(&claim, &fresh)
            .await
            .is_err()
    );
    assert!(
        repo.get_container_activation(a.id, claim.revision)
            .await
            .unwrap()
            .is_none()
    );
    repo.authorize_recovered_activation(&parent, &fresh)
        .await
        .unwrap();
    let planned = repo
        .claim_recovered_container_activation(&claim, &fresh)
        .await
        .unwrap();
    let mut stopping = planned.clone();
    stopping.phase = Phase::StoppingPrevious;
    assert!(
        repo.advance_recovered_activation(&planned, &stopping, &proof)
            .await
            .is_err()
    );
    repo.advance_recovered_activation(&planned, &stopping, &fresh)
        .await
        .unwrap();
    assert_eq!(
        planned
            .claim
            .anchor()
            .prepared
            .container
            .registration
            .generation,
        proof.lease.request.launch_id
    );
    assert!(
        repo.get_container_recovery(claim.previous.prepared.container.registration.generation)
            .await
            .unwrap()
            .is_none()
    );
    db.close().await.unwrap();
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
        // A live recovery fence does not waive the newly requested activation's drain.
        let held = repo
            .advance_recovered_container(&published, &proof.lease, "stopping")
            .await;
        assert!(matches!(held, Err(shared::AppError::Conflict(message))
            if message == "Runtime capacity or unknown acceptance remains held"));
        assert_eq!(
            json!(repo.get_container_launch(a.id).await.unwrap().unwrap()),
            json!(published)
        );
        assert!(origin_live(&db, &published).await);
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

        // Independently exercise regular stop with a draft, but no pending activation.
        let (repo, db, a, record, proof) = recovered_publication(rollback).await;
        let published = if rollback {
            record.rollback.clone()
        } else {
            record.candidate.clone()
        }
        .unwrap();
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
        assert!(!repo.agent_is_draining(a.id).await.unwrap());
        assert!(origin_live(&db, &published).await);
        repo.authorize_recovered_activation(&record, &proof)
            .await
            .unwrap();
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
        assert!(!head.try_get::<bool>("", "draining").unwrap());
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
    let writer = PostgresFleetRepository::new(test_database().await);
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
    let wait = deadline
        .signed_duration_since(chrono::Utc::now())
        .to_std()
        .unwrap_or_default()
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
    let reader = test_database().await;
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

async fn test_database() -> DatabaseConnection {
    let url = std::env::var("FLEET_CONTAINER_ACTIVATION_TEST_DATABASE_URL")
        .expect("own activation database required");
    assert_eq!(
        reqwest::Url::parse(&url).unwrap().path(),
        "/fleet_container_activation_test"
    );
    Database::connect(url).await.unwrap()
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
    let db = test_database().await;
    migration::Migrator::up(&db, None).await.unwrap();
    let repo = PostgresFleetRepository::new(test_database().await);
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
        lineage: None,
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
