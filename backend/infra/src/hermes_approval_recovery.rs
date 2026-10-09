use super::*;

pub(super) async fn context(
    repo: &PostgresFleetRepository,
    run_id: Uuid,
) -> Result<Option<app::HermesDispatchIntent>, AppError> {
    let row = repo.db.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT j.run_id FROM hermes_dispatch_journal j
         JOIN session_agent_runs r ON r.id=j.run_id AND r.agent_id=j.agent_id AND r.session_id=j.session_id
         JOIN agent_sessions s ON s.id=r.session_id AND s.agent_id=r.agent_id
         JOIN agents a ON a.id=r.agent_id
         JOIN session_messages m ON m.id=j.message_id AND m.session_id=r.session_id
         JOIN message_dispatch_outbox o ON o.message_id=m.id AND o.agent_id=r.agent_id
         WHERE j.run_id=$1 AND j.state='accepted' AND a.kind='hermes' AND a.archived_at IS NULL
           AND a.api_port BETWEEN 1024 AND 65535 AND j.origin='http://127.0.0.1:' || a.api_port::text
           AND r.state IN ('pending','running','waiting','stopping') AND r.run_role='primary'
           AND r.runtime_run_id IS NOT NULL AND r.runtime_session_id IS NOT NULL
           AND m.runtime_message_id=r.runtime_run_id AND m.delivery_state='dispatched' AND o.state='dispatched'
           AND NOT EXISTS(SELECT 1 FROM task_chat_bindings b WHERE b.session_id=r.session_id)
           AND NOT EXISTS(SELECT 1 FROM pm_run_bindings b WHERE b.session_run_id=r.id)",
        [run_id.into()],
    )).await.map_err(AppError::database)?;
    if row.is_none() {
        return Ok(None);
    }
    hermes_dispatch_journal::get_for_run(repo, run_id).await
}

pub(super) async fn queue(
    repo: &PostgresFleetRepository,
    after: Option<Uuid>,
) -> Result<Vec<(SessionMessage, SessionAgentRun)>, AppError> {
    let rows = repo.db.query_all(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT j.run_id,j.message_id FROM hermes_dispatch_journal j
         JOIN session_agent_runs r ON r.id=j.run_id AND r.agent_id=j.agent_id AND r.session_id=j.session_id
         JOIN agent_sessions s ON s.id=r.session_id AND s.agent_id=r.agent_id
         JOIN agents a ON a.id=r.agent_id
         JOIN session_messages m ON m.id=j.message_id AND m.session_id=r.session_id
         JOIN message_dispatch_outbox o ON o.message_id=m.id AND o.agent_id=r.agent_id
         WHERE a.kind='hermes' AND a.archived_at IS NULL
           AND r.state IN ('pending','running','waiting','stopping') AND r.run_role='primary'
           AND r.runtime_session_id IS NOT NULL
           AND m.author_type IN ('user','agent') AND m.message_kind IN ('user_prompt','control')
           AND ((j.state='accepted' AND r.runtime_run_id IS NOT NULL
                 AND m.runtime_message_id=r.runtime_run_id AND m.delivery_state='dispatched' AND o.state='dispatched')
             OR (j.state='submitted' AND r.runtime_run_id IS NULL AND r.state='pending'
                 AND m.runtime_message_id IS NULL AND m.delivery_state='pending'
                 AND o.state IN ('dispatching','uncertain')
                 AND jsonb_typeof(j.capabilities->'fleet_recovery')='object'))
           AND NOT EXISTS(SELECT 1 FROM task_chat_bindings b WHERE b.session_id=r.session_id)
           AND NOT EXISTS(SELECT 1 FROM pm_run_bindings b WHERE b.session_run_id=r.id OR b.session_id=r.session_id)
           AND ($1::uuid IS NULL OR r.id>$1) ORDER BY r.id LIMIT 20",
        [after.into()],
    )).await.map_err(AppError::database)?;
    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let message_id: Uuid = row.try_get("", "message_id").map_err(AppError::database)?;
        let run_id: Uuid = row.try_get("", "run_id").map_err(AppError::database)?;
        result.push((
            repo.message_by_id(message_id).await?,
            repo.get_session_agent_run(run_id).await?,
        ));
    }
    Ok(result)
}

