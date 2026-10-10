use super::*;
use domain::{PmRunRecord, PmRunReservation, PmRuntimeStatus};

pub(super) async fn locked<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
) -> Result<Option<PmRunRecord>, AppError> {
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('reservation',reservation,'hermes_run_ref',hermes_run_ref,'hermes_session_ref',hermes_session_ref,'terminal_status',terminal_status) AS record FROM pm_run_bindings WHERE session_run_id=$1 FOR UPDATE",
        [id.into()])).await.map_err(AppError::database)?;
    row.map(|row| {
        let value: Value = row.try_get("", "record").map_err(AppError::database)?;
        serde_json::from_value(value).map_err(AppError::internal)
    })
    .transpose()
}

pub(super) fn visible_terminal(status: PmRuntimeStatus) -> &'static str {
    match status {
        PmRuntimeStatus::Completed => "completed",
        PmRuntimeStatus::Failed => "failed",
        PmRuntimeStatus::Cancelled | PmRuntimeStatus::Stopped => "cancelled",
        PmRuntimeStatus::Running => "running",
    }
}

async fn load<C: ConnectionTrait>(db: &C, id: Uuid, lock: bool) -> Result<PmRunRecord, AppError> {
    if lock {
        return locked(db, id)
            .await?
            .ok_or_else(|| AppError::not_found("pm_run", id));
    }
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT jsonb_build_object('reservation',reservation,'hermes_run_ref',hermes_run_ref,'hermes_session_ref',hermes_session_ref,'terminal_status',terminal_status) AS record FROM pm_run_bindings WHERE session_run_id=$1",
            [id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("pm_run", id))?;
    let value: Value = row.try_get("", "record").map_err(AppError::database)?;
    serde_json::from_value(value).map_err(AppError::internal)
}

pub(super) async fn get(repo: &PostgresFleetRepository, id: Uuid) -> Result<PmRunRecord, AppError> {
    load(&repo.db, id, false).await
}

