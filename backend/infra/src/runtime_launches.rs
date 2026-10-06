use crate::PostgresFleetRepository;
use app::{
    RuntimeStatePatch,
    runtime_launch::{RuntimeConfigurationClaim, RuntimeLaunchBinding, RuntimeLaunchRecord},
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

fn valid_endpoint_origin(origin: &str, port: i32) -> bool {
    let Ok(url) = reqwest::Url::parse(origin) else {
        return false;
    };
    let Some(host) = url
        .host_str()
        .and_then(|host| host.parse::<std::net::Ipv4Addr>().ok())
    else {
        return false;
    };
    (host.is_private() || host == std::net::Ipv4Addr::LOCALHOST)
        && (1024..=65535).contains(&port)
        && origin == format!("http://{host}:{port}")
}

pub(super) async fn record_endpoint(
    repo: &PostgresFleetRepository,
    binding: &RuntimeLaunchBinding,
    pid: i32,
    origin: &str,
) -> Result<(), AppError> {
    validate_container_binding(binding)?;
    if binding.container.is_none()
        || pid <= 0
        || !valid_endpoint_origin(origin, binding.api_port.unwrap_or_default())
    {
        return Err(AppError::validation("invalid original container endpoint"));
    }
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id=$1 FOR NO KEY UPDATE",
        [binding.agent_id.into()],
    ))
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::not_found("agent", binding.agent_id))?;
    let record = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT state,pid,binding FROM runtime_launches WHERE id=$1 AND agent_id=$2 FOR UPDATE",
            [binding.id.into(), binding.agent_id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::conflict("original container launch is missing"))?;
    if record
        .try_get::<String>("", "state")
        .map_err(AppError::database)?
        != "gateway_started"
        || record
            .try_get::<Option<i32>>("", "pid")
            .map_err(AppError::database)?
            != Some(pid)
        || record
            .try_get::<serde_json::Value>("", "binding")
            .map_err(AppError::database)?
            != serde_json::to_value(binding).map_err(AppError::internal)?
    {
        return Err(AppError::conflict(
            "original container endpoint custody changed",
        ));
    }
    if let Some(previous) = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT origin,pid FROM runtime_launch_endpoints WHERE launch_id=$1",
            [binding.id.into()],
        ))
        .await
        .map_err(AppError::database)?
    {
        if previous
            .try_get::<String>("", "origin")
            .map_err(AppError::database)?
            != origin
            || previous
                .try_get::<i32>("", "pid")
                .map_err(AppError::database)?
                != pid
        {
            return Err(AppError::conflict(
                "original container endpoint is immutable",
            ));
        }
    } else {
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO runtime_launch_endpoints(launch_id,origin,pid) VALUES($1,$2,$3)",
            [binding.id.into(), origin.into(), pid.into()],
        ))
        .await
        .map_err(AppError::database)?;
    }
    txn.commit().await.map_err(AppError::database)
}

