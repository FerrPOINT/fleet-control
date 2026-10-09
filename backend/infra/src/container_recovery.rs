use super::*;
use crate::runtime::container_control::canonical_hash;
use app::container_runtime::{ContainerLaunch, ContainerRecovery, ContainerRecoveryCommand};
use sea_orm::{DatabaseTransaction, QueryResult};

fn held() -> AppError {
    AppError::Unavailable("Original controller recovery remains held".into())
}

fn deadline(value: &str) -> Result<shared::Timestamp, AppError> {
    let parsed = chrono::DateTime::parse_from_rfc3339(value).map_err(|_| held())?;
    if parsed.offset().local_minus_utc() != 0 {
        return Err(held());
    }
    Ok(parsed)
}

const SELECT: &str =
    "SELECT command,receipt,lease,lease_receipt,expires_at>clock_timestamp() AS lease_valid
    FROM runtime_container_recoveries WHERE generation=$1 ORDER BY epoch DESC LIMIT 1";

fn decode(row: QueryResult) -> Result<ContainerRecovery, AppError> {
    Ok(ContainerRecovery {
        command: serde_json::from_value(row.try_get::<Value>("", "command").map_err(|_| held())?)
            .map_err(|_| held())?,
        receipt: row.try_get("", "receipt").map_err(|_| held())?,
        lease: serde_json::from_value(row.try_get::<Value>("", "lease").map_err(|_| held())?)
            .map_err(|_| held())?,
        lease_receipt: row.try_get("", "lease_receipt").map_err(|_| held())?,
        lease_valid: row.try_get("", "lease_valid").map_err(|_| held())?,
    })
}

pub(super) async fn get(
    repo: &PostgresFleetRepository,
    generation: Uuid,
) -> Result<Option<ContainerRecovery>, AppError> {
    repo.db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            SELECT,
            [generation.into()],
        ))
        .await
        .map_err(|_| held())?
        .map(decode)
        .transpose()
}

pub(super) async fn get_txn(
    txn: &DatabaseTransaction,
    generation: Uuid,
) -> Result<Option<ContainerRecovery>, AppError> {
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        SELECT,
        [generation.into()],
    ))
    .await
    .map_err(|_| held())?
    .map(decode)
    .transpose()
}

pub(super) async fn claim(
    repo: &PostgresFleetRepository,
    launch: &ContainerLaunch,
    command: &ContainerRecoveryCommand,
) -> Result<(), AppError> {
    let r = &command.request;
    let b = &launch.prepared.container;
    let m = &b.mapped.as_ref().ok_or_else(held)?.mapping;
    if r.id.is_nil()
        || r.controller_id.is_nil()
        || r.controller_id == launch.controller_id
        || r.agent_id != launch.prepared.agent_id
        || r.launch_id != b.registration.generation
        || r.original_controller_id != launch.controller_id
        || r.launch_sha256 != crate::runtime::container_control::launch_hash(launch)?
        || r.mapping_sha256 != canonical_hash(m)?
        || r.registration_sha256 != canonical_hash(&b.registration)?
        || r.controller_snapshot.container_id != m.controller.container_id
        || r.controller_snapshot.inventory_sha256 != m.snapshot.inventory_sha256
        || r.controller_snapshot.started_at == m.snapshot.started_at
        || r.controller_snapshot.init_pid == 0
        || r.controller_snapshot.started_at.starts_with("0001-")
        || chrono::DateTime::parse_from_rfc3339(&r.controller_snapshot.started_at).is_err()
        || launch
            .snapshot
            .as_ref()
            .and_then(|v| v["init_pid"].as_i64())
            != Some(r.agent_pid as i64)
        || command.lease_version != 1
    {
        return Err(held());
    }
    let txn = repo.db.begin().await.map_err(|_| held())?;
    crate::container_runtime::lock(&txn, r.agent_id).await?;
    let original = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT prepared,controller_id,state,snapshot,origin,stop_id FROM runtime_container_launches WHERE generation=$1",
        [r.launch_id.into()])).await.map_err(|_| held())?.ok_or_else(held)?;
    let stored = json!({"prepared":original.try_get::<Value>("","prepared").map_err(|_| held())?,
        "controller_id":original.try_get::<Uuid>("","controller_id").map_err(|_| held())?,
        "snapshot":original.try_get::<Option<Value>>("","snapshot").map_err(|_| held())?,
        "origin":original.try_get::<Option<String>>("","origin").map_err(|_| held())?,
        "stop_id":original.try_get::<Uuid>("","stop_id").map_err(|_| held())?});
    if canonical_hash(&stored)? != r.launch_sha256 {
        return Err(held());
    }
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO runtime_container_recoveries(id,generation,epoch,command,lease,expires_at)
         VALUES($1,$2,$3,$4,$4,$5)",
        [
            r.id.into(),
            r.launch_id.into(),
            command.epoch.into(),
            json!(command).into(),
            deadline(&command.lease_expires_at)?.into(),
        ],
    ))
    .await
    .map_err(|_| held())?;
    txn.commit().await.map_err(|_| held())
}

