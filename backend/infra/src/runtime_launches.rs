use crate::PostgresFleetRepository;
use app::{
    RuntimeStatePatch,
    runtime_launch::{RuntimeLaunchBinding, RuntimeLaunchRecord},
};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseTransaction, Statement, TransactionTrait};
use sha2::{Digest, Sha256};
use shared::AppError;
use uuid::Uuid;

pub(crate) fn snapshot_hash(value: &serde_json::Value) -> Result<String, AppError> {
    Ok(hex::encode(Sha256::digest(
        serde_json::to_vec(value).map_err(AppError::internal)?,
    )))
}

pub(crate) fn dispatch_launch_id(caps: &serde_json::Value) -> Result<Option<Uuid>, AppError> {
    let Some(binding) = caps.get("fleet_launch") else {
        // Historical unjournaled free-chat intent, never managed-launch authority.
        return Ok(None);
    };
    let invalid = || AppError::Unavailable("original dispatch launch binding is malformed".into());
    let object = binding.as_object().ok_or_else(invalid)?;
    if object.len() != 2 || object.get("version").and_then(serde_json::Value::as_u64) != Some(1) {
        return Err(invalid());
    }
    match object.get("launch_id") {
        Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(value)) => {
            let id = Uuid::parse_str(value).map_err(|_| invalid())?;
            if id.is_nil() || id.to_string() != *value {
                return Err(invalid());
            }
            Ok(Some(id))
        }
        _ => Err(invalid()),
    }
}

pub(super) async fn claim(
    repo: &PostgresFleetRepository,
    binding: &RuntimeLaunchBinding,
) -> Result<(), AppError> {
    if binding.id.is_nil()
        || binding.agent_id.is_nil()
        || binding.controller_id.is_nil()
        || binding
            .api_port
            .is_some_and(|port| !(1..=65535).contains(&port))
        || !matches!(
            binding.phase.as_str(),
            "regular" | "activation" | "rollback"
        )
        || binding.configuration_revision.is_some_and(|rev| rev < 1)
        || binding.configuration_revision.is_some() != binding.configuration_sha256.is_some()
        || !valid_hash(&binding.command_sha256)
        || binding
            .configuration_sha256
            .as_deref()
            .is_some_and(|hash| !valid_hash(hash))
    {
        return Err(AppError::validation("invalid runtime launch identity"));
    }
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let agent = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT kind, status, runtime_path, config_path, workspace_path, logs_path, api_port
            FROM agents WHERE id=$1 AND archived_at IS NULL FOR UPDATE",
            [binding.agent_id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent", binding.agent_id))?;
    let kind = binding.kind.to_string();
    if matches!(
        agent
            .try_get::<String>("", "status")
            .map_err(AppError::database)?
            .as_str(),
        "running" | "starting" | "degraded" | "archived"
    ) {
        return Err(AppError::Unavailable(
            "runtime ownership changed before launch".into(),
        ));
    }
    let runtime = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pid,desired_state FROM agent_runtime WHERE agent_id=$1 FOR UPDATE",
            [binding.agent_id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::Unavailable("runtime ownership is missing".into()))?;
    if runtime
        .try_get::<Option<i32>>("", "pid")
        .map_err(AppError::database)?
        .is_some()
        || runtime
            .try_get::<String>("", "desired_state")
            .map_err(AppError::database)?
            != "stopped"
    {
        return Err(AppError::Unavailable(
            "runtime ownership changed before launch".into(),
        ));
    }
    for (column, expected) in [
        ("kind", kind.as_str()),
        ("runtime_path", binding.paths.runtime.as_str()),
        ("config_path", binding.paths.config.as_str()),
        ("workspace_path", binding.paths.workspace.as_str()),
        ("logs_path", binding.paths.logs.as_str()),
    ] {
        if agent
            .try_get::<String>("", column)
            .map_err(AppError::database)?
            != expected
        {
            return Err(AppError::conflict("agent launch binding changed"));
        }
    }
    if agent
        .try_get::<Option<i32>>("", "api_port")
        .map_err(AppError::database)?
        != binding.api_port
    {
        return Err(AppError::conflict("agent launch port changed"));
    }
    let head = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT desired_revision,effective_revision,draining FROM agent_config_heads WHERE agent_id=$1 FOR UPDATE",
        [binding.agent_id.into()])).await.map_err(AppError::database)?;
    let draining = head
        .as_ref()
        .map(|row| row.try_get::<bool>("", "draining"))
        .transpose()
        .map_err(AppError::database)?
        .unwrap_or(false);
    if draining != (binding.phase != "regular") {
        return Err(AppError::conflict(
            "configuration drain changed before launch",
        ));
    }
    let column = if binding.phase == "activation" {
        "desired_revision"
    } else {
        "effective_revision"
    };
    let current = head
        .as_ref()
        .map(|row| row.try_get::<Option<i64>>("", column))
        .transpose()
        .map_err(AppError::database)?
        .flatten();
    if current != binding.configuration_revision
        || (binding.phase == "activation" && current.is_none())
    {
        return Err(AppError::conflict(
            "configuration revision changed before launch",
        ));
    }
    if let Some(revision) = current {
        let row = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT state,snapshot,claimed_at IS NOT NULL AS claimed FROM agent_config_revisions WHERE agent_id=$1 AND revision=$2",
            [binding.agent_id.into(), revision.into()])).await.map_err(AppError::database)?
            .ok_or_else(|| AppError::conflict("runtime configuration snapshot is missing"))?;
        let expected_state = if binding.phase == "activation" {
            "activating"
        } else {
            "active"
        };
        if row
            .try_get::<String>("", "state")
            .map_err(AppError::database)?
            != expected_state
            || (binding.phase == "activation"
                && !row
                    .try_get::<bool>("", "claimed")
                    .map_err(AppError::database)?)
            || Some(snapshot_hash(
                &row.try_get::<serde_json::Value>("", "snapshot")
                    .map_err(AppError::database)?,
            )?) != binding.configuration_sha256
        {
            return Err(AppError::conflict("runtime configuration snapshot changed"));
        }
    }
    let original = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT id FROM runtime_launches WHERE agent_id=$1 AND state IN ('claimed','gateway_started')",
        [binding.agent_id.into()])).await.map_err(AppError::database)?;
    if original.is_some() {
        // A matching key is not a second spawn permit, even after a controller crash.
        return Err(AppError::Unavailable(
            "original runtime launch requires reconciliation".into(),
        ));
    }
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO runtime_launches(id,agent_id,controller_id,binding) VALUES ($1,$2,$3,$4)",
        [
            binding.id.into(),
            binding.agent_id.into(),
            binding.controller_id.into(),
            serde_json::to_value(binding)
                .map_err(AppError::internal)?
                .into(),
        ],
    ))
    .await
    .map_err(AppError::database)?;
    txn.commit().await.map_err(AppError::database)
}

fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) async fn open(
    repo: &PostgresFleetRepository,
    agent: Uuid,
) -> Result<Option<RuntimeLaunchRecord>, AppError> {
    repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT binding,state,pid FROM runtime_launches WHERE agent_id=$1 AND state IN ('claimed','gateway_started')",
        [agent.into()])).await.map_err(AppError::database)?.map(|row| Ok(RuntimeLaunchRecord {
            binding: serde_json::from_value(row.try_get("", "binding").map_err(AppError::database)?)
                .map_err(AppError::internal)?,
            state: row.try_get("", "state").map_err(AppError::database)?,
            pid: row.try_get("", "pid").map_err(AppError::database)?,
        })).transpose()
}

pub(super) async fn guard_runtime_patch(
    txn: &DatabaseTransaction,
    agent: Uuid,
    patch: &RuntimeStatePatch,
) -> Result<(), AppError> {
    let Some(original) = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT state,pid FROM runtime_launches WHERE agent_id=$1 AND state IN ('claimed','gateway_started')",
        [agent.into()])).await.map_err(AppError::database)? else {
        return Ok(());
    };
    let state: String = original.try_get("", "state").map_err(AppError::database)?;
    let pid: Option<i32> = original.try_get("", "pid").map_err(AppError::database)?;
    let status = patch.status.as_str();
    let running = patch.desired_state.as_str() == "running";
    let valid = patch.pid == pid
        && match state.as_str() {
            "claimed" => (status == "starting" && running) || status == "degraded",
            "gateway_started" => running && matches!(status, "starting" | "running" | "degraded"),
            _ => false,
        };
    if !valid {
        return Err(AppError::Unavailable(
            "runtime metadata conflicts with the outstanding original launch".into(),
        ));
    }
    Ok(())
}

pub(super) async fn observe(
    repo: &PostgresFleetRepository,
    binding: &RuntimeLaunchBinding,
    state: &str,
    pid: Option<i32>,
) -> Result<(), AppError> {
    if !matches!(state, "gateway_started" | "gateway_exited" | "spawn_failed")
        || matches!(state, "gateway_started" | "gateway_exited") != pid.is_some()
        || pid.is_some_and(|pid| pid <= 0)
    {
        return Err(AppError::validation("invalid gateway observation"));
    }
    let value = serde_json::to_value(binding).map_err(AppError::internal)?;
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id=$1 FOR UPDATE",
        [binding.agent_id.into()],
    ))
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::not_found("agent", binding.agent_id))?;
    let changed = txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_launches SET state=$2,pid=$3,observed_at=now()
            WHERE id=$1 AND binding=$4 AND (state='claimed' OR (state='gateway_started' AND $2='gateway_exited' AND pid=$3))",
        [binding.id.into(), state.into(), pid.into(), value.clone().into()]))
        .await.map_err(AppError::database)?;
    if changed.rows_affected() == 1 {
        let status = match state {
            "gateway_started" => "starting",
            "gateway_exited" => "stopped",
            _ => "failed",
        };
        txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "UPDATE agents SET status=CASE WHEN archived_at IS NULL THEN $2 ELSE status END WHERE id=$1", [binding.agent_id.into(), status.into()]))
            .await.map_err(AppError::database)?;
        let changed = txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "UPDATE agent_runtime SET pid=$2,desired_state=$3,health_status=$4,
                health_detail='Original native gateway observation; boundary quiescence is not attested',
                last_capabilities_json='{}'::jsonb,last_health_at=now(),
                stopped_at=CASE WHEN $3='stopped' THEN now() ELSE NULL END
                WHERE agent_id=$1", [binding.agent_id.into(), if state=="gateway_started" { pid } else { None }.into(),
                if state=="gateway_started" { "running" } else { "stopped" }.into(), state.into()]))
            .await.map_err(AppError::database)?;
        if changed.rows_affected() != 1 {
            return Err(AppError::Unavailable(
                "original runtime metadata is missing".into(),
            ));
        }
        return txn.commit().await.map_err(AppError::database);
    }
    let replay = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT id FROM runtime_launches WHERE id=$1 AND binding=$4 AND state=$2 AND pid IS NOT DISTINCT FROM $3",
        [binding.id.into(), state.into(), pid.into(), value.into()]))
        .await.map_err(AppError::database)?;
    if replay.is_some() {
        // An old observation replay must not overwrite a newer launch's metadata.
        return txn.commit().await.map_err(AppError::database);
    }
    Err(AppError::conflict("original gateway observation changed"))
}
