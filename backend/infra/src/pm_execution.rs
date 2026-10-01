use super::*;
use domain::{PmRunRecord, PmRunReservation, PmRuntimeStatus};

async fn load<C: ConnectionTrait>(db: &C, id: Uuid, lock: bool) -> Result<PmRunRecord, AppError> {
    let sql = if lock {
        "SELECT jsonb_build_object('reservation',reservation,'hermes_run_ref',hermes_run_ref,'hermes_session_ref',hermes_session_ref,'terminal_status',terminal_status) AS record FROM pm_run_bindings WHERE session_run_id=$1 FOR UPDATE"
    } else {
        "SELECT jsonb_build_object('reservation',reservation,'hermes_run_ref',hermes_run_ref,'hermes_session_ref',hermes_session_ref,'terminal_status',terminal_status) AS record FROM pm_run_bindings WHERE session_run_id=$1"
    };
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
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
        "SELECT EXISTS(SELECT 1 FROM session_agent_runs WHERE agent_id=$1
            AND state IN ('pending','running','waiting','stopping') AND runtime_session_id IS NOT NULL)
         OR EXISTS(SELECT 1 FROM agent_config_heads WHERE agent_id=$1 AND draining)
         OR EXISTS(SELECT 1 FROM message_dispatch_outbox WHERE agent_id=$1 AND state IN ('dispatching','uncertain')) AS busy", [agent_id.into()]))
        .await.map_err(AppError::database)?.ok_or_else(|| AppError::internal("missing PM capacity check"))?;
    if busy
        .try_get::<bool>("", "busy")
        .map_err(AppError::database)?
    {
        return Err(AppError::conflict(
            "PM is draining or has an active/unresolved run",
        ));
    }
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO session_agent_runs(id,session_id,agent_id,runtime_session_id,run_role,state,model_options,created_at,updated_at)
            VALUES($1,$2,$3,$4,'primary','pending','{}'::jsonb,now(),now())",
        [req.session_run_id.into(), req.session_id.into(), agent_id.into(), req.runtime_session_id().into()]))
        .await.map_err(AppError::database)?;
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
    id: Uuid,
    status: PmRuntimeStatus,
) -> Result<(), AppError> {
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let record = load(&txn, id, true).await?;
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