pub(super) async fn acknowledge(
    repo: &PostgresFleetRepository,
    command: &ContainerRecoveryCommand,
    receipt: Value,
    heartbeat: bool,
) -> Result<(), AppError> {
    let launch = crate::container_runtime::get_generation(
        repo,
        command.request.agent_id,
        command.request.launch_id,
    )
    .await?
    .ok_or_else(held)?;
    let b = &launch.prepared.container;
    if b.registration.generation != command.request.launch_id {
        return Err(held());
    }
    let files = crate::runtime::container_control::ContainerLaunchFiles {
        policy: b.policy.clone(),
        compose: b.compose.clone().into(),
        journal: b.journal.clone().into(),
        stop_journal: b.stop_journal.clone().into(),
        mapped: b.mapped.clone(),
        recovery: Some(command.clone()),
    };
    if heartbeat {
        if receipt
            .as_object()
            .is_none_or(|o| o.len() != 2 || !o.contains_key("ack") || !o.contains_key("receipt"))
        {
            return Err(held());
        }
        crate::runtime::container_control::validate_recovery_value(
            &files,
            &b.registration,
            "heartbeat_controller",
            0,
            &receipt["ack"],
        )?;
        let observed: crate::runtime::container_control::ContainerReceipt =
            serde_json::from_value(receipt["receipt"].clone()).map_err(|_| held())?;
        crate::runtime::container_control::validate_receipt(
            &observed,
            &b.registration,
            0,
            "observe",
        )?;
    } else {
        crate::runtime::container_control::validate_recovery_value(
            &files,
            &b.registration,
            "read_controller_recovery",
            0,
            &receipt,
        )?;
    }
    let txn = repo.db.begin().await.map_err(|_| held())?;
    crate::container_runtime::lock(&txn, command.request.agent_id).await?;
    let sql = if heartbeat {
        "UPDATE runtime_container_recoveries SET lease_receipt=$1 WHERE id=$2 AND lease=$3 AND (lease_receipt IS NULL OR lease_receipt=$1)"
    } else {
        "UPDATE runtime_container_recoveries SET receipt=$1 WHERE id=$2 AND command=$3 AND (receipt IS NULL OR receipt=$1)"
    };
    let result = txn
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [
                receipt.into(),
                command.request.id.into(),
                json!(command).into(),
            ],
        ))
        .await
        .map_err(|_| held())?;
    if result.rows_affected() != 1 {
        return Err(held());
    }
    txn.commit().await.map_err(|_| held())
}

pub(super) async fn renew(
    repo: &PostgresFleetRepository,
    previous: &ContainerRecovery,
    command: &ContainerRecoveryCommand,
) -> Result<(), AppError> {
    if previous.lease.request != command.request
        || previous.lease.epoch != command.epoch
        || previous.lease.lease_version.checked_add(1) != Some(command.lease_version)
    {
        return Err(held());
    }
    let txn = repo.db.begin().await.map_err(|_| held())?;
    crate::container_runtime::lock(&txn, command.request.agent_id).await?;
    let result = txn
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE runtime_container_recoveries SET lease=$1,expires_at=$2,lease_receipt=NULL
         WHERE id=$3 AND lease=$4 AND lease_receipt IS NOT NULL AND receipt IS NOT NULL",
            [
                json!(command).into(),
                deadline(&command.lease_expires_at)?.into(),
                command.request.id.into(),
                json!(previous.lease).into(),
            ],
        ))
        .await
        .map_err(|_| held())?;
    if result.rows_affected() != 1 {
        return Err(held());
    }
    txn.commit().await.map_err(|_| held())
}
