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
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE agent_sessions SET state='active',updated_at=now() WHERE id=$1 AND agent_id=$2 AND state='draft'",
        [record.reservation.session_id.into(),record.reservation.identity.agent_id()?.into()]))
        .await.map_err(AppError::database)?;
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

pub(super) async fn prepare_dispatch(
    repo: &PostgresFleetRepository,
    intent: domain::PmDispatchIntent,
) -> Result<domain::PmDispatchIntent, AppError> {
    if intent.submitted || intent.hermes_run_ref.is_some() {
        return Err(AppError::validation("PM dispatch must start unsubmitted"));
    }
    let record = get(repo, intent.session_run_id).await?;
    record.reservation.validate()?;
    let body: Value = serde_json::from_str(&intent.request_body)
        .map_err(|_| AppError::validation("invalid PM request"))?;
    if intent.request_body.len() > 1_048_576
        || body.as_object().is_none_or(|map| map.len() != 2)
        || body["input"].as_str().is_none_or(str::is_empty)
        || body["session_id"].as_str() != Some(record.reservation.runtime_session_id().as_str())
    {
        return Err(AppError::validation("invalid PM exact request"));
    }
    let frozen = serde_json::to_value(&intent).map_err(AppError::internal)?;
    repo.db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO pm_dispatch_journal(session_run_id,intent) VALUES($1,$2) ON CONFLICT DO NOTHING",
        [intent.session_run_id.into(),frozen.clone().into()])).await.map_err(AppError::database)?;
    let row = repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT intent,submitted,hermes_run_ref FROM pm_dispatch_journal WHERE session_run_id=$1",
        [intent.session_run_id.into()])).await.map_err(AppError::database)?
        .ok_or_else(|| AppError::internal("missing PM dispatch journal"))?;
    if row
        .try_get::<Value>("", "intent")
        .map_err(AppError::database)?
        != frozen
    {
        return Err(AppError::conflict(
            "PM original dispatch request or context changed",
        ));
    }
    Ok(domain::PmDispatchIntent {
        submitted: row.try_get("", "submitted").map_err(AppError::database)?,
        hermes_run_ref: row
            .try_get("", "hermes_run_ref")
            .map_err(AppError::database)?,
        ..intent
    })
}

pub(super) fn dispatch_error(error: AppError) -> AppError {
    match error {
        AppError::Database(_) => {
            AppError::Database("PM dispatch journal database operation failed".into())
        }
        other => other,
    }
}

pub(super) async fn claim_submission(
    repo: &PostgresFleetRepository,
    id: Uuid,
) -> Result<bool, AppError> {
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let record = get(repo, id).await?;
    let agent = record.reservation.identity.agent_id()?;
    // Same agent lock as capacity reservation/configuration drain, before consuming the one-shot permit.
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id=$1 FOR NO KEY UPDATE",
        [agent.into()],
    ))
    .await
    .map_err(AppError::database)?;
    let result = txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE pm_dispatch_journal j SET submitted=true FROM pm_run_bindings b,session_agent_runs r,
            agent_sessions s,users u,task_chat_bindings t,agents a
         WHERE j.session_run_id=$1 AND NOT j.submitted AND b.session_run_id=j.session_run_id
            AND r.id=b.session_run_id AND r.state='pending' AND r.runtime_run_id IS NULL
            AND b.hermes_run_ref IS NULL AND b.terminal_status IS NULL
            AND s.id=b.session_id AND s.agent_id=b.agent_id AND s.state IN ('draft','active')
            AND u.id=s.user_id AND u.is_active AND t.session_id=s.id AND t.agent_id=b.agent_id
            AND t.owner_subject=u.central_sub AND a.id=b.agent_id AND a.kind='hermes'
            AND a.status='running' AND a.sdlc_role='project_manager'
            AND NOT EXISTS(SELECT 1 FROM agent_config_heads WHERE agent_id=b.agent_id AND draining)
            AND NOT EXISTS(SELECT 1 FROM message_dispatch_outbox WHERE agent_id=b.agent_id AND state IN ('dispatching','uncertain'))",
        [id.into()])).await.map_err(AppError::database)?;
    txn.commit().await.map_err(AppError::database)?;
    Ok(result.rows_affected() == 1)
}

