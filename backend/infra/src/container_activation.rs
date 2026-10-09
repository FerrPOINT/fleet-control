//! Transactions couple activation progress, launch custody and effective publication.
use super::*;
use app::container_activation::{Activation, Claim, Phase, held, stopped};
use app::container_runtime::ContainerLaunch;
use sea_orm::DatabaseTransaction;

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
    owner: Uuid,
) -> Result<Vec<domain::AgentConfigRevision>, AppError> {
    let rows = repo.db.query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT agent_id,revision FROM runtime_container_activations WHERE record#>>'{claim,controller_id}'=$1
            AND record->>'phase' NOT IN ('committed','rolled_back') ORDER BY agent_id LIMIT 16",
        [owner.to_string().into()])).await.map_err(|_| held())?;
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

async fn checked(tx: &DatabaseTransaction, claim: &Claim) -> Result<(), AppError> {
    let agent = container_runtime::lock(tx, claim.agent_id).await?;
    if agent.try_get::<String>("", "kind").map_err(|_| held())? != "hermes"
        || agent
            .try_get::<Option<shared::Timestamp>>("", "archived_at")
            .map_err(|_| held())?
            .is_some()
    {
        return Err(held());
    }
    let row = tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT h.effective_revision,r.snapshot FROM agent_config_heads h JOIN agent_config_revisions r
            ON r.agent_id=h.agent_id AND r.revision=h.desired_revision
         WHERE h.agent_id=$1 AND h.desired_revision=$2 AND h.draining AND r.state='activating'
            AND r.claimed_at IS NOT NULL AND r.validation_errors='[]'::jsonb
            AND NOT EXISTS(SELECT 1 FROM session_agent_runs WHERE agent_id=$1 AND state IN ('pending','running','waiting','stopping'))
            AND NOT EXISTS(SELECT 1 FROM hermes_dispatch_journal WHERE agent_id=$1 AND state IN ('prepared','submitted'))
            AND NOT EXISTS(SELECT 1 FROM message_dispatch_outbox WHERE agent_id=$1 AND state IN ('dispatching','uncertain'))
            AND NOT EXISTS(SELECT 1 FROM runtime_container_recoveries x JOIN runtime_container_launches l USING(generation) WHERE l.agent_id=$1)",
        [claim.agent_id.into(),claim.revision.into()])).await.map_err(|_| held())?.ok_or_else(held)?;
    if row
        .try_get::<Option<i64>>("", "effective_revision")
        .map_err(|_| held())?
        != claim.previous_revision
        || runtime::container_control::canonical_hash(
            &row.try_get::<Value>("", "snapshot").map_err(|_| held())?,
        )? != claim.configuration_sha256
    {
        return Err(held());
    }
    for (column, expected) in [
        ("runtime_path", &claim.previous.prepared.paths.runtime),
        ("config_path", &claim.previous.prepared.paths.config),
        ("workspace_path", &claim.previous.prepared.paths.workspace),
        ("logs_path", &claim.previous.prepared.paths.logs),
    ] {
        if agent.try_get::<String>("", column).map_err(|_| held())? != *expected {
            return Err(held());
        }
    }
    if agent
        .try_get::<Option<i32>>("", "api_port")
        .map_err(|_| held())?
        != claim.previous.prepared.api_port
    {
        return Err(held());
    }
    Ok(())
}

pub(super) async fn claim(
    repo: &PostgresFleetRepository,
    claim: &Claim,
) -> Result<Activation, AppError> {
    let tx = repo.db.begin().await.map_err(|_| held())?;
    checked(&tx, claim).await?;
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
    let planned = Activation::planned(claim.clone());
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
    checked(&tx, &old.claim).await?;
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