pub(crate) fn valid_agent_container_project(name: &str) -> bool {
    matches!(name, "sdlc1" | "sdlc2")
        || name.strip_prefix("sdlc-qa-").is_some_and(|suffix| {
            suffix
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
                && name.len() <= 128
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

pub(crate) fn validate_container_binding(binding: &RuntimeLaunchBinding) -> Result<(), AppError> {
    let Some(container) = &binding.container else {
        return Ok(());
    };
    let invalid = || AppError::validation("invalid original container launch identity");
    crate::runtime::container_control::validate_registration(&container.registration)?;
    let registration = &container.registration;
    let policy = &container.policy;
    if binding.kind != domain::AgentKind::Hermes
        || !matches!(registration.contract_version, 2 | 3)
        || registration.resource_id != binding.agent_id
        || registration.generation != binding.id
        || policy
            .get("contract_version")
            .and_then(serde_json::Value::as_u64)
            != Some(registration.contract_version.into())
        || policy
            .get("resource_id")
            .and_then(serde_json::Value::as_str)
            != Some(binding.agent_id.to_string().as_str())
        || policy.get("generation").and_then(serde_json::Value::as_str)
            != Some(binding.id.to_string().as_str())
        || !policy
            .get("project")
            .and_then(serde_json::Value::as_str)
            .is_some_and(valid_agent_container_project)
        || container.source_sha256.iter().any(|hash| !valid_hash(hash))
        || container.context.is_empty()
        || container.context.len() > 128
        || !container
            .context
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
        || [
            &container.compose,
            &container.journal,
            &container.stop_journal,
        ]
        .iter()
        .any(|path| !std::path::Path::new(path).is_absolute())
        || container.compose == container.journal
        || container.compose == container.stop_journal
        || container.journal == container.stop_journal
        || snapshot_hash(&serde_json::to_value(container).map_err(AppError::internal)?)?
            != binding.command_sha256
    {
        return Err(invalid());
    }
    let local_policy = match (&container.mount_mapping, &container.mapping_file) {
        (Some(mapping), Some(file)) if registration.contract_version == 3 => {
            let control = crate::runtime::container_control::local_mapping_policy(policy, mapping)?;
            crate::runtime::container_control::validate_mapping_registration(
                policy,
                Some(mapping),
                registration,
            )?;
            let file = std::path::Path::new(file);
            if !file.is_absolute()
                || file.parent() != std::path::Path::new(&container.journal).parent()
                || [
                    &container.compose,
                    &container.journal,
                    &container.stop_journal,
                ]
                .iter()
                .any(|other| std::path::Path::new(other) == file)
                || crate::runtime::container_control::canonical_hash(policy)?
                    != registration.policy_sha256
            {
                return Err(invalid());
            }
            control
        }
        (None, None) if registration.contract_version == 2 => policy.clone(),
        _ => return Err(invalid()),
    };
    let mounts = local_policy
        .get("mounts")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(invalid)?;
    if mounts.len() != 4 {
        return Err(invalid());
    }
    for (destination, source, read_only) in [
        ("/runtime", &binding.paths.runtime, true),
        ("/config", &binding.paths.config, false),
        ("/workspace", &binding.paths.workspace, false),
        ("/logs", &binding.paths.logs, false),
    ] {
        if mounts
            .iter()
            .filter(|mount| {
                mount.get("destination").and_then(serde_json::Value::as_str) == Some(destination)
                    && mount.get("source").and_then(serde_json::Value::as_str)
                        == Some(source.as_str())
                    && mount.get("type").and_then(serde_json::Value::as_str) == Some("bind")
                    && mount.get("read_only").and_then(serde_json::Value::as_bool)
                        == Some(read_only)
            })
            .count()
            != 1
        {
            return Err(invalid());
        }
    }
    Ok(())
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
    validate_container_binding(binding)?;
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
    verify_configuration_claim(
        &txn,
        binding.agent_id,
        &RuntimeConfigurationClaim {
            phase: binding.phase.clone(),
            revision: binding.configuration_revision,
            sha256: binding.configuration_sha256.clone(),
        },
    )
    .await?;
    let original = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT id FROM runtime_launches WHERE agent_id=$1 AND state IN ('claimed','gateway_started')",
        [binding.agent_id.into()])).await.map_err(AppError::database)?;
    if original.is_some() {
        // A matching key is not a second spawn permit, even after a controller crash.
        return Err(AppError::Unavailable(
            "original runtime launch requires reconciliation".into(),
        ));
    }
    let preparation = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT controller_id,generation,operation_id FROM runtime_container_preparations
            WHERE agent_id=$1 AND ordinal=(SELECT count(*) FROM runtime_launches WHERE agent_id=$1)",
        [binding.agent_id.into()])).await.map_err(AppError::database)?;
    if let Some(preparation) = preparation {
        let matches = binding.container.as_ref().is_some_and(|container| {
            preparation.try_get::<Uuid>("", "controller_id").ok() == Some(binding.controller_id)
                && preparation.try_get::<Uuid>("", "generation").ok() == Some(binding.id)
                && preparation.try_get::<Uuid>("", "operation_id").ok()
                    == Some(container.registration.operation_id)
        });
        if !matches {
            return Err(AppError::Unavailable(
                "original container preparation requires reconciliation".into(),
            ));
        }
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

async fn verify_configuration_claim(
    txn: &DatabaseTransaction,
    agent_id: Uuid,
    configuration: &RuntimeConfigurationClaim,
) -> Result<(), AppError> {
    if !matches!(
        configuration.phase.as_str(),
        "regular" | "activation" | "rollback"
    ) || configuration.revision.is_some() != configuration.sha256.is_some()
        || configuration.revision.is_some_and(|revision| revision <= 0)
        || configuration
            .sha256
            .as_deref()
            .is_some_and(|hash| !valid_hash(hash))
    {
        return Err(AppError::validation("invalid runtime configuration claim"));
    }
    let head = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT desired_revision,effective_revision,draining FROM agent_config_heads WHERE agent_id=$1 FOR UPDATE",
        [agent_id.into()])).await.map_err(AppError::database)?;
    let draining = head
        .as_ref()
        .map(|row| row.try_get::<bool>("", "draining"))
        .transpose()
        .map_err(AppError::database)?
        .unwrap_or(false);
    if draining != (configuration.phase != "regular") {
        return Err(AppError::conflict(
            "configuration drain changed before launch",
        ));
    }
    let column = if configuration.phase == "activation" {
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
    if current != configuration.revision
        || (configuration.phase == "activation" && current.is_none())
    {
        return Err(AppError::conflict(
            "configuration revision changed before launch",
        ));
    }
    if let Some(revision) = current {
        let row = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT state,snapshot,claimed_at IS NOT NULL AS claimed FROM agent_config_revisions WHERE agent_id=$1 AND revision=$2",
            [agent_id.into(), revision.into()])).await.map_err(AppError::database)?
            .ok_or_else(|| AppError::conflict("runtime configuration snapshot is missing"))?;
        let expected_state = if configuration.phase == "activation" {
            "activating"
        } else {
            "active"
        };
        if row
            .try_get::<String>("", "state")
            .map_err(AppError::database)?
            != expected_state
            || (configuration.phase == "activation"
                && !row
                    .try_get::<bool>("", "claimed")
                    .map_err(AppError::database)?)
            || Some(snapshot_hash(
                &row.try_get::<serde_json::Value>("", "snapshot")
                    .map_err(AppError::database)?,
            )?) != configuration.sha256
        {
            return Err(AppError::conflict("runtime configuration snapshot changed"));
        }
    }
    Ok(())
}