pub(super) async fn get_dispatch(
    repo: &PostgresFleetRepository,
    id: Uuid,
) -> Result<Option<domain::PmDispatchIntent>, AppError> {
    let row = repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT intent || jsonb_build_object('submitted',submitted,'hermes_run_ref',hermes_run_ref) AS intent FROM pm_dispatch_journal WHERE session_run_id=$1",
        [id.into()])).await.map_err(AppError::database)?;
    row.map(|row| {
        serde_json::from_value(
            row.try_get::<Value>("", "intent")
                .map_err(AppError::database)?,
        )
        .map_err(|_| AppError::conflict("invalid PM dispatch journal"))
    })
    .transpose()
}

pub(super) async fn record_submission(
    repo: &PostgresFleetRepository,
    id: Uuid,
    run_ref: String,
) -> Result<(), AppError> {
    if !valid_hermes_ref(&run_ref) {
        return Err(AppError::validation("invalid Hermes PM ACK"));
    }
    let result = repo
        .db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE pm_dispatch_journal SET hermes_run_ref=$2 WHERE session_run_id=$1 AND submitted
            AND (hermes_run_ref IS NULL OR hermes_run_ref=$2)",
            [id.into(), run_ref.into()],
        ))
        .await
        .map_err(AppError::database)?;
    if result.rows_affected() != 1 {
        return Err(AppError::conflict(
            "PM ACK is not for the original submitted request",
        ));
    }
    Ok(())
}

pub(super) async fn claim_guidance(
    repo: &PostgresFleetRepository,
    id: Uuid,
    body: String,
) -> Result<domain::PmGuidancePermit, AppError> {
    if body.is_empty() || body.len() > 32768 {
        return Err(AppError::validation("PM guidance exceeds its bound"));
    }
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let record = get(repo, id).await?;
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id=$1 FOR NO KEY UPDATE",
        [record.reservation.identity.agent_id()?.into()],
    ))
    .await
    .map_err(AppError::database)?;
    let result = txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE pm_dispatch_journal j SET guidance_attempted=true,guidance_body=$2
         FROM pm_run_bindings b,session_agent_runs r,agent_sessions s,users u,agents a
         WHERE j.session_run_id=$1 AND NOT j.guidance_attempted
            AND b.session_run_id=j.session_run_id AND b.hermes_run_ref=j.hermes_run_ref
            AND b.terminal_status IS NULL AND r.id=b.session_run_id AND r.state='running'
            AND s.id=b.session_id AND s.agent_id=b.agent_id AND s.state='active'
            AND u.id=s.user_id AND u.is_active
            AND a.id=b.agent_id AND a.kind='hermes' AND a.sdlc_role='project_manager' AND a.status='running'
            AND NOT EXISTS(SELECT 1 FROM agent_config_heads WHERE agent_id=b.agent_id AND draining)",
        [id.into(),body.clone().into()])).await.map_err(AppError::database)?;
    txn.commit().await.map_err(AppError::database)?;
    if result.rows_affected() == 1 {
        return Ok(domain::PmGuidancePermit::Claimed);
    }
    let row = repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT guidance_body,guidance_delivered FROM pm_dispatch_journal WHERE session_run_id=$1", [id.into()]))
        .await.map_err(AppError::database)?.ok_or_else(|| AppError::conflict("missing PM guidance journal"))?;
    let saved: Option<String> = row
        .try_get("", "guidance_body")
        .map_err(AppError::database)?;
    if saved.as_ref().is_some_and(|saved| saved != &body) {
        return Err(AppError::conflict("PM original Workflow guidance changed"));
    }
    if row
        .try_get::<bool>("", "guidance_delivered")
        .map_err(AppError::database)?
    {
        Ok(domain::PmGuidancePermit::Delivered)
    } else {
        Ok(domain::PmGuidancePermit::Unknown)
    }
}

pub(super) async fn finish_guidance(
    repo: &PostgresFleetRepository,
    id: Uuid,
) -> Result<(), AppError> {
    let result = repo.db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE pm_dispatch_journal SET guidance_delivered=true WHERE session_run_id=$1 AND guidance_attempted", [id.into()]))
        .await.map_err(AppError::database)?;
    if result.rows_affected() != 1 {
        return Err(AppError::conflict("missing attempted PM guidance"));
    }
    Ok(())
}