pub(super) async fn record(
    repo: &PostgresFleetRepository,
    req: RuntimeApprovalCreate,
    native_session_id: String,
    origin: String,
    credential_fingerprint: String,
) -> Result<(RuntimeApprovalRequest, bool), AppError> {
    let request_id = req
        .runtime_approval_id
        .as_deref()
        .filter(|id| domain::valid_ref(id, 256))
        .ok_or_else(|| AppError::validation("exact approval request identity is required"))?;
    if !domain::valid_ref(&native_session_id, 512)
        || req.detail["event"] != "approval.request"
        || req.detail["run_id"].as_str() != Some(req.runtime_run_id.as_str())
        || req.detail["request_id"].as_str() != Some(request_id)
        || req.prompt.is_empty()
        || req.prompt.len() > 16_384
        || serde_json::to_vec(&req.detail)
            .map_err(AppError::internal)?
            .len()
            > 65_536
    {
        return Err(AppError::validation("invalid bounded approval snapshot"));
    }
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id=$1 FOR NO KEY UPDATE",
        [req.agent_id.into()],
    ))
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::not_found("agent", req.agent_id))?;
    let agent = agent::Entity::find_by_id(req.agent_id)
        .one(&txn)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent", req.agent_id))?;
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agent_sessions WHERE id=$1 FOR NO KEY UPDATE",
        [req.session_id.into()],
    ))
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::not_found("agent_session", req.session_id))?;
    let session = agent_session::Entity::find_by_id(req.session_id)
        .one(&txn)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent_session", req.session_id))?;
    let run = session_agent_run::Entity::find_by_id(req.session_run_id)
        .lock_exclusive()
        .one(&txn)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("session_agent_run", req.session_run_id))?;
    if agent.kind != "hermes"
        || agent.archived_at.is_some()
        || session.agent_id != req.agent_id
        || run.session_id != req.session_id
        || run.agent_id != req.agent_id
        || run.run_role != "primary"
        || run.runtime_run_id.as_deref() != Some(req.runtime_run_id.as_str())
        || run.runtime_session_id.as_deref() != Some(native_session_id.as_str())
        || !matches!(run.state.as_str(), "running" | "waiting" | "stopping")
        || agent
            .api_port
            .map(|port| format!("http://127.0.0.1:{port}"))
            != Some(origin.clone())
    {
        return Err(AppError::conflict(
            "approval snapshot current identity changed",
        ));
    }
    let accepted = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT 1 FROM hermes_dispatch_journal j
         JOIN session_messages m ON m.id=j.message_id AND m.session_id=j.session_id
         JOIN message_dispatch_outbox o ON o.message_id=m.id AND o.agent_id=j.agent_id
         WHERE j.run_id=$1 AND j.session_id=$2 AND j.agent_id=$3 AND j.state='accepted'
           AND j.origin=$4 AND j.credential_fingerprint=$5
           AND m.runtime_message_id=$6 AND m.delivery_state='dispatched' AND o.state='dispatched'
           AND NOT EXISTS(SELECT 1 FROM task_chat_bindings b WHERE b.session_id=$2)
           AND NOT EXISTS(SELECT 1 FROM pm_run_bindings b WHERE b.session_run_id=$1)",
            [
                run.id.into(),
                req.session_id.into(),
                req.agent_id.into(),
                origin.into(),
                credential_fingerprint.into(),
                req.runtime_run_id.clone().into(),
            ],
        ))
        .await
        .map_err(AppError::database)?;
    if accepted.is_none() {
        return Err(AppError::Unavailable(
            "approval snapshot has no accepted free-chat context".into(),
        ));
    }
    let id = Uuid::new_v4();
    let prompt = redact_text(&req.prompt);
    let detail = redact_json(req.detail);
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_approval_requests(id,session_id,session_run_id,agent_id,runtime_run_id,runtime_approval_id,prompt,detail,state,created_at)
         VALUES($1,$2,$3,$4,$5,$6,$7,$8,'pending',clock_timestamp())
         ON CONFLICT(session_run_id,runtime_approval_id) WHERE runtime_approval_id IS NOT NULL DO NOTHING",
        [id.into(),req.session_id.into(),run.id.into(),req.agent_id.into(),req.runtime_run_id.clone().into(),
         request_id.into(),prompt.clone().into(),detail.clone().into()])).await.map_err(AppError::database)?;
    let row = runtime_approval_request::Entity::find()
        .filter(runtime_approval_request::Column::SessionRunId.eq(run.id))
        .filter(runtime_approval_request::Column::RuntimeApprovalId.eq(request_id))
        .one(&txn)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::internal("missing recovered approval snapshot"))?;
    if row.session_id != req.session_id
        || row.agent_id != req.agent_id
        || row.runtime_run_id != req.runtime_run_id
        || row.prompt != prompt
        || row.detail != detail
    {
        return Err(AppError::conflict("approval snapshot content changed"));
    }
    // A prior decision and stopping/terminal state are never reopened by readback.
    if row.state == "pending" && run.state == "running" {
        let mut updated = run.into_active_model();
        updated.state = Set("waiting".into());
        updated.updated_at = Set(now());
        updated.update(&txn).await.map_err(AppError::database)?;
    }
    let created = row.id == id;
    txn.commit().await.map_err(AppError::database)?;
    Ok((runtime_approval_from_model(row), created))
}