pub(super) async fn reserve(
    repo: &PostgresFleetRepository,
    req: PmRunReservation,
) -> Result<PmRunRecord, AppError> {
    req.validate()?;
    let runtime_binding = req
        .runtime_binding
        .as_ref()
        .ok_or_else(|| AppError::Unavailable("PM original runtime binding is required".into()))?;
    let agent_id = req.identity.agent_id()?;
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let agent = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT kind,status,sdlc_role FROM agents WHERE id=$1 FOR NO KEY UPDATE",
            [agent_id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent", agent_id))?;

    // Agent lock serializes both the idempotency lookup and capacity reservation.
    if let Some(row) = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT session_run_id FROM pm_run_bindings WHERE agent_id=$1 AND dispatch_operation_key=$2",
        [agent_id.into(), req.dispatch_operation_key.clone().into()])).await.map_err(AppError::database)? {
        let id: Uuid = row.try_get("", "session_run_id").map_err(AppError::database)?;
        let previous = load(&txn, id, false).await?;
        if previous.reservation != req {
            return Err(AppError::conflict("PM dispatch key has a different payload"));
        }
        return Ok(previous);
    }
    let kind: String = agent.try_get("", "kind").map_err(AppError::database)?;
    let status: String = agent.try_get("", "status").map_err(AppError::database)?;
    let role: Option<String> = agent.try_get("", "sdlc_role").map_err(AppError::database)?;
    if kind != "hermes" || status != "running" || role.as_deref() != Some("project_manager") {
        return Err(AppError::conflict(
            "a running Hermes Project Manager is required",
        ));
    }
    verify_runtime_binding(&txn, agent_id, runtime_binding).await?;
    let binding = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT b.tracker_instance_id,b.project_id,b.task_id,b.root_task_id,b.agent_id,b.owner_subject,u.central_sub
         FROM task_chat_bindings b JOIN agent_sessions s ON s.id=b.session_id JOIN users u ON u.id=s.user_id
         WHERE b.session_id=$1 FOR UPDATE OF s", [req.session_id.into()])).await.map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("task_chat_binding", req.session_id))?;
    let owner: String = binding
        .try_get("", "owner_subject")
        .map_err(AppError::database)?;
    let central: Option<String> = binding
        .try_get("", "central_sub")
        .map_err(AppError::database)?;
    if binding
        .try_get::<String>("", "tracker_instance_id")
        .map_err(AppError::database)?
        != req.identity.tracker_instance_ref
        || binding
            .try_get::<Uuid>("", "project_id")
            .map_err(AppError::database)?
            .to_string()
            != req.identity.tracker_project_ref
        || binding
            .try_get::<Uuid>("", "task_id")
            .map_err(AppError::database)?
            .to_string()
            != req.identity.task_ref
        || binding
            .try_get::<Uuid>("", "root_task_id")
            .map_err(AppError::database)?
            .to_string()
            != req.identity.root_ref
        || binding
            .try_get::<Uuid>("", "agent_id")
            .map_err(AppError::database)?
            != agent_id
        || central.as_deref() != Some(owner.as_str())
    {
        return Err(AppError::Forbidden);
    }
    let busy = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM session_agent_runs WHERE agent_id=$1 AND id<>$2
            AND state IN ('pending','running','waiting','stopping') AND runtime_session_id IS NOT NULL)
         OR EXISTS(SELECT 1 FROM agent_config_heads WHERE agent_id=$1 AND draining)
         OR EXISTS(SELECT 1 FROM message_dispatch_outbox o WHERE agent_id=$1 AND state IN ('dispatching','uncertain')
            AND NOT EXISTS(SELECT 1 FROM hermes_dispatch_journal j WHERE j.message_id=o.message_id AND j.run_id=$2)) AS busy",
            [agent_id.into(),req.session_run_id.into()]))
        .await.map_err(AppError::database)?.ok_or_else(|| AppError::internal("missing PM capacity check"))?;
    if busy
        .try_get::<bool>("", "busy")
        .map_err(AppError::database)?
    {
        return Err(AppError::conflict(
            "PM is draining or has an active/unresolved run",
        ));
    }
    let existing = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT session_id,agent_id,runtime_session_id,state FROM session_agent_runs WHERE id=$1 FOR UPDATE",
        [req.session_run_id.into()])).await.map_err(AppError::database)?;
    if let Some(existing) = existing {
        if existing
            .try_get::<Uuid>("", "session_id")
            .map_err(AppError::database)?
            != req.session_id
            || existing
                .try_get::<Uuid>("", "agent_id")
                .map_err(AppError::database)?
                != agent_id
            || existing
                .try_get::<Option<String>>("", "runtime_session_id")
                .map_err(AppError::database)?
                .as_deref()
                != Some(req.runtime_session_id().as_str())
            || existing
                .try_get::<String>("", "state")
                .map_err(AppError::database)?
                != "pending"
        {
            return Err(AppError::conflict(
                "PM reservation differs from the prepared native journal run",
            ));
        }
    } else {
        txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO session_agent_runs(id,session_id,agent_id,runtime_session_id,run_role,state,model_options,created_at,updated_at)
            VALUES($1,$2,$3,$4,'primary','pending','{}'::jsonb,now(),now())",
        [req.session_run_id.into(), req.session_id.into(), agent_id.into(), req.runtime_session_id().into()]))
        .await.map_err(AppError::database)?;
    }
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO pm_run_bindings(session_run_id,session_id,agent_id,reservation,dispatch_operation_key,runtime_session_id)
            VALUES($1,$2,$3,$4,$5,$6)",
        [req.session_run_id.into(), req.session_id.into(), agent_id.into(), serde_json::to_value(&req).map_err(AppError::internal)?.into(),
            req.dispatch_operation_key.clone().into(), req.runtime_session_id().into()])).await.map_err(AppError::database)?;
    let result = load(&txn, req.session_run_id, false).await?;
    audit(
        &txn,
        req.session_run_id,
        "pm.run.reserved",
        json!({"identity":req.identity,"session_id":req.session_id,"fence":req.fence}),
    )
    .await?;
    txn.commit().await.map_err(AppError::database)?;
    Ok(result)
}

pub(super) async fn accept(
    repo: &PostgresFleetRepository,
    id: Uuid,
    hermes: String,
    hermes_session: String,
) -> Result<PmRunRecord, AppError> {
    if !valid_hermes_ref(&hermes) || !domain::valid_ref(&hermes_session, 512) {
        return Err(AppError::validation("invalid Hermes run reference"));
    }
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let record = load(&txn, id, true).await?;
    if record.hermes_run_ref.as_ref().is_some_and(|v| v != &hermes)
        || record
            .hermes_session_ref
            .as_ref()
            .is_some_and(|v| v != &hermes_session)
    {
        return Err(AppError::conflict("PM runtime mapping is immutable"));
    }
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE pm_run_bindings SET hermes_run_ref=$2,hermes_session_ref=$3 WHERE session_run_id=$1",
        [id.into(), hermes.clone().into(), hermes_session.into()],
    ))
    .await
    .map_err(AppError::database)?;
    let changed = txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE session_agent_runs SET runtime_run_id=$2,state=CASE WHEN state='pending' THEN 'running' ELSE state END,updated_at=now()
         WHERE id=$1 AND session_id=$3 AND agent_id=$4 AND runtime_session_id=$5 AND (runtime_run_id IS NULL OR runtime_run_id=$2)",
        [id.into(), hermes.into(), record.reservation.session_id.into(), record.reservation.identity.agent_id()?.into(),
            record.reservation.runtime_session_id().into()])).await.map_err(AppError::database)?;
    if changed.rows_affected() != 1 {
        return Err(AppError::conflict(
            "PM runtime run no longer matches reservation",
        ));
    }
    let result = load(&txn, id, false).await?;
    if record.hermes_run_ref.is_none() {
        audit(&txn, id, "pm.run.accepted", json!({"session_id":record.reservation.session_id,"agent_id":record.reservation.identity.agent_ref,"runtime_run_id":result.hermes_run_ref})).await?;
    }
    txn.commit().await.map_err(AppError::database)?;
    Ok(result)
}

