use crate::{PostgresFleetRepository, controller_recovery as owners, controller_recovery_delivery};
use app::runtime_launch::{
    ControllerRecoveryCommand, ControllerStopDelivery, ControllerStopIntent, RuntimeLaunchRecord,
};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseTransaction, Statement, TransactionTrait};
use serde_json::{Value, json};
use shared::AppError;
use uuid::Uuid;

use crate::runtime::container_control::{
    ContainerObservation, ContainerReceipt, ContainerStopReceipt, canonical_hash,
};

pub(crate) async fn read<C: ConnectionTrait>(
    db: &C,
    launch_id: Uuid,
) -> Result<Option<ControllerStopDelivery>, AppError> {
    let Some(row) = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT intent,intent_sha256,dispatch_command,dispatch_command_sha256,native_outcome,native_outcome_sha256
         FROM runtime_controller_stop_deliveries WHERE launch_id=$1", [launch_id.into()]))
        .await.map_err(AppError::database)? else { return Ok(None); };
    let intent: ControllerStopIntent =
        serde_json::from_value(row.try_get("", "intent").map_err(AppError::database)?)
            .map_err(|_| owners::held())?;
    let command: Option<Value> = row
        .try_get("", "dispatch_command")
        .map_err(AppError::database)?;
    let outcome: Option<Value> = row
        .try_get("", "native_outcome")
        .map_err(AppError::database)?;
    if intent.launch_id != launch_id
        || intent.operation_id != launch_id
        || canonical_hash(&intent)?
            != row
                .try_get::<String>("", "intent_sha256")
                .map_err(AppError::database)?
        || command.as_ref().map(canonical_hash).transpose()?
            != row
                .try_get::<Option<String>>("", "dispatch_command_sha256")
                .map_err(AppError::database)?
        || outcome.as_ref().map(canonical_hash).transpose()?
            != row
                .try_get::<Option<String>>("", "native_outcome_sha256")
                .map_err(AppError::database)?
    {
        return Err(owners::held());
    }
    Ok(Some(ControllerStopDelivery {
        intent,
        dispatch_command: command
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| owners::held())?,
        native_outcome: outcome,
    }))
}

