//! Transactions couple activation progress, launch custody and effective publication.
use super::*;
use app::container_activation::RecoveredProof;
use app::container_activation::{Activation, Claim, Phase, RecoveryHold, held, stopped};
use app::container_runtime::ContainerLaunch;
use sea_orm::DatabaseTransaction;

pub(super) async fn open(
    repo: &PostgresFleetRepository,
    agent: Uuid,
) -> Result<Option<Activation>, AppError> {
    repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT a.record FROM runtime_container_activations a JOIN agent_config_heads h USING(agent_id)
         WHERE a.agent_id=$1 AND ((a.revision=h.desired_revision AND h.draining)
         OR (a.record->>'phase' IN ('committed','rolled_back')
           AND EXISTS(SELECT 1 FROM runtime_container_activation_authorities x WHERE x.activation_id=a.id)
           AND EXISTS(SELECT 1 FROM runtime_container_launches c WHERE c.agent_id=a.agent_id AND c.generation::text=a.record#>>'{readiness,generation}'
             AND NOT EXISTS(SELECT 1 FROM runtime_container_launches other WHERE other.agent_id=c.agent_id AND other.state<>'exited' AND other.generation<>c.generation))))
         ORDER BY (a.revision=h.desired_revision AND h.draining) DESC,a.revision DESC LIMIT 1", [agent.into()]))
        .await.map_err(|_| held())?.map(|r| serde_json::from_value(r.try_get("","record").map_err(|_| held())?).map_err(|_| held())).transpose()
}

pub(super) async fn authorize(
    repo: &PostgresFleetRepository,
    record: &Activation,
    proof: &RecoveredProof,
) -> Result<(), AppError> {
    let tx = repo
        .db
        .begin()
        .await
        .map_err(|_| authorize_failure("activation_authorize_begin"))?;
    if record.phase.terminal() {
        container_runtime::lock(&tx, record.claim.agent_id)
            .await
            .map_err(|_| authorize_failure("activation_authorize_lock"))?;
        let observation: runtime::container_control::ContainerReceipt =
            serde_json::from_value(proof.observation.clone())
                .map_err(|_| authorize_failure("activation_authorize_receipt_decode"))?;
        runtime::container_control::validate_receipt(
            &observation,
            &record.claim.anchor().prepared.container.registration,
            0,
            "observe",
        )
        .map_err(|_| authorize_failure("activation_authorize_receipt_validation"))?;
        if encode(&observation.snapshot)
            .map_err(|_| authorize_failure("activation_authorize_receipt_validation"))?
            != record
                .claim
                .anchor()
                .snapshot
                .clone()
                .ok_or_else(|| authorize_failure("activation_authorize_receipt_validation"))?
            || (record.claim.lineage.is_some()
                && observation.observation
                    != runtime::container_control::ContainerObservation::NamespaceExited)
        {
            return Err(authorize_failure("activation_authorize_receipt_validation"));
        }
        let row=tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT a.id FROM runtime_container_activations a JOIN runtime_container_recoveries r
             ON r.generation=fleet_activation_scope(a.record)
             WHERE a.id=$1 AND a.record=$2 AND r.lease=$3 AND fleet_activation_anchor(r.generation)
             AND r.receipt IS NOT NULL AND r.lease_receipt IS NOT NULL AND r.expires_at>clock_timestamp()
             AND NOT EXISTS(SELECT 1 FROM runtime_container_recoveries n WHERE n.generation=r.generation AND n.epoch>r.epoch)
             FOR UPDATE OF r,a",[record.claim.id.into(),encode(record).map_err(|_| authorize_failure("activation_authorize_current"))?.into(),encode(&proof.lease).map_err(|_| authorize_failure("activation_authorize_current"))?.into()]))
             .await.map_err(|_| authorize_failure("activation_authorize_current"))?;
        if row.is_none() {
            return Err(authorize_failure("activation_authorize_current"));
        }
    } else {
        checked(&tx, &record.claim, Some((record, proof))).await?;
    }
    tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_container_activation_authorities(activation_id,recovery_id,controller_id,plan_sha256,claim)
         VALUES($1,$2,$3,$4,$5) ON CONFLICT(activation_id,recovery_id) DO NOTHING",
        [record.claim.id.into(),proof.lease.request.id.into(),proof.lease.request.controller_id.into(),
        record.claim.intent_sha256.clone().into(),encode(&record.claim).map_err(|_| authorize_failure("activation_authorize_authority_insert"))?.into()])).await.map_err(|_| authorize_failure("activation_authorize_authority_insert"))?;
    tx.commit()
        .await
        .map_err(|_| authorize_failure("activation_authorize_commit"))
}