pub(super) fn valid_hermes_ref(value: &str) -> bool {
    domain::valid_ref(value, 512)
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

pub(super) async fn observe(
    repo: &PostgresFleetRepository,
    expected: &PmRunRecord,
    status: PmRuntimeStatus,
    custody: &dyn app::PmRuntimeCustody,
) -> Result<(), AppError> {
    expected.reservation.validate()?;
    let id = expected.reservation.session_run_id;
    let agent_id = expected.reservation.identity.agent_id()?;
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id=$1 FOR NO KEY UPDATE",
        [agent_id.into()],
    ))
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::not_found("agent", agent_id))?;
    let record = load(&txn, id, true).await?;
    if record.reservation != expected.reservation
        || record.hermes_run_ref != expected.hermes_run_ref
        || record.hermes_session_ref != expected.hermes_session_ref
    {
        return Err(AppError::conflict("PM observation context changed"));
    }
    let binding =
        record.reservation.runtime_binding.as_ref().ok_or_else(|| {
            AppError::Unavailable("PM original runtime binding is missing".into())
        })?;
    let (launch, pid) = verify_runtime_binding(&txn, agent_id, binding).await?;
    if record.hermes_run_ref.is_none() {
        return Err(AppError::Unavailable(
            "PM runtime acceptance is unknown".into(),
        ));
    }
    if record
        .terminal_status
        .is_some_and(|previous| previous != status)
    {
        return Err(AppError::conflict(
            "Hermes contradicted immutable PM terminal proof",
        ));
    }
    let visible = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT session_id,agent_id,runtime_session_id,runtime_run_id,state FROM session_agent_runs WHERE id=$1 FOR UPDATE",
        [id.into()])).await.map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("session run", id))?;
    if visible
        .try_get::<Uuid>("", "session_id")
        .map_err(AppError::database)?
        != record.reservation.session_id
        || visible
            .try_get::<Uuid>("", "agent_id")
            .map_err(AppError::database)?
            != agent_id
        || visible
            .try_get::<Option<String>>("", "runtime_session_id")
            .map_err(AppError::database)?
            .as_deref()
            != Some(record.reservation.runtime_session_id().as_str())
        || visible
            .try_get::<Option<String>>("", "runtime_run_id")
            .map_err(AppError::database)?
            != record.hermes_run_ref
    {
        return Err(AppError::conflict(
            "PM terminal proof no longer matches runtime mapping",
        ));
    }
    // A retained process can exit while row locks are awaited, independently of lifecycle exclusion.
    custody.verify(binding, &launch, pid).await?;
    if record.terminal_status.is_some() {
        if visible
            .try_get::<String>("", "state")
            .map_err(AppError::database)?
            != visible_terminal(status)
        {
            return Err(AppError::conflict("PM visible terminal state changed"));
        }
        txn.commit().await.map_err(AppError::database)?;
        return Ok(());
    }
    let terminal = status
        .terminal()
        .then(|| serde_json::to_value(status))
        .transpose()
        .map_err(AppError::internal)?
        .and_then(|value| value.as_str().map(str::to_owned));
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE pm_run_bindings SET terminal_status=$2,observed_at=now() WHERE session_run_id=$1",
        [id.into(), terminal.into()],
    ))
    .await
    .map_err(AppError::database)?;
    if status.terminal() {
        let changed = txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "UPDATE session_agent_runs SET state=$2,updated_at=now(),last_error=CASE WHEN $2='completed' THEN NULL ELSE last_error END
             WHERE id=$1 AND session_id=$3 AND agent_id=$4 AND runtime_session_id=$5 AND runtime_run_id=$6",
            [id.into(), visible_terminal(status).into(), record.reservation.session_id.into(),
                record.reservation.identity.agent_id()?.into(), record.reservation.runtime_session_id().into(), record.hermes_run_ref.clone().into()]
        )).await.map_err(AppError::database)?;
        if changed.rows_affected() != 1 {
            return Err(AppError::conflict(
                "PM terminal proof no longer matches runtime mapping",
            ));
        }
    }
    if status.terminal() && record.terminal_status.is_none() {
        audit(
            &txn,
            id,
            "pm.run.terminal_verified",
            json!({"session_id":record.reservation.session_id,"status":status}),
        )
        .await?;
    }
    txn.commit().await.map_err(AppError::database)?;
    Ok(())
}