pub(super) async fn claim_preparation(
    repo: &PostgresFleetRepository,
    preparation: &app::runtime_launch::RuntimeContainerPreparation,
    configuration: &RuntimeConfigurationClaim,
) -> Result<(), AppError> {
    if preparation.agent_id.is_nil()
        || preparation.controller_id.is_nil()
        || preparation.generation.is_nil()
        || preparation.operation_id.is_nil()
        || preparation.ordinal < 0
        || !valid_hash(&preparation.intent_sha256)
    {
        return Err(AppError::validation(
            "invalid container preparation identity",
        ));
    }
    let held =
        || AppError::Unavailable("original container preparation requires reconciliation".into());
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let agent = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT kind,status FROM agents WHERE id=$1 AND archived_at IS NULL FOR UPDATE",
            [preparation.agent_id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(held)?;
    if agent
        .try_get::<String>("", "kind")
        .map_err(AppError::database)?
        != "hermes"
        || matches!(
            agent
                .try_get::<String>("", "status")
                .map_err(AppError::database)?
                .as_str(),
            "running" | "starting" | "degraded" | "archived"
        )
    {
        return Err(held());
    }
    let runtime = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pid,desired_state FROM agent_runtime WHERE agent_id=$1 FOR UPDATE",
            [preparation.agent_id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(held)?;
    if runtime
        .try_get::<Option<i32>>("", "pid")
        .map_err(AppError::database)?
        .is_some()
        || runtime
            .try_get::<String>("", "desired_state")
            .map_err(AppError::database)?
            != "stopped"
    {
        return Err(held());
    }
    verify_configuration_claim(&txn, preparation.agent_id, configuration).await?;
    let history = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT count(*) AS ordinal,
            count(*) FILTER (WHERE state IN ('claimed','gateway_started')) AS outstanding
            FROM runtime_launches WHERE agent_id=$1",
            [preparation.agent_id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(held)?;
    if history
        .try_get::<i64>("", "ordinal")
        .map_err(AppError::database)?
        != preparation.ordinal
        || history
            .try_get::<i64>("", "outstanding")
            .map_err(AppError::database)?
            != 0
    {
        return Err(held());
    }
    let original = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('agent_id',agent_id,'ordinal',ordinal,'controller_id',controller_id,
            'generation',generation,'operation_id',operation_id,'intent_sha256',intent_sha256) AS preparation
            FROM runtime_container_preparations WHERE agent_id=$1 AND ordinal=$2",
        [preparation.agent_id.into(), preparation.ordinal.into()])).await.map_err(AppError::database)?;
    if let Some(original) = original {
        let original: app::runtime_launch::RuntimeContainerPreparation = serde_json::from_value(
            original
                .try_get("", "preparation")
                .map_err(AppError::database)?,
        )
        .map_err(AppError::internal)?;
        if original != *preparation {
            return Err(held());
        }
    } else {
        txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO runtime_container_preparations
                (agent_id,ordinal,controller_id,generation,operation_id,intent_sha256) VALUES($1,$2,$3,$4,$5,$6)",
            [preparation.agent_id.into(), preparation.ordinal.into(), preparation.controller_id.into(),
                preparation.generation.into(), preparation.operation_id.into(), preparation.intent_sha256.clone().into()]))
            .await.map_err(AppError::database)?;
    }
    txn.commit().await.map_err(AppError::database)
}