fn authorize_failure(stage: &'static str) -> AppError {
    AppError::Unavailable(stage.into())
}

pub(super) async fn for_launch(
    repo: &PostgresFleetRepository,
    launch: &ContainerLaunch,
) -> Result<Option<Activation>, AppError> {
    let row=repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT a.record FROM runtime_container_activations a WHERE a.agent_id=$1
         AND (a.record#>>'{claim,candidate,generation}'=$2::text OR a.record#>>'{claim,rollback,generation}'=$2::text)
         AND EXISTS(SELECT 1 FROM runtime_container_activation_authorities x WHERE x.activation_id=a.id)",
        [launch.prepared.agent_id.into(),launch.prepared.container.registration.generation.into()])).await.map_err(|_| held())?;
    let record: Option<Activation> = row
        .map(|r| {
            serde_json::from_value(r.try_get("", "record").map_err(|_| held())?).map_err(|_| held())
        })
        .transpose()?;
    if record.as_ref().is_some_and(|a| !a.tracks_launch(launch)) {
        return Err(held());
    }
    Ok(record)
}

fn encode<T: serde::Serialize>(value: &T) -> Result<Value, AppError> {
    serde_json::to_value(value).map_err(|_| held())
}

pub(super) async fn get(
    repo: &PostgresFleetRepository,
    agent: Uuid,
    revision: i64,
) -> Result<Option<Activation>, AppError> {
    repo.db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT record FROM runtime_container_activations WHERE agent_id=$1 AND revision=$2",
            [agent.into(), revision.into()],
        ))
        .await
        .map_err(|_| held())?
        .map(|r| {
            serde_json::from_value(r.try_get("", "record").map_err(|_| held())?).map_err(|_| held())
        })
        .transpose()
}

pub(super) async fn pending(
    repo: &PostgresFleetRepository,
    after_agent: Option<Uuid>,
) -> Result<Vec<domain::AgentConfigRevision>, AppError> {
    let rows = repo.db.query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT r.agent_id,r.revision FROM agent_config_revisions r JOIN agent_config_heads h USING(agent_id)
         LEFT JOIN runtime_container_activations a ON a.agent_id=r.agent_id AND a.revision=r.revision
         WHERE r.revision=h.desired_revision AND h.draining AND r.state='activating' AND r.claimed_at IS NOT NULL
            AND ($1::uuid IS NULL OR r.agent_id>$1)
            AND (a.record IS NULL OR a.record->>'phase' NOT IN ('committed','rolled_back'))
            AND EXISTS(SELECT 1 FROM runtime_container_launches l WHERE l.agent_id=r.agent_id)
         ORDER BY r.agent_id LIMIT 16",
        [after_agent.into()])).await.map_err(|_| held())?;
    let mut revisions = Vec::new();
    for row in rows {
        revisions.push(
            config_revisions::get(
                repo,
                row.try_get("", "agent_id").map_err(|_| held())?,
                row.try_get("", "revision").map_err(|_| held())?,
            )
            .await?,
        );
    }
    Ok(revisions)
}

