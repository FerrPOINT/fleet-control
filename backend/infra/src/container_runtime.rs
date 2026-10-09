use super::*;
use app::container_runtime::ContainerLaunch;
use sea_orm::{DatabaseTransaction, QueryResult};

fn held() -> AppError {
    AppError::Unavailable("Original container journal requires reconciliation".into())
}

pub(super) async fn idle(txn: &DatabaseTransaction, agent: Uuid) -> Result<(), AppError> {
    let row = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM agent_config_heads WHERE agent_id=$1 AND draining)
            OR EXISTS(SELECT 1 FROM session_agent_runs WHERE agent_id=$1
                AND state IN ('pending','running','waiting','stopping') AND runtime_session_id IS NOT NULL)
            OR EXISTS(SELECT 1 FROM hermes_dispatch_journal WHERE agent_id=$1 AND state IN ('prepared','submitted')) AS busy",
        [agent.into()])).await.map_err(|_| held())?.ok_or_else(held)?;
    if row.try_get::<bool>("", "busy").map_err(|_| held())? {
        return Err(AppError::conflict(
            "Runtime capacity or unknown acceptance remains held",
        ));
    }
    Ok(())
}

pub(super) async fn get(
    repo: &PostgresFleetRepository,
    agent: Uuid,
) -> Result<Option<ContainerLaunch>, AppError> {
    repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT prepared,controller_id,state,snapshot,origin,stop_id FROM runtime_container_launches
         WHERE agent_id=$1 ORDER BY (state<>'exited') DESC,created_at DESC,generation DESC LIMIT 1",
        [agent.into()])).await.map_err(|_| held())?.map(decode).transpose()
}

fn decode(row: QueryResult) -> Result<ContainerLaunch, AppError> {
    Ok(ContainerLaunch {
        prepared: serde_json::from_value(row.try_get::<Value>("", "prepared").map_err(|_| held())?)
            .map_err(|_| held())?,
        controller_id: row.try_get("", "controller_id").map_err(|_| held())?,
        state: row.try_get("", "state").map_err(|_| held())?,
        snapshot: row.try_get("", "snapshot").map_err(|_| held())?,
        origin: row.try_get("", "origin").map_err(|_| held())?,
        stop_id: row.try_get("", "stop_id").map_err(|_| held())?,
    })
}

pub(super) async fn get_generation(
    repo: &PostgresFleetRepository,
    agent: Uuid,
    generation: Uuid,
) -> Result<Option<ContainerLaunch>, AppError> {
    repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT prepared,controller_id,state,snapshot,origin,stop_id FROM runtime_container_launches WHERE agent_id=$1 AND generation=$2",
        [agent.into(),generation.into()])).await.map_err(|_| held())?.map(decode).transpose()
}

pub(super) async fn lock(txn: &DatabaseTransaction, agent: Uuid) -> Result<QueryResult, AppError> {
    txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT kind,status,archived_at,api_port,runtime_path,config_path,workspace_path,logs_path FROM agents WHERE id=$1 FOR UPDATE",
        [agent.into()])).await.map_err(|_| held())?.ok_or_else(held)
}