async fn launch(
    txn: &DatabaseTransaction,
    agent_id: Uuid,
    launch_id: Uuid,
) -> Result<RuntimeLaunchRecord, AppError> {
    owners::lock_agent(txn, agent_id).await?;
    let row = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT binding,state,pid FROM runtime_launches WHERE id=$1 AND agent_id=$2 FOR UPDATE",
            [launch_id.into(), agent_id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(owners::held)?;
    Ok(RuntimeLaunchRecord {
        binding: serde_json::from_value(row.try_get("", "binding").map_err(AppError::database)?)
            .map_err(|_| owners::held())?,
        state: row.try_get("", "state").map_err(AppError::database)?,
        pid: row.try_get("", "pid").map_err(AppError::database)?,
        controller_recovery: true,
    })
}

async fn intent(
    txn: &DatabaseTransaction,
    command: &ControllerRecoveryCommand,
    original: &RuntimeLaunchRecord,
    require_live: bool,
) -> Result<ControllerStopIntent, AppError> {
    let record = owners::read(txn, command.request.id)
        .await?
        .ok_or_else(owners::held)?;
    if record.request != command.request
        || record.epoch != command.epoch
        || (require_live
            && (record.state != "acknowledged"
                || !record.lease_valid
                || record.lease_version != command.lease_version
                || record.lease_expires_at != command.lease_expires_at))
    {
        return Err(owners::held());
    }
    let saved = controller_recovery_delivery::read(txn, record.request.id)
        .await?
        .ok_or_else(owners::held)?;
    let receipt = saved.native_receipt.as_ref().ok_or_else(owners::held)?;
    if !saved.dispatch_claimed
        || saved.command.request != record.request
        || saved.command.epoch != record.epoch
        || saved.native_receipt_sha256 != record.native_receipt_sha256
    {
        return Err(owners::held());
    }
    // Terminal settlement uses the original started witness, never the current lease as authority.
    let mut started = original.clone();
    started.state = "gateway_started".into();
    crate::runtime::controller_recovery_wire::validate_receipt(receipt, &saved.command, &started)?;
    let snapshot: crate::runtime::container_control::ContainerSnapshot = serde_json::from_value(
        receipt
            .pointer("/witness/receipt/snapshot")
            .cloned()
            .ok_or_else(owners::held)?,
    )
    .map_err(|_| owners::held())?;
    let pid = original.pid.ok_or_else(owners::held)?;
    if i32::try_from(snapshot.init_pid).ok() != Some(pid)
        || (require_live && original.state != "gateway_started")
    {
        return Err(owners::held());
    }
    Ok(ControllerStopIntent {
        launch_id: original.binding.id,
        agent_id: original.binding.agent_id,
        operation_id: original.binding.id,
        launch_sha256: canonical_hash(&original.binding)?,
        snapshot_sha256: canonical_hash(&snapshot)?,
        pid,
    })
}

pub(crate) async fn retain(
    repo: &PostgresFleetRepository,
    command: &ControllerRecoveryCommand,
) -> Result<ControllerStopDelivery, AppError> {
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let original = launch(&txn, command.request.agent_id, command.request.launch_id).await?;
    let expected = intent(&txn, command, &original, true).await?;
    if let Some(saved) = read(&txn, original.binding.id).await? {
        if saved.intent != expected {
            return Err(owners::held());
        }
        return Ok(saved);
    }
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_controller_stop_deliveries(launch_id,intent,intent_sha256) VALUES($1,$2,$3)",
        [expected.launch_id.into(), json!(expected).into(), canonical_hash(&expected)?.into()]))
        .await.map_err(AppError::database)?;
    let saved = read(&txn, expected.launch_id)
        .await?
        .ok_or_else(owners::held)?;
    txn.commit().await.map_err(AppError::database)?;
    Ok(saved)
}

pub(crate) async fn claim(
    repo: &PostgresFleetRepository,
    command: &ControllerRecoveryCommand,
) -> Result<bool, AppError> {
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let original = launch(&txn, command.request.agent_id, command.request.launch_id).await?;
    let saved = read(&txn, original.binding.id)
        .await?
        .ok_or_else(owners::held)?;
    if saved.dispatch_command.is_some() {
        return Ok(false);
    }
    if saved.intent != intent(&txn, command, &original, true).await? {
        return Err(owners::held());
    }
    let changed = txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_controller_stop_deliveries SET dispatch_command=$2,dispatch_command_sha256=$3
         WHERE launch_id=$1 AND dispatch_command IS NULL",
        [original.binding.id.into(), json!(command).into(), canonical_hash(command)?.into()]))
        .await.map_err(AppError::database)?;
    if changed.rows_affected() != 1 {
        return Err(owners::held());
    }
    txn.commit().await.map_err(AppError::database)?;
    Ok(true)
}

fn validate_outcome(
    outcome: &Value,
    intent: &ControllerStopIntent,
    original: &RuntimeLaunchRecord,
) -> Result<(), AppError> {
    if outcome.as_object().map(|object| object.len()) != Some(2)
        || serde_json::to_vec(outcome)
            .map_err(|_| owners::held())?
            .len()
            > 65536
    {
        return Err(owners::held());
    }
    let receipt = outcome.get("receipt").cloned().ok_or_else(owners::held)?;
    let registration = &original
        .binding
        .container
        .as_ref()
        .ok_or_else(owners::held)?
        .registration;
    match outcome.get("kind").and_then(Value::as_str) {
        Some("stop") => {
            let receipt: ContainerStopReceipt =
                serde_json::from_value(receipt).map_err(|_| owners::held())?;
            crate::runtime::container_control::validate_stop_receipt(
                &receipt,
                registration,
                intent.operation_id,
                &intent.snapshot_sha256,
                0,
            )?;
        }
        Some("observe") => {
            let receipt: ContainerReceipt =
                serde_json::from_value(receipt).map_err(|_| owners::held())?;
            crate::runtime::container_control::validate_receipt(
                &receipt,
                registration,
                0,
                "observe",
            )?;
            if receipt.observation != ContainerObservation::NamespaceExited
                || canonical_hash(receipt.snapshot.as_ref().ok_or_else(owners::held)?)?
                    != intent.snapshot_sha256
            {
                return Err(owners::held());
            }
        }
        _ => return Err(owners::held()),
    }
    Ok(())
}

