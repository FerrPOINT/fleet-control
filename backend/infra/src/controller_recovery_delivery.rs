use crate::{PostgresFleetRepository, controller_recovery as owners};
use app::runtime_launch::{
    ControllerRecoveryCommand, ControllerRecoveryDelivery, ControllerRecoveryRecord,
    RuntimeLaunchRecord,
};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseTransaction, Statement, TransactionTrait};
use serde_json::Value;
use shared::AppError;
use uuid::Uuid;

pub(crate) async fn current<C: ConnectionTrait>(
    db: &C,
    launch_id: Uuid,
) -> Result<Option<ControllerRecoveryRecord>, AppError> {
    db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        format!("SELECT {} FROM runtime_controller_recoveries WHERE launch_id=$1 AND state<>'superseded'", owners::COLUMNS),
        [launch_id.into()])).await.map_err(AppError::database)?.map(owners::decode).transpose()
}

pub(crate) async fn read<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
) -> Result<Option<ControllerRecoveryDelivery>, AppError> {
    let Some(row) = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT command,command_sha256,dispatch_claimed,native_receipt,native_receipt_sha256
         FROM runtime_controller_recovery_deliveries WHERE recovery_id=$1",
            [id.into()],
        ))
        .await
        .map_err(AppError::database)?
    else {
        return Ok(None);
    };
    let command: ControllerRecoveryCommand =
        serde_json::from_value(row.try_get("", "command").map_err(AppError::database)?)
            .map_err(|_| owners::held())?;
    let receipt: Option<Value> = row
        .try_get("", "native_receipt")
        .map_err(AppError::database)?;
    let hash: Option<String> = row
        .try_get("", "native_receipt_sha256")
        .map_err(AppError::database)?;
    if command.request.id != id
        || crate::runtime::container_control::canonical_hash(&command)?
            != row
                .try_get::<String>("", "command_sha256")
                .map_err(AppError::database)?
        || receipt
            .as_ref()
            .map(crate::runtime::container_control::canonical_hash)
            .transpose()?
            != hash
    {
        return Err(owners::held());
    }
    Ok(Some(ControllerRecoveryDelivery {
        command,
        dispatch_claimed: row
            .try_get("", "dispatch_claimed")
            .map_err(AppError::database)?,
        native_receipt: receipt,
        native_receipt_sha256: hash,
    }))
}

async fn launch(
    txn: &DatabaseTransaction,
    record: &ControllerRecoveryRecord,
) -> Result<RuntimeLaunchRecord, AppError> {
    let row = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT binding,state,pid FROM runtime_launches WHERE id=$1 AND agent_id=$2 FOR UPDATE",
            [
                record.request.launch_id.into(),
                record.request.agent_id.into(),
            ],
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

async fn locked(txn: &DatabaseTransaction, id: Uuid) -> Result<ControllerRecoveryRecord, AppError> {
    let record = owners::read(txn, id).await?.ok_or_else(owners::held)?;
    owners::lock_agent(txn, record.request.agent_id).await?;
    owners::read(txn, id).await?.ok_or_else(owners::held)
}

pub(crate) async fn retain(
    repo: &PostgresFleetRepository,
    command: &ControllerRecoveryCommand,
) -> Result<ControllerRecoveryDelivery, AppError> {
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let record = locked(&txn, command.request.id).await?;
    if let Some(saved) = read(&txn, command.request.id).await? {
        if saved.command != *command {
            return Err(AppError::conflict("original controller command changed"));
        }
        return Ok(saved);
    }
    if record.request != command.request
        || record.epoch != command.epoch
        || record.lease_version != 1
        || command.lease_version != 1
        || record.lease_expires_at != command.lease_expires_at
        || !record.lease_valid
        || record.state != "reserved"
    {
        return Err(owners::held());
    }
    crate::runtime::controller_recovery_wire::validate_command(
        command,
        &launch(&txn, &record).await?,
    )?;
    let value = serde_json::to_value(command).map_err(AppError::internal)?;
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_controller_recovery_deliveries(recovery_id,command,command_sha256) VALUES($1,$2,$3)",
        [command.request.id.into(), value.into(), crate::runtime::container_control::canonical_hash(command)?.into()]))
        .await.map_err(AppError::database)?;
    let saved = read(&txn, command.request.id)
        .await?
        .ok_or_else(owners::held)?;
    txn.commit().await.map_err(AppError::database)?;
    Ok(saved)
}