/// Publish a typed diagnostic and recovery action atomically, without changing any custody.
pub(super) async fn hold(
    repo: &PostgresFleetRepository,
    revision: &domain::AgentConfigRevision,
    activation: Option<&Activation>,
    launch: &ContainerLaunch,
    recovery: &RecoveryHold,
) -> Result<(), AppError> {
    let predecessor = if activation.is_none() {
        for_launch(repo, launch).await?
    } else {
        None
    };
    let custody = activation
        .and_then(|a| a.claim.lineage.as_ref().map(|l| &l.anchor))
        .or_else(|| predecessor.as_ref().map(|p| p.claim.anchor()));
    let mut expected = RecoveryHold::new(recovery.controller_id, launch, activation);
    if let Some(anchor) = custody {
        expected.bind_custody(anchor);
    }
    if recovery.controller_id.is_nil()
        || recovery.original_controller_id != expected.original_controller_id
        || recovery.custody_generation != expected.custody_generation
        || recovery.generation != expected.generation
        || recovery.operation_id != expected.operation_id
        || recovery.stop_id != expected.stop_id
        || recovery.activation_id != expected.activation_id
        || recovery.phase != expected.phase
        || recovery.intent_sha256 != expected.intent_sha256
        || launch.prepared.agent_id != revision.agent_id
        || activation.is_some_and(|a| !a.tracks_launch(launch))
    {
        return Err(held());
    }
    let tx = repo.db.begin().await.map_err(|_| held())?;
    container_runtime::lock(&tx, revision.agent_id).await?;
    let row = tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT r.snapshot,r.last_error,a.record FROM agent_config_revisions r JOIN agent_config_heads h USING(agent_id)
         LEFT JOIN runtime_container_activations a ON a.agent_id=r.agent_id AND a.revision=r.revision
         WHERE r.agent_id=$1 AND r.revision=$2 AND h.desired_revision=r.revision AND h.draining
            AND r.state='activating' AND r.claimed_at IS NOT NULL FOR UPDATE OF r,h",
        [revision.agent_id.into(),revision.revision.into()])).await.map_err(|_| held())?.ok_or_else(held)?;
    if row.try_get::<Value>("", "snapshot").map_err(|_| held())? != encode(&revision.snapshot)?
        || row
            .try_get::<Option<Value>>("", "record")
            .map_err(|_| held())?
            != activation.map(encode).transpose()?
    {
        return Err(held());
    }
    let original = tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT generation FROM runtime_container_launches WHERE agent_id=$1 AND generation=$2 AND controller_id=$3
            AND prepared=$4 AND state=$5 AND snapshot IS NOT DISTINCT FROM $6
            AND origin IS NOT DISTINCT FROM $7 AND stop_id=$8 FOR UPDATE",
        [revision.agent_id.into(),launch.prepared.container.registration.generation.into(),launch.controller_id.into(),encode(&launch.prepared)?.into(),
            launch.state.clone().into(),launch.snapshot.clone().into(),launch.origin.clone().into(),launch.stop_id.into()]))
        .await.map_err(|_| held())?;
    if original.is_none() {
        return Err(held());
    }
    if let Some(anchor) = custody {
        let root=tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT generation FROM runtime_container_launches WHERE generation=$1 AND agent_id=$2 AND controller_id=$3
             AND prepared=$4 AND snapshot=$5 AND origin=$6 AND stop_id=$7 FOR UPDATE",
            [recovery.custody_generation.into(),revision.agent_id.into(),anchor.controller_id.into(),encode(&anchor.prepared)?.into(),
                anchor.snapshot.clone().into(),anchor.origin.clone().into(),anchor.stop_id.into()])).await.map_err(|_| held())?;
        if root.is_none() {
            return Err(held());
        }
    }
    let payload = encode(recovery)?;
    let error = serde_json::to_string(&payload).map_err(|_| held())?;
    if row
        .try_get::<Option<String>>("", "last_error")
        .map_err(|_| held())?
        .as_ref()
        != Some(&error)
    {
        tx.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agent_config_revisions SET last_error=$3 WHERE agent_id=$1 AND revision=$2",
            [
                revision.agent_id.into(),
                revision.revision.into(),
                error.into(),
            ],
        ))
        .await
        .map_err(|_| held())?;
        tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO audit_log(id,actor_user_id,action,entity_type,entity_id,payload,created_at)
             VALUES($1,NULL,'agent_config.recovery_required','agent_config',$2,$3,now())",
            [Uuid::new_v4().into(),revision.agent_id.to_string().into(),
                serde_json::json!({"revision":revision.revision,"hold":payload}).into()])).await.map_err(|_| held())?;
    }
    tx.commit().await.map_err(|_| held())
}