/// Trusted native proof only: this private operation is not an HTTP receipt submission API.
pub(crate) async fn settle(
    repo: &PostgresFleetRepository,
    launch_id: Uuid,
    outcome: &Value,
) -> Result<ControllerStopDelivery, AppError> {
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let saved = read(&txn, launch_id).await?.ok_or_else(owners::held)?;
    let original = launch(&txn, saved.intent.agent_id, launch_id).await?;
    let saved = read(&txn, launch_id).await?.ok_or_else(owners::held)?;
    let command = saved.dispatch_command.as_ref().ok_or_else(owners::held)?;
    if saved.intent != intent(&txn, command, &original, false).await? {
        return Err(owners::held());
    }
    validate_outcome(outcome, &saved.intent, &original)?;
    if let Some(prior) = &saved.native_outcome {
        if prior != outcome {
            return Err(AppError::conflict(
                "original namespace exit outcome changed",
            ));
        }
        // An old receipt cannot overwrite a later launch's runtime metadata.
        settle_free_chat_runs(&txn, &original).await?;
        txn.commit().await.map_err(AppError::database)?;
        return Ok(saved);
    }
    if original.state != "gateway_started" {
        return Err(owners::held());
    }
    let changed = txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_controller_stop_deliveries SET native_outcome=$2,native_outcome_sha256=$3
         WHERE launch_id=$1 AND native_outcome IS NULL",
        [launch_id.into(), outcome.clone().into(), canonical_hash(outcome)?.into()])).await.map_err(AppError::database)?;
    if changed.rows_affected() != 1 {
        return Err(owners::held());
    }
    let changed = txn
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE runtime_launches SET state='gateway_exited',observed_at=clock_timestamp()
         WHERE id=$1 AND state='gateway_started' AND pid=$2 AND binding=$3",
            [
                launch_id.into(),
                saved.intent.pid.into(),
                json!(original.binding).into(),
            ],
        ))
        .await
        .map_err(AppError::database)?;
    if changed.rows_affected() != 1 {
        return Err(owners::held());
    }
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE agents SET status=CASE WHEN archived_at IS NULL THEN 'stopped' ELSE status END WHERE id=$1",
        [saved.intent.agent_id.into()])).await.map_err(AppError::database)?;
    let changed = txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE agent_runtime SET pid=NULL,desired_state='stopped',health_status='gateway_exited',
         health_detail='Original container namespace exit confirmed after controller recovery',
         last_capabilities_json='{}'::jsonb,last_health_at=clock_timestamp(),stopped_at=clock_timestamp()
         WHERE agent_id=$1", [saved.intent.agent_id.into()])).await.map_err(AppError::database)?;
    if changed.rows_affected() != 1 {
        return Err(owners::held());
    }
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO audit_log(id,actor_user_id,action,entity_type,entity_id,payload,created_at)
         VALUES($1,NULL,'runtime.controller_stop.outcome','runtime_launch',$2,$3,clock_timestamp())",
        [Uuid::new_v4().into(), launch_id.to_string().into(),
         json!({"native_outcome_sha256":canonical_hash(outcome)?,"recovery_id":command.request.id}).into()]))
        .await.map_err(AppError::database)?;
    settle_free_chat_runs(&txn, &original).await?;
    let saved = read(&txn, launch_id).await?.ok_or_else(owners::held)?;
    txn.commit().await.map_err(AppError::database)?;
    Ok(saved)
}