pub(crate) async fn claim(
    repo: &PostgresFleetRepository,
    id: Uuid,
    controller: Uuid,
) -> Result<bool, AppError> {
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let record = locked(&txn, id).await?;
    let delivery = read(&txn, id).await?.ok_or_else(owners::held)?;
    if record.request.controller_id != controller {
        return Err(owners::held());
    }
    if delivery.dispatch_claimed {
        return Ok(false);
    }
    if !record.lease_valid
        || record.state != "reserved"
        || record.lease_version != 1
        || delivery.command.request != record.request
    {
        return Err(owners::held());
    }
    crate::runtime::controller_recovery_wire::validate_command(
        &delivery.command,
        &launch(&txn, &record).await?,
    )?;
    let result = txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_controller_recovery_deliveries SET dispatch_claimed=true WHERE recovery_id=$1 AND NOT dispatch_claimed", [id.into()]))
        .await.map_err(AppError::database)?;
    if result.rows_affected() != 1 {
        return Err(owners::held());
    }
    txn.commit().await.map_err(AppError::database)?;
    Ok(true)
}

pub(crate) async fn settle(
    repo: &PostgresFleetRepository,
    id: Uuid,
    receipt: &Value,
) -> Result<ControllerRecoveryRecord, AppError> {
    if serde_json::to_vec(receipt)
        .map_err(AppError::internal)?
        .len()
        > 65536
    {
        return Err(owners::held());
    }
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let record = locked(&txn, id).await?;
    let delivery = read(&txn, id).await?.ok_or_else(owners::held)?;
    if !delivery.dispatch_claimed
        || delivery.command.request != record.request
        || delivery.command.epoch != record.epoch
    {
        return Err(owners::held());
    }
    crate::runtime::controller_recovery_wire::validate_receipt(
        receipt,
        &delivery.command,
        &launch(&txn, &record).await?,
    )?;
    let hash = crate::runtime::container_control::canonical_hash(receipt)?;
    if let Some(previous) = delivery.native_receipt {
        if previous != *receipt {
            return Err(AppError::conflict("original controller outcome changed"));
        }
        if record.native_receipt_sha256.as_ref() != Some(&hash) {
            return Err(owners::held());
        }
        return Ok(record);
    }
    if record.state != "reserved" {
        return Err(owners::held());
    }
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_controller_recovery_deliveries SET native_receipt=$2,native_receipt_sha256=$3
         WHERE recovery_id=$1 AND native_receipt IS NULL AND dispatch_claimed", [id.into(), receipt.clone().into(), hash.clone().into()]))
        .await.map_err(AppError::database)?;
    // Persist the historical outcome without changing version, deadline, or live authority.
    let row = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        format!("UPDATE runtime_controller_recoveries SET state='acknowledged',native_receipt_sha256=$2,acknowledged_at=clock_timestamp()
                 WHERE id=$1 AND state='reserved' RETURNING {}", owners::COLUMNS), [id.into(), hash.clone().into()]))
        .await.map_err(AppError::database)?.ok_or_else(owners::held)?;
    let result = owners::decode(row)?;
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO audit_log(id,actor_user_id,action,entity_type,entity_id,payload,created_at)
         VALUES($1,NULL,'runtime.controller_recovery.outcome','runtime_controller_recovery',$2,$3,clock_timestamp())",
        [Uuid::new_v4().into(), id.to_string().into(), serde_json::json!({"native_receipt_sha256":hash,"historical":!result.lease_valid}).into()]))
        .await.map_err(AppError::database)?;
    txn.commit().await.map_err(AppError::database)?;
    Ok(result)
}