pub(super) async fn generation_intent_hash(
    repo: &PostgresFleetRepository,
    agent: Uuid,
    generation: Uuid,
) -> Result<Option<String>, AppError> {
    repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT claim->>'intent_sha256' AS hash FROM runtime_container_preparations WHERE agent_id=$1 AND generation=$2 AND receipt IS NOT NULL
         UNION ALL SELECT CASE WHEN record#>>'{claim,candidate,generation}'=$2::text
            THEN record#>>'{claim,candidate_intent_sha256}' ELSE record#>>'{claim,rollback_intent_sha256}' END AS hash
         FROM runtime_container_activations WHERE agent_id=$1 AND
            ((record->>'phase'='committed' AND record#>>'{claim,candidate,generation}'=$2::text)
             OR (record->>'phase'='rolled_back' AND record#>>'{claim,rollback,generation}'=$2::text))",
        [agent.into(),generation.into()])).await.map_err(|_| held())?
        .map(|r| r.try_get("", "hash").map_err(|_| held())).transpose()
}

async fn checked(
    tx: &DatabaseTransaction,
    claim: &Claim,
    recovered: Option<(&Activation, &RecoveredProof)>,
) -> Result<(), AppError> {
    // Preserve ordinary activation errors; recovered failures expose fixed stages only.
    let fail = |stage| {
        if recovered.is_some() {
            authorize_failure(stage)
        } else {
            held()
        }
    };
    let agent = container_runtime::lock(tx, claim.agent_id)
        .await
        .map_err(|error| {
            recovered.map_or(error, |_| authorize_failure("activation_authorize_lock"))
        })?;
    if agent
        .try_get::<String>("", "kind")
        .map_err(|_| fail("activation_authorize_agent"))?
        != "hermes"
        || agent
            .try_get::<Option<shared::Timestamp>>("", "archived_at")
            .map_err(|_| fail("activation_authorize_agent"))?
            .is_some()
    {
        return Err(fail("activation_authorize_agent"));
    }
    let row = tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
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
        [claim.agent_id.into(),claim.revision.into(),recovered.map(|_| claim.anchor().prepared.container.registration.generation).into()])).await.map_err(|_| fail("activation_authorize_readback"))?.ok_or_else(|| fail("activation_authorize_readback"))?;
    if let Some((record, proof)) = recovered {
        let b = &claim.anchor().prepared.container;
        let receipt: runtime::container_control::ContainerReceipt =
            serde_json::from_value(proof.observation.clone())
                .map_err(|_| fail("activation_authorize_receipt_decode"))?;
        runtime::container_control::validate_receipt(&receipt, &b.registration, 0, "observe")
            .map_err(|_| fail("activation_authorize_receipt_validation"))?;
        if encode(&record.claim).map_err(|_| fail("activation_authorize_receipt_validation"))?
            != encode(claim).map_err(|_| fail("activation_authorize_receipt_validation"))?
            || (claim.lineage.is_some()
                && receipt.observation
                    != runtime::container_control::ContainerObservation::NamespaceExited)
            || proof.lease.request.launch_id != b.registration.generation
            || proof.lease.request.agent_id != claim.agent_id
            || proof.lease.request.original_controller_id != claim.controller_id
            || proof.lease.request.controller_id == claim.controller_id
            || proof.lease.request.launch_sha256
                != runtime::container_control::launch_hash(claim.anchor())
                    .map_err(|_| fail("activation_authorize_receipt_validation"))?
            || encode(&receipt.snapshot)
                .map_err(|_| fail("activation_authorize_receipt_validation"))?
                != claim
                    .anchor()
                    .snapshot
                    .clone()
                    .ok_or_else(|| fail("activation_authorize_receipt_validation"))?
        {
            return Err(fail("activation_authorize_receipt_validation"));
        }
        let current=tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT r.id FROM runtime_container_recoveries r
             WHERE r.generation=$3 AND r.lease=$4 AND
               (EXISTS(SELECT 1 FROM runtime_container_activations a WHERE a.id=$1 AND a.record=$2)
               OR ($5 AND NOT EXISTS(SELECT 1 FROM runtime_container_activations a WHERE a.agent_id=$6 AND a.revision=$7)
                   AND fleet_activation_lineage($8)))
             AND r.receipt IS NOT NULL AND r.lease_receipt IS NOT NULL AND r.expires_at>clock_timestamp()
             AND NOT EXISTS(SELECT 1 FROM runtime_container_recoveries n WHERE n.generation=r.generation AND n.epoch>r.epoch) FOR UPDATE OF r",
            [claim.id.into(),encode(record).map_err(|_| fail("activation_authorize_current"))?.into(),b.registration.generation.into(),encode(&proof.lease).map_err(|_| fail("activation_authorize_current"))?.into(),
                (record.phase==Phase::Planned && claim.lineage.is_some()).into(),claim.agent_id.into(),claim.revision.into(),encode(claim).map_err(|_| fail("activation_authorize_current"))?.into()]))
            .await.map_err(|_| fail("activation_authorize_current"))?;
        if current.is_none() {
            return Err(fail("activation_authorize_current"));
        }
    }
    if row
        .try_get::<Option<i64>>("", "effective_revision")
        .map_err(|_| fail("activation_authorize_config"))?
        != claim.previous_revision
        || runtime::container_control::canonical_hash(
            &row.try_get::<Value>("", "snapshot")
                .map_err(|_| fail("activation_authorize_config"))?,
        )
        .map_err(|error| {
            recovered.map_or(error, |_| authorize_failure("activation_authorize_config"))
        })? != claim.configuration_sha256
    {
        return Err(fail("activation_authorize_config"));
    }
    for (column, expected) in [
        ("runtime_path", &claim.previous.prepared.paths.runtime),
        ("config_path", &claim.previous.prepared.paths.config),
        ("workspace_path", &claim.previous.prepared.paths.workspace),
        ("logs_path", &claim.previous.prepared.paths.logs),
    ] {
        if agent
            .try_get::<String>("", column)
            .map_err(|_| fail("activation_authorize_agent"))?
            != *expected
        {
            return Err(fail("activation_authorize_agent"));
        }
    }
    if agent
        .try_get::<Option<i32>>("", "api_port")
        .map_err(|_| fail("activation_authorize_agent"))?
        != claim.previous.prepared.api_port
    {
        return Err(fail("activation_authorize_agent"));
    }
    Ok(())
}