async fn verify_runtime_binding<C: ConnectionTrait>(
    db: &C,
    agent: Uuid,
    expected: &domain::PmRuntimeBinding,
) -> Result<(app::runtime_launch::RuntimeLaunchBinding, i32), AppError> {
    expected.validate()?;
    let unavailable = || AppError::Unavailable("PM original runtime custody changed".into());
    // Caller holds the agent row, the same exclusion used by lifecycle and recovery.
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT l.binding,l.state,l.pid,r.pid AS runtime_pid,r.desired_state,a.api_port,
            a.archived_at IS NOT NULL AS archived,a.kind,a.sdlc_role,a.runtime_path,a.config_path,a.workspace_path,a.logs_path,
            e.origin,e.pid AS endpoint_pid,h.effective_revision,
            EXISTS(SELECT 1 FROM runtime_controller_recoveries c WHERE c.launch_id=l.id) AS recovery
         FROM runtime_launches l JOIN agent_runtime r ON r.agent_id=l.agent_id
         JOIN agents a ON a.id=l.agent_id LEFT JOIN runtime_launch_endpoints e ON e.launch_id=l.id
         LEFT JOIN agent_config_heads h ON h.agent_id=l.agent_id
         WHERE l.id=$1 AND l.agent_id=$2 FOR UPDATE OF l,r",
        [expected.launch_id.into(),agent.into()])).await.map_err(AppError::database)?
        .ok_or_else(unavailable)?;
    let binding: app::runtime_launch::RuntimeLaunchBinding = serde_json::from_value(
        row.try_get::<Value>("", "binding")
            .map_err(AppError::database)?,
    )
    .map_err(|_| unavailable())?;
    let pid: Option<i32> = row.try_get("", "pid").map_err(AppError::database)?;
    let origin: Option<String> = row.try_get("", "origin").map_err(AppError::database)?;
    let port: Option<i32> = row.try_get("", "api_port").map_err(AppError::database)?;
    let matches_origin = if binding.container.is_some() {
        origin.as_deref() == Some(expected.origin.as_str())
            && row
                .try_get::<Option<i32>>("", "endpoint_pid")
                .map_err(AppError::database)?
                == pid
    } else {
        port.is_some_and(|port| {
            (1024..=65535).contains(&port) && expected.origin == format!("http://127.0.0.1:{port}")
        }) && origin.is_none()
    };
    if binding.id != expected.launch_id
        || binding.agent_id != agent
        || binding.controller_id != expected.controller_id
        || binding.kind != AgentKind::Hermes
        || row
            .try_get::<String>("", "kind")
            .map_err(AppError::database)?
            != "hermes"
        || row
            .try_get::<Option<String>>("", "sdlc_role")
            .map_err(AppError::database)?
            .as_deref()
            != Some("project_manager")
        || binding.api_port != port
        || !matches_origin
        || pid.is_none_or(|pid| pid <= 0)
        || row
            .try_get::<Option<i32>>("", "runtime_pid")
            .map_err(AppError::database)?
            != pid
        || row
            .try_get::<String>("", "state")
            .map_err(AppError::database)?
            != "gateway_started"
        || row
            .try_get::<String>("", "desired_state")
            .map_err(AppError::database)?
            != "running"
        || row
            .try_get::<bool>("", "recovery")
            .map_err(AppError::database)?
        || row
            .try_get::<bool>("", "archived")
            .map_err(AppError::database)?
        || row
            .try_get::<Option<i64>>("", "effective_revision")
            .map_err(AppError::database)?
            != binding.configuration_revision
    {
        return Err(unavailable());
    }
    for (column, path) in [
        ("runtime_path", &binding.paths.runtime),
        ("config_path", &binding.paths.config),
        ("workspace_path", &binding.paths.workspace),
        ("logs_path", &binding.paths.logs),
    ] {
        if row
            .try_get::<String>("", column)
            .map_err(AppError::database)?
            != *path
        {
            return Err(unavailable());
        }
    }
    Ok((binding, pid.ok_or_else(unavailable)?))
}

async fn audit<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
    action: &str,
    payload: Value,
) -> Result<(), AppError> {
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO audit_log(id,actor_user_id,action,entity_type,entity_id,payload,created_at) VALUES($1,NULL,$2,'session_run',$3,$4,now())",
        [Uuid::new_v4().into(), action.into(), id.to_string().into(), payload.into()])).await.map_err(AppError::database)?;
    Ok(())
}