pub(super) async fn pending_preparation(
    repo: &PostgresFleetRepository,
    agent: Uuid,
) -> Result<bool, AppError> {
    let row = repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM runtime_container_preparations
            WHERE agent_id=$1 AND ordinal=(SELECT count(*) FROM runtime_launches WHERE agent_id=$1)) AS pending",
        [agent.into()])).await.map_err(AppError::database)?.ok_or_else(|| AppError::Unavailable(
            "container preparation journal is unavailable".into()))?;
    row.try_get("", "pending").map_err(AppError::database)
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

pub(super) async fn next_container_ordinal(
    repo: &PostgresFleetRepository,
    agent: Uuid,
) -> Result<i64, AppError> {
    // History cannot be deleted. One read snapshot avoids clock-dependent ordering
    // and keeps unknown preparation on the same key until a launch is claimed.
    let row = repo
        .db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT count(*) AS ordinal,
                count(*) FILTER (WHERE state IN ('claimed','gateway_started')) AS outstanding
                FROM runtime_launches WHERE agent_id=$1",
            [agent.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::Unavailable("runtime launch history is unavailable".into()))?;
    if row
        .try_get::<i64>("", "outstanding")
        .map_err(AppError::database)?
        != 0
    {
        return Err(AppError::Unavailable(
            "original runtime launch requires reconciliation".into(),
        ));
    }
    row.try_get("", "ordinal").map_err(AppError::database)
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
        let changed = txn
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE agent_runtime SET pid=$2,desired_state=$3,health_status=$4,
                health_detail=$5,
                last_capabilities_json='{}'::jsonb,last_health_at=now(),
                stopped_at=CASE WHEN $3='stopped' THEN now() ELSE NULL END
                WHERE agent_id=$1",
                [
                    binding.agent_id.into(),
                    if state == "gateway_started" {
                        pid
                    } else {
                        None
                    }
                    .into(),
                    if state == "gateway_started" {
                        "running"
                    } else {
                        "stopped"
                    }
                    .into(),
                    state.into(),
                    if binding.container.is_some() {
                        if state == "gateway_exited" {
                            "Original container namespace exit confirmed"
                        } else {
                            "Original container launch acknowledgement"
                        }
                    } else {
                        "Original native gateway observation; boundary quiescence is not attested"
                    }
                    .into(),
                ],
            ))
            .await
            .map_err(AppError::database)?;
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

#[cfg(test)]
mod endpoint_tests {
    use super::valid_endpoint_origin;

    #[test]
    fn endpoint_shape_is_canonical_private_ipv4_and_exact_port() {
        for host in ["10.0.0.2", "172.18.0.2", "192.168.1.2", "127.0.0.1"] {
            assert!(valid_endpoint_origin(
                &format!("http://{host}:29100"),
                29100
            ));
        }
        for origin in [
            "http://8.8.8.8:29100",
            "http://0.0.0.0:29100",
            "http://169.254.169.254:29100",
            "http://172.18.0.2:29101",
            "https://172.18.0.2:29100",
            "http://172.18.0.2:29100/",
            "http://172.18.0.2:29100/?x=1",
            "http://user@172.18.0.2:29100",
            "http://localhost:29100",
            "http://[::1]:29100",
            "http://2130706433:29100",
            "http://172.18.0.2:029100",
        ] {
            assert!(!valid_endpoint_origin(origin, 29100), "{origin}");
        }
        assert!(!valid_endpoint_origin("http://10.0.0.2:100", 100));
    }
}
