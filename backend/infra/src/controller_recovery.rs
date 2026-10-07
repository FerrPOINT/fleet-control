use crate::PostgresFleetRepository;
use app::runtime_launch::{ControllerRecoveryRecord, ControllerRecoveryRequest};
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseTransaction, QueryResult, Statement, TransactionTrait,
};
use shared::AppError;
use uuid::Uuid;

fn held() -> AppError {
    AppError::Unavailable("original controller recovery remains fenced".into())
}

fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn decode(row: QueryResult) -> Result<ControllerRecoveryRecord, AppError> {
    let request: ControllerRecoveryRequest =
        serde_json::from_value(row.try_get("", "request").map_err(AppError::database)?)
            .map_err(|_| held())?;
    if request.id != row.try_get::<Uuid>("", "id").map_err(AppError::database)?
        || request.launch_id
            != row
                .try_get::<Uuid>("", "launch_id")
                .map_err(AppError::database)?
        || request.agent_id
            != row
                .try_get::<Uuid>("", "agent_id")
                .map_err(AppError::database)?
        || request.controller_id
            != row
                .try_get::<Uuid>("", "controller_id")
                .map_err(AppError::database)?
        || request.predecessor_id
            != row
                .try_get::<Option<Uuid>>("", "predecessor_id")
                .map_err(AppError::database)?
        || crate::runtime_launches::snapshot_hash(
            &serde_json::to_value(&request).map_err(AppError::internal)?,
        )? != row
            .try_get::<String>("", "request_sha256")
            .map_err(AppError::database)?
    {
        return Err(held());
    }
    Ok(ControllerRecoveryRecord {
        request,
        epoch: row.try_get("", "epoch").map_err(AppError::database)?,
        state: row.try_get("", "state").map_err(AppError::database)?,
        lease_version: row
            .try_get("", "lease_version")
            .map_err(AppError::database)?,
        lease_expires_at: row
            .try_get::<shared::Timestamp>("", "lease_expires_at")
            .map_err(AppError::database)?
            .to_rfc3339(),
        lease_valid: row.try_get("", "lease_valid").map_err(AppError::database)?,
        native_receipt_sha256: row
            .try_get("", "native_receipt_sha256")
            .map_err(AppError::database)?,
    })
}

const COLUMNS: &str = "id,launch_id,agent_id,controller_id,predecessor_id,request,request_sha256,epoch,state,lease_version,lease_expires_at,
    lease_expires_at>clock_timestamp() AS lease_valid,native_receipt_sha256";

async fn read<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
) -> Result<Option<ControllerRecoveryRecord>, AppError> {
    db.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        format!("SELECT {COLUMNS} FROM runtime_controller_recoveries WHERE id=$1"),
        [id.into()],
    ))
    .await
    .map_err(AppError::database)?
    .map(decode)
    .transpose()
}

async fn lock_agent(txn: &DatabaseTransaction, agent: Uuid) -> Result<(), AppError> {
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id=$1 FOR UPDATE",
        [agent.into()],
    ))
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::not_found("agent", agent))?;
    Ok(())
}