pub(super) async fn claim(
    repo: &PostgresFleetRepository,
    claim: &Claim,
) -> Result<Activation, AppError> {
    claim_proved(repo, claim, None).await
}

pub(super) async fn claim_proved(
    repo: &PostgresFleetRepository,
    claim: &Claim,
    proof: Option<&RecoveredProof>,
) -> Result<Activation, AppError> {
    if claim.lineage.is_some() != proof.is_some() {
        return Err(held());
    }
    let planned = Activation::planned(claim.clone());
    let tx = repo.db.begin().await.map_err(|_| held())?;
    checked(&tx, claim, proof.map(|p| (&planned, p))).await?;
    if let Some(proof) = proof {
        tx.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT set_config('fleet.container_recovery_id',$1,true)",
            [proof.lease.request.id.to_string().into()],
        ))
        .await
        .map_err(|_| held())?;
    }
    if claim.id.is_nil()
        || claim.controller_id.is_nil()
        || claim.previous.controller_id != claim.controller_id
        || claim.previous.prepared.agent_id != claim.agent_id
        || claim.previous.state != "running"
        || claim.previous.snapshot.is_none()
        || claim.previous.origin.is_none()
        || claim.previous.prepared.configuration_revision != claim.previous_revision
        || claim.previous.prepared.configuration_sha256 != claim.previous_configuration_sha256
        || claim.candidate.generation == claim.rollback.generation
        || claim.candidate.generation == claim.previous.prepared.container.registration.generation
        || claim.rollback.generation == claim.previous.prepared.container.registration.generation
        || [
            claim.candidate.generation,
            claim.candidate.operation_id,
            claim.candidate.stop_id,
            claim.rollback.generation,
            claim.rollback.operation_id,
            claim.rollback.stop_id,
        ]
        .iter()
        .any(Uuid::is_nil)
        || claim.candidate.operation_id == claim.rollback.operation_id
        || claim.candidate.stop_id == claim.rollback.stop_id
    {
        return Err(held());
    }
    let exists = tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT generation FROM runtime_container_launches WHERE generation=$1 AND agent_id=$2 AND controller_id=$3
            AND prepared=$4 AND state='running' AND snapshot=$5 AND origin=$6 AND stop_id=$7 FOR UPDATE",
        [claim.previous.prepared.container.registration.generation.into(),claim.agent_id.into(),claim.controller_id.into(),
            encode(&claim.previous.prepared)?.into(),claim.previous.snapshot.clone().into(),claim.previous.origin.clone().into(),
            claim.previous.stop_id.into()])).await.map_err(|_| held())?;
    if exists.is_none() {
        return Err(held());
    }
    tx.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO runtime_container_activations(id,agent_id,revision,record) VALUES($1,$2,$3,$4)
            ON CONFLICT(agent_id,revision) DO NOTHING",
        [
            claim.id.into(),
            claim.agent_id.into(),
            claim.revision.into(),
            encode(&planned)?.into(),
        ],
    ))
    .await
    .map_err(|_| held())?;
    let row = tx
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT record FROM runtime_container_activations WHERE agent_id=$1 AND revision=$2",
            [claim.agent_id.into(), claim.revision.into()],
        ))
        .await
        .map_err(|_| held())?
        .ok_or_else(held)?;
    let saved: Activation = serde_json::from_value(row.try_get("", "record").map_err(|_| held())?)
        .map_err(|_| held())?;
    if encode(&saved.claim)? != encode(claim)? {
        return Err(held());
    }
    if let Some(proof) = proof {
        tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO runtime_container_activation_authorities(activation_id,recovery_id,controller_id,plan_sha256,claim)
             VALUES($1,$2,$3,$4,$5) ON CONFLICT(activation_id,recovery_id) DO NOTHING",
            [claim.id.into(),proof.lease.request.id.into(),proof.lease.request.controller_id.into(),
                claim.intent_sha256.clone().into(),encode(claim)?.into()])).await.map_err(|_| held())?;
    }
    tx.commit().await.map_err(|_| held())?;
    Ok(saved)
}