pub(super) async fn claim(
    repo: &PostgresFleetRepository,
    launch: &ContainerLaunch,
) -> Result<(), AppError> {
    let p = &launch.prepared;
    let txn = repo.db.begin().await.map_err(|_| held())?;
    let agent = lock(&txn, p.agent_id).await?;
    idle(&txn, p.agent_id).await?;
    if launch.state != "claimed"
        || launch.snapshot.is_some()
        || launch.origin.is_some()
        || launch.controller_id.is_nil()
        || launch.stop_id.is_nil()
        || agent.try_get::<String>("", "kind").map_err(|_| held())? != "hermes"
        || !matches!(
            agent
                .try_get::<String>("", "status")
                .map_err(|_| held())?
                .as_str(),
            "ready" | "stopped" | "failed"
        )
        || agent
            .try_get::<Option<shared::Timestamp>>("", "archived_at")
            .map_err(|_| held())?
            .is_some()
        || agent
            .try_get::<Option<i32>>("", "api_port")
            .map_err(|_| held())?
            != p.api_port
    {
        return Err(held());
    }
    for (column, expected) in [
        ("runtime_path", &p.paths.runtime),
        ("config_path", &p.paths.config),
        ("workspace_path", &p.paths.workspace),
        ("logs_path", &p.paths.logs),
    ] {
        if agent.try_get::<String>("", column).map_err(|_| held())? != *expected {
            return Err(held());
        }
    }
    let revision = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT r.revision,r.snapshot FROM agent_config_heads h JOIN agent_config_revisions r
         ON r.agent_id=h.agent_id AND r.revision=h.effective_revision WHERE h.agent_id=$1",
            [p.agent_id.into()],
        ))
        .await
        .map_err(|_| held())?;
    let (number, hash) = match revision {
        Some(row) => (
            Some(row.try_get::<i64>("", "revision").map_err(|_| held())?),
            Some(crate::runtime::container_control::canonical_hash(
                &row.try_get::<Value>("", "snapshot").map_err(|_| held())?,
            )?),
        ),
        None => (None, None),
    };
    if p.configuration_revision != number || p.configuration_sha256 != hash {
        return Err(held());
    }
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_container_launches(generation,agent_id,controller_id,prepared,state,stop_id)
         VALUES($1,$2,$3,$4,'claimed',$5)", [p.container.registration.generation.into(),p.agent_id.into(),
         launch.controller_id.into(),serde_json::to_value(p).map_err(|_| held())?.into(),launch.stop_id.into()]))
        .await.map_err(|_| held())?;
    txn.commit().await.map_err(|_| held())
}

pub(super) async fn advance(
    repo: &PostgresFleetRepository,
    launch: &ContainerLaunch,
    state: &str,
    snapshot: Option<Value>,
    origin: Option<String>,
) -> Result<(), AppError> {
    advance_owned(repo, launch, state, snapshot, origin, None).await
}

pub(super) async fn advance_owned(
    repo: &PostgresFleetRepository,
    launch: &ContainerLaunch,
    state: &str,
    snapshot: Option<Value>,
    origin: Option<String>,
    recovery: Option<&app::container_runtime::ContainerRecoveryCommand>,
) -> Result<(), AppError> {
    let txn = repo.db.begin().await.map_err(|_| held())?;
    lock(&txn, launch.prepared.agent_id).await?;
    if let Some(command) = recovery {
        let current = crate::container_recovery::get_txn(&txn, command.request.launch_id)
            .await?
            .ok_or_else(held)?;
        if current.lease != *command
            || !current.lease_valid
            || current.receipt.is_none()
            || current.lease_receipt.is_none()
            || command.request.original_controller_id != launch.controller_id
        {
            return Err(held());
        }
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT set_config('fleet.container_recovery_id',$1,true)",
            [command.request.id.to_string().into()],
        ))
        .await
        .map_err(|_| held())?;
    }
    if state == "stopping" {
        idle(&txn, launch.prepared.agent_id).await?;
    }
    let result = txn
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE runtime_container_launches SET state=$1,snapshot=$2,origin=$3
         WHERE generation=$4 AND controller_id=$5 AND state=$6 AND prepared=$7 AND stop_id=$8
            AND snapshot IS NOT DISTINCT FROM $9 AND origin IS NOT DISTINCT FROM $10",
            [
                state.into(),
                snapshot.into(),
                origin.into(),
                launch.prepared.container.registration.generation.into(),
                launch.controller_id.into(),
                launch.state.clone().into(),
                serde_json::to_value(&launch.prepared)
                    .map_err(|_| held())?
                    .into(),
                launch.stop_id.into(),
                launch.snapshot.clone().into(),
                launch.origin.clone().into(),
            ],
        ))
        .await
        .map_err(|_| held())?;
    if result.rows_affected() != 1 {
        return Err(held());
    }
    txn.commit().await.map_err(|_| held())
}