impl PostgresFleetRepository {
    pub(crate) async fn reserve_controller_recovery(
        &self,
        request: &ControllerRecoveryRequest,
    ) -> Result<ControllerRecoveryRecord, AppError> {
        if [
            request.id,
            request.launch_id,
            request.agent_id,
            request.controller_id,
            request.original_controller_id,
        ]
        .into_iter()
        .any(|id| id.is_nil())
            || request
                .predecessor_id
                .is_some_and(|id| id.is_nil() || id == request.id)
            || request.controller_id == request.original_controller_id
            || request.agent_pid <= 0
            || request.controller_snapshot.init_pid == 0
            || !hash(&request.controller_snapshot.container_id)
            || !hash(&request.controller_snapshot.inventory_sha256)
            || request.controller_snapshot.started_at.is_empty()
            || request.controller_snapshot.started_at.starts_with("0001-")
            || request.controller_snapshot.started_at.len() > 128
            || !request
                .controller_snapshot
                .started_at
                .bytes()
                .all(|byte| byte.is_ascii_graphic())
            || [
                &request.launch_sha256,
                &request.mapping_sha256,
                &request.registration_sha256,
            ]
            .into_iter()
            .any(|value| !hash(value))
        {
            return Err(AppError::validation("invalid controller recovery identity"));
        }
        let value = serde_json::to_value(request).map_err(AppError::internal)?;
        let txn = self.db.begin().await.map_err(AppError::database)?;
        lock_agent(&txn, request.agent_id).await?;
        if let Some(previous) = read(&txn, request.id).await? {
            if previous.request != *request {
                return Err(AppError::conflict(
                    "original controller recovery payload changed",
                ));
            }
            // Historical readback does not renew an expired lease or repeat a native effect.
            return Ok(previous);
        }
        let launch = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT binding,state,pid FROM runtime_launches WHERE id=$1 AND agent_id=$2 FOR UPDATE",
            [request.launch_id.into(), request.agent_id.into()])).await.map_err(AppError::database)?.ok_or_else(held)?;
        let binding: app::runtime_launch::RuntimeLaunchBinding =
            serde_json::from_value(launch.try_get("", "binding").map_err(AppError::database)?)
                .map_err(|_| held())?;
        crate::runtime_launches::validate_container_binding(&binding)?;
        let container = binding.container.as_ref().ok_or_else(held)?;
        let mapping = container.mount_mapping.as_ref().ok_or_else(held)?;
        if launch
            .try_get::<String>("", "state")
            .map_err(AppError::database)?
            != "gateway_started"
            || launch
                .try_get::<Option<i32>>("", "pid")
                .map_err(AppError::database)?
                != Some(request.agent_pid)
            || binding.controller_id != request.original_controller_id
            || request.launch_sha256
                != crate::runtime_launches::snapshot_hash(
                    &serde_json::to_value(&binding).map_err(AppError::internal)?,
                )?
            || request.mapping_sha256 != crate::runtime::container_control::canonical_hash(mapping)?
            || request.registration_sha256
                != crate::runtime::container_control::canonical_hash(&container.registration)?
            || request.controller_snapshot.container_id != mapping.snapshot.container_id
            || request.controller_snapshot.inventory_sha256 != mapping.snapshot.inventory_sha256
            || request.controller_snapshot.started_at == mapping.snapshot.started_at
        {
            return Err(held());
        }
        let current = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            format!("SELECT {COLUMNS} FROM runtime_controller_recoveries WHERE launch_id=$1 AND state<>'superseded' FOR UPDATE"),
            [request.launch_id.into()])).await.map_err(AppError::database)?.map(decode).transpose()?;
        let epoch = match current {
            None if request.predecessor_id.is_none() => 1,
            Some(previous)
                if request.predecessor_id == Some(previous.request.id)
                    && previous.state == "acknowledged"
                    && !previous.lease_valid
                    && previous.request.controller_id != request.controller_id
                    && previous.request.controller_snapshot.started_at
                        != request.controller_snapshot.started_at =>
            {
                let epoch = previous.epoch.checked_add(1).ok_or_else(held)?;
                txn.execute(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "UPDATE runtime_controller_recoveries SET state='superseded'
                     WHERE id=$1 AND state='acknowledged' AND lease_expires_at<=clock_timestamp()",
                    [previous.request.id.into()],
                ))
                .await
                .map_err(AppError::database)?;
                epoch
            }
            _ => return Err(held()),
        };
        txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO runtime_controller_recoveries
             (id,launch_id,agent_id,controller_id,predecessor_id,epoch,request,request_sha256,lease_expires_at)
             VALUES($1,$2,$3,$4,$5,$6,$7,$8,clock_timestamp()+interval '30 seconds')",
            [request.id.into(), request.launch_id.into(), request.agent_id.into(), request.controller_id.into(),
                request.predecessor_id.into(), epoch.into(), value.clone().into(),
                crate::runtime_launches::snapshot_hash(&value)?.into()]))
            .await.map_err(AppError::database)?;
        let result = read(&txn, request.id).await?.ok_or_else(held)?;
        txn.commit().await.map_err(AppError::database)?;
        Ok(result)
    }

    pub(crate) async fn read_controller_recovery(
        &self,
        id: Uuid,
    ) -> Result<Option<ControllerRecoveryRecord>, AppError> {
        read(&self.db, id).await
    }

    pub(crate) async fn heartbeat_controller_recovery(
        &self,
        id: Uuid,
        controller: Uuid,
        version: i64,
    ) -> Result<ControllerRecoveryRecord, AppError> {
        let txn = self.db.begin().await.map_err(AppError::database)?;
        let original = read(&txn, id).await?.ok_or_else(held)?;
        lock_agent(&txn, original.request.agent_id).await?;
        let row = txn
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                format!(
                    "UPDATE runtime_controller_recoveries SET lease_version=lease_version+1,
                lease_expires_at=clock_timestamp()+interval '30 seconds'
                WHERE id=$1 AND controller_id=$2 AND lease_version=$3 AND state<>'superseded'
                  AND lease_expires_at>clock_timestamp() RETURNING {COLUMNS}"
                ),
                [id.into(), controller.into(), version.into()],
            ))
            .await
            .map_err(AppError::database)?
            .ok_or_else(held)?;
        let result = decode(row)?;
        txn.commit().await.map_err(AppError::database)?;
        Ok(result)
    }

    pub(crate) async fn acknowledge_controller_recovery(
        &self,
        id: Uuid,
        controller: Uuid,
        version: i64,
        native_receipt_sha256: &str,
    ) -> Result<ControllerRecoveryRecord, AppError> {
        if !hash(native_receipt_sha256) {
            return Err(AppError::validation(
                "invalid private controller receipt hash",
            ));
        }
        let txn = self.db.begin().await.map_err(AppError::database)?;
        let original = read(&txn, id).await?.ok_or_else(held)?;
        lock_agent(&txn, original.request.agent_id).await?;
        let original = read(&txn, id).await?.ok_or_else(held)?;
        if original.request.controller_id != controller || original.lease_version != version {
            return Err(held());
        }
        if original.native_receipt_sha256.as_deref() == Some(native_receipt_sha256) {
            // A saved acknowledgement is readback, not live owner authority.
            return Ok(original);
        }
        if original.state != "reserved" || !original.lease_valid {
            return Err(held());
        }
        let row = txn
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                format!(
                    "UPDATE runtime_controller_recoveries SET state='acknowledged',
                native_receipt_sha256=$4,acknowledged_at=clock_timestamp()
                WHERE id=$1 AND controller_id=$2 AND lease_version=$3 AND state='reserved'
                  AND lease_expires_at>clock_timestamp() RETURNING {COLUMNS}"
                ),
                [
                    id.into(),
                    controller.into(),
                    version.into(),
                    native_receipt_sha256.into(),
                ],
            ))
            .await
            .map_err(AppError::database)?
            .ok_or_else(held)?;
        let result = decode(row)?;
        txn.commit().await.map_err(AppError::database)?;
        Ok(result)
    }
}