async fn update_launch(
    tx: &DatabaseTransaction,
    old: &ContainerLaunch,
    next: &ContainerLaunch,
) -> Result<(), AppError> {
    let changed = tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_container_launches SET state=$1,snapshot=$2,origin=$3 WHERE generation=$4
            AND controller_id=$5 AND state=$6 AND prepared=$7 AND snapshot IS NOT DISTINCT FROM $8
            AND origin IS NOT DISTINCT FROM $9 AND stop_id=$10",
        [next.state.clone().into(),next.snapshot.clone().into(),next.origin.clone().into(),
            old.prepared.container.registration.generation.into(),old.controller_id.into(),old.state.clone().into(),
            encode(&old.prepared)?.into(),old.snapshot.clone().into(),old.origin.clone().into(),old.stop_id.into()])).await.map_err(|_| held())?;
    if changed.rows_affected() != 1 {
        return Err(held());
    }
    Ok(())
}

fn stop_proof(launch: &ContainerLaunch, proof: &Option<Value>) -> Result<(), AppError> {
    let hash =
        runtime::container_control::canonical_hash(launch.snapshot.as_ref().ok_or_else(held)?)?;
    if !stopped(launch, proof.as_ref().ok_or_else(held)?, &hash) {
        return Err(held());
    }
    Ok(())
}

pub(super) async fn advance(
    repo: &PostgresFleetRepository,
    old: &Activation,
    next: &Activation,
) -> Result<(), AppError> {
    advance_proved(repo, old, next, None).await
}