async fn settle_free_chat_runs(
    txn: &DatabaseTransaction,
    original: &RuntimeLaunchRecord,
) -> Result<(), AppError> {
    // A namespace exit proves interruption, not an assistant result or a control ACK.
    let runs = txn.query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT r.id,r.session_id,j.message_id FROM session_agent_runs r
         JOIN hermes_dispatch_journal j ON j.run_id=r.id AND j.agent_id=r.agent_id AND j.session_id=r.session_id
         JOIN runtime_launch_endpoints e ON e.launch_id=$1 AND e.origin=j.origin AND e.pid=$3
         JOIN session_messages m ON m.id=j.message_id AND m.session_id=r.session_id
         JOIN message_dispatch_outbox o ON o.message_id=m.id AND o.agent_id=r.agent_id
         JOIN agent_sessions s ON s.id=r.session_id AND s.agent_id=r.agent_id
         WHERE r.agent_id=$2 AND r.state IN ('pending','running','waiting','stopping')
           AND r.runtime_run_id IS NOT NULL AND r.runtime_session_id IS NOT NULL
           AND j.state='accepted' AND j.capabilities->'fleet_launch'=$4
           AND j.run_role=r.run_role AND j.submitted_at IS NOT NULL AND j.accepted_at IS NOT NULL
           AND m.runtime_message_id=r.runtime_run_id AND m.delivery_state='dispatched' AND o.state='dispatched'
           AND NOT EXISTS(SELECT 1 FROM task_chat_bindings b WHERE b.session_id=r.session_id)
           AND NOT EXISTS(SELECT 1 FROM pm_run_bindings b WHERE b.session_run_id=r.id OR b.session_id=r.session_id)
         ORDER BY r.session_id,r.id LIMIT 101",
        [original.binding.id.into(),original.binding.agent_id.into(),original.pid.into(),
         json!({"version":1,"launch_id":original.binding.id}).into()]))
        .await.map_err(AppError::database)?;
    if runs.len() > 100 {
        return Err(owners::held());
    }
    for row in runs {
        let run: Uuid = row.try_get("", "id").map_err(AppError::database)?;
        let session: Uuid = row.try_get("", "session_id").map_err(AppError::database)?;
        let message: Uuid = row.try_get("", "message_id").map_err(AppError::database)?;
        // Match the agent -> session -> run ordering used by acceptance/control writers.
        txn.query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM agent_sessions WHERE id=$1 FOR NO KEY UPDATE",
            [session.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(owners::held)?;
        txn.query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM session_agent_runs WHERE id=$1 FOR UPDATE",
            [run.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(owners::held)?;
        let changed = txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "UPDATE session_agent_runs SET state='cancelled',
             last_error='Original runtime namespace exit confirmed',last_event_at=clock_timestamp(),updated_at=clock_timestamp()
             WHERE id=$1 AND state IN ('pending','running','waiting','stopping')", [run.into()]))
            .await.map_err(AppError::database)?;
        if changed.rows_affected() != 1 {
            continue;
        }
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE session_messages SET delivery_state='completed',delivery_error=NULL
             WHERE id=$1 AND delivery_state='dispatched'",
            [message.into()],
        ))
        .await
        .map_err(AppError::database)?;
        txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "UPDATE runtime_approval_requests a SET state='cancelled',resolved_by_user_id=NULL,resolved_at=clock_timestamp()
             FROM session_agent_runs r WHERE r.id=$1 AND a.session_run_id=r.id AND a.session_id=r.session_id
               AND a.agent_id=r.agent_id AND a.runtime_run_id=r.runtime_run_id AND a.state='pending'", [run.into()]))
            .await.map_err(AppError::database)?;
        // Existing row triggers append durable run/message/approval events in this same transaction.
        txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO audit_log(id,actor_user_id,action,entity_type,entity_id,payload,created_at)
             VALUES($1,NULL,'runtime.namespace_exit.run_cancelled','session_agent_run',$2,$3,clock_timestamp())",
            [Uuid::new_v4().into(),run.to_string().into(),
             json!({"launch_id":original.binding.id,"session_id":session,"reason":"namespace_exit"}).into()]))
            .await.map_err(AppError::database)?;
    }
    Ok(())
}