pub(super) async fn advance_proved(
    repo: &PostgresFleetRepository,
    old: &Activation,
    next: &Activation,
    proof: Option<&RecoveredProof>,
) -> Result<(), AppError> {
    old.validate_next(next)?;
    if next.previous_stop.is_some() {
        stop_proof(&old.claim.previous, &next.previous_stop)?;
    }
    if next.candidate_stop.is_some() {
        stop_proof(
            next.candidate.as_ref().ok_or_else(held)?,
            &next.candidate_stop,
        )?;
    }
    let tx = repo.db.begin().await.map_err(|_| held())?;
    checked(&tx, &old.claim, proof.map(|p| (old, p))).await?;
    if let Some(proof) = proof {
        let row=tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT activation_id FROM runtime_container_activation_authorities WHERE activation_id=$1 AND recovery_id=$2
             AND controller_id=$3 AND plan_sha256=$4 AND claim=$5",
            [old.claim.id.into(),proof.lease.request.id.into(),proof.lease.request.controller_id.into(),old.claim.intent_sha256.clone().into(),encode(&old.claim)?.into()]))
            .await.map_err(|_| held())?;
        if row.is_none() {
            return Err(held());
        }
        tx.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT set_config('fleet.container_recovery_id',$1,true)",
            [proof.lease.request.id.to_string().into()],
        ))
        .await
        .map_err(|_| held())?;
    }
    tx.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT set_config('fleet.container_activation_id',$1,true)",
        [old.claim.id.to_string().into()],
    ))
    .await
    .map_err(|_| held())?;
    let changed = tx
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE runtime_container_activations SET record=$1 WHERE id=$2 AND record=$3",
            [
                encode(next)?.into(),
                old.claim.id.into(),
                encode(old)?.into(),
            ],
        ))
        .await
        .map_err(|_| held())?;
    if changed.rows_affected() != 1 {
        return Err(held());
    }
    use Phase::*;
    match next.phase {
        StoppingPrevious | PreviousStopped => {
            let mut before = old.claim.previous.clone();
            if next.phase == PreviousStopped {
                before.state = "stopping".into();
            }
            let mut after = before.clone();
            after.state = if next.phase == StoppingPrevious {
                "stopping"
            } else {
                "exited"
            }
            .into();
            update_launch(&tx, &before, &after).await?;
        }
        CandidatePrepared | RollbackPrepared => {
            let l = if next.phase == CandidatePrepared {
                &next.candidate
            } else {
                &next.rollback
            }
            .as_ref()
            .ok_or_else(held)?;
            tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "INSERT INTO runtime_container_launches(generation,agent_id,controller_id,prepared,state,stop_id)
                    VALUES($1,$2,$3,$4,'claimed',$5)", [l.prepared.container.registration.generation.into(),old.claim.agent_id.into(),
                    l.controller_id.into(),encode(&l.prepared)?.into(),l.stop_id.into()])).await.map_err(|_| held())?;
        }
        CandidateRunning | StoppingCandidate | CandidateStopped | RollbackRunning => {
            let (before, after) = if next.phase == RollbackRunning {
                (&old.rollback, &next.rollback)
            } else {
                (&old.candidate, &next.candidate)
            };
            update_launch(
                &tx,
                before.as_ref().ok_or_else(held)?,
                after.as_ref().ok_or_else(held)?,
            )
            .await?;
        }
        Committed | RolledBack => {
            let l = if next.phase == Committed {
                &next.candidate
            } else {
                &next.rollback
            }
            .as_ref()
            .ok_or_else(held)?;
            let error: Option<String> = if next.phase == RolledBack {
                Some("Candidate readiness failed; exact previous configuration restored on a fresh original generation".into())
            } else {
                None
            };
            tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "UPDATE agent_config_revisions SET state=$3,last_error=$4 WHERE agent_id=$1 AND revision=$2 AND state='activating'",
                [old.claim.agent_id.into(),old.claim.revision.into(),if error.is_some() {"failed"} else {"active"}.into(),error.into()])).await.map_err(|_| held())?;
            let changed=tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "UPDATE agent_config_heads SET effective_revision=$3,draining=false WHERE agent_id=$1 AND desired_revision=$2 AND draining",
                [old.claim.agent_id.into(),old.claim.revision.into(),l.prepared.configuration_revision.into()])).await.map_err(|_| held())?;
            if changed.rows_affected() != 1 {
                return Err(held());
            }
            tx.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE agents SET status='running',updated_at=now() WHERE id=$1",
                [old.claim.agent_id.into()],
            ))
            .await
            .map_err(|_| held())?;
            let pid = l
                .snapshot
                .as_ref()
                .and_then(|s| s["init_pid"].as_i64())
                .and_then(|p| i32::try_from(p).ok())
                .ok_or_else(held)?;
            tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "UPDATE agent_runtime SET pid=$2,health_status='running',health_detail='Original generation passed activation readiness; admission remains separate',
                    last_capabilities_json='{}'::jsonb,started_at=now(),stopped_at=NULL,last_health_at=now() WHERE agent_id=$1",
                [old.claim.agent_id.into(),pid.into()])).await.map_err(|_| held())?;
        }
        _ => {}
    }
    tx.commit().await.map_err(|_| held())
}
