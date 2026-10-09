use super::*;

pub(super) async fn accept(
    repo: &PostgresFleetRepository,
    message_id: Uuid,
    run_id: Uuid,
    runtime_run_id: String,
) -> Result<SessionAgentRun, AppError> {
    if message_id.is_nil() || run_id.is_nil() || !pm_execution::valid_hermes_ref(&runtime_run_id) {
        return Err(AppError::validation("invalid Hermes acceptance identity"));
    }
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let run = locked_run(&txn, run_id).await?;
    let message = session_message::Entity::find_by_id(message_id)
        .lock_exclusive()
        .one(&txn)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("session_message", message_id))?;
    let outbox = locked_outbox(&txn, message_id).await?;
    validate_message(&run, &message, &outbox)?;
    let intent = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT run_id,state FROM hermes_dispatch_journal WHERE message_id=$1 FOR UPDATE",
            [message_id.into()],
        ))
        .await
        .map_err(|_| AppError::Database("Hermes acceptance journal is unavailable".into()))?;
    if let Some(intent) = &intent {
        let journal_run: Uuid = intent.try_get("", "run_id").map_err(AppError::database)?;
        let state: String = intent.try_get("", "state").map_err(AppError::database)?;
        let expected = if run.runtime_run_id.is_some() {
            "accepted"
        } else {
            "submitted"
        };
        if journal_run != run_id || state != expected {
            return Err(AppError::conflict(
                "Hermes ACK does not match its submission journal",
            ));
        }
    }
    if let Some(previous) = &run.runtime_run_id {
        if previous != &runtime_run_id
            || message.runtime_message_id.as_deref() != Some(runtime_run_id.as_str())
            || !matches!(
                message.delivery_state.as_str(),
                "dispatched" | "completed" | "failed"
            )
            || outbox.1 != "dispatched"
        {
            return Err(AppError::conflict(
                "Hermes acceptance mapping is immutable or inconsistent",
            ));
        }
        txn.commit().await.map_err(AppError::database)?;
        return session_run_from_model(&repo.db, run).await;
    }
    if run.state != "pending"
        || run
            .runtime_session_id
            .as_deref()
            .is_none_or(|id| !domain::valid_ref(id, 512))
        || message.runtime_message_id.is_some()
        || message.delivery_state != "pending"
        || !matches!(outbox.1.as_str(), "dispatching" | "uncertain")
    {
        return Err(AppError::conflict(
            "Hermes acceptance requires an unresolved prepared dispatch",
        ));
    }
    let reused = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM session_agent_runs WHERE agent_id=$1 AND runtime_run_id=$2 AND id<>$3)
            OR EXISTS(SELECT 1 FROM session_messages m JOIN message_dispatch_outbox o ON o.message_id=m.id
                WHERE o.agent_id=$1 AND m.runtime_message_id=$2 AND m.id<>$4) AS reused",
        [run.agent_id.into(), runtime_run_id.clone().into(), run_id.into(), message_id.into()]))
        .await.map_err(AppError::database)?.ok_or_else(|| AppError::internal("missing runtime identity check"))?;
    if reused
        .try_get::<bool>("", "reused")
        .map_err(AppError::database)?
    {
        return Err(AppError::conflict(
            "Hermes run ID is already bound to another dispatch",
        ));
    }
    let mut updated = run.into_active_model();
    updated.runtime_run_id = Set(Some(runtime_run_id.clone()));
    updated.updated_at = Set(now());
    let updated = updated.update(&txn).await.map_err(AppError::database)?;
    let mut message = message.into_active_model();
    message.runtime_message_id = Set(Some(runtime_run_id));
    message.delivery_state = Set("dispatched".into());
    message.delivery_error = Set(None);
    message.update(&txn).await.map_err(AppError::database)?;
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE message_dispatch_outbox SET state='dispatched',last_error=NULL,updated_at=now() WHERE message_id=$1",
        [message_id.into()])).await.map_err(AppError::database)?;
    if intent.is_some() {
        txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "UPDATE hermes_dispatch_journal SET state='accepted',accepted_at=clock_timestamp() WHERE message_id=$1",
            [message_id.into()])).await.map_err(|_| AppError::Database("Hermes acceptance journal update failed".into()))?;
    }
    txn.commit().await.map_err(AppError::database)?;
    session_run_from_model(&repo.db, updated).await
}

pub(super) async fn pin_session(
    repo: &PostgresFleetRepository,
    run_id: Uuid,
    runtime_run_id: String,
    requested_session_id: String,
    effective_session_id: String,
) -> Result<(SessionAgentRun, bool), AppError> {
    if run_id.is_nil()
        || !pm_execution::valid_hermes_ref(&runtime_run_id)
        || !domain::valid_ref(&requested_session_id, 512)
        || !domain::valid_ref(&effective_session_id, 512)
    {
        return Err(AppError::validation("invalid Hermes session pin identity"));
    }
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let run = locked_run(&txn, run_id).await?;
    if run.runtime_run_id.as_deref() != Some(runtime_run_id.as_str()) {
        return Err(AppError::conflict(
            "Hermes session pin requires exact persisted acceptance",
        ));
    }
    let accepted = txn.query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT m.id FROM session_messages m JOIN message_dispatch_outbox o ON o.message_id=m.id
         WHERE m.session_id=$1 AND o.agent_id=$2 AND m.runtime_message_id=$3
            AND m.message_kind IN ('user_prompt','control') ORDER BY m.id LIMIT 2",
        [run.session_id.into(), run.agent_id.into(), runtime_run_id.clone().into()]))
        .await.map_err(AppError::database)?;
    if accepted.len() != 1 {
        return Err(AppError::conflict(
            "Hermes session pin requires one atomic dispatch acceptance",
        ));
    }
    let message_id: Uuid = accepted[0].try_get("", "id").map_err(AppError::database)?;
    let message = session_message::Entity::find_by_id(message_id)
        .lock_exclusive()
        .one(&txn)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("session_message", message_id))?;
    let outbox = locked_outbox(&txn, message_id).await?;
    validate_message(&run, &message, &outbox)?;
    if message.runtime_message_id.as_deref() != Some(runtime_run_id.as_str())
        || !matches!(
            message.delivery_state.as_str(),
            "dispatched" | "completed" | "failed"
        )
        || outbox.1 != "dispatched"
    {
        return Err(AppError::conflict(
            "Hermes dispatch acceptance is inconsistent",
        ));
    }
    if run.state != "pending" {
        if run.runtime_session_id.as_deref() != Some(effective_session_id.as_str()) {
            return Err(AppError::conflict(
                "Hermes effective session pin is immutable",
            ));
        }
        txn.commit().await.map_err(AppError::database)?;
        return Ok((session_run_from_model(&repo.db, run).await?, false));
    }
    if message.delivery_state != "dispatched" {
        return Err(AppError::conflict(
            "unresolved session pin requires dispatched acceptance",
        ));
    }
    if run.runtime_session_id.as_deref() != Some(requested_session_id.as_str()) {
        return Err(AppError::conflict(
            "Hermes requested session alias changed before pin",
        ));
    }
    let mut updated = run.into_active_model();
    updated.runtime_session_id = Set(Some(effective_session_id));
    updated.state = Set("running".into());
    updated.last_error = Set(None);
    updated.last_event_at = Set(Some(now()));
    updated.updated_at = Set(now());
    let updated = updated.update(&txn).await.map_err(AppError::database)?;
    txn.commit().await.map_err(AppError::database)?;
    Ok((session_run_from_model(&repo.db, updated).await?, true))
}

async fn locked_run(
    txn: &sea_orm::DatabaseTransaction,
    run_id: Uuid,
) -> Result<session_agent_run::Model, AppError> {
    let seed = session_agent_run::Entity::find_by_id(run_id)
        .one(txn)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("session_agent_run", run_id))?;
    // NO KEY UPDATE serializes capacity without deadlocking message inserts' agent FK key-share.
    let agent = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT kind FROM agents WHERE id=$1 FOR NO KEY UPDATE",
            [seed.agent_id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent", seed.agent_id))?;
    let session = agent_session::Entity::find_by_id(seed.session_id)
        .lock_exclusive()
        .one(txn)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent_session", seed.session_id))?;
    let run = session_agent_run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(txn)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("session_agent_run", run_id))?;
    let kind: String = agent.try_get("", "kind").map_err(AppError::database)?;
    if run.agent_id != seed.agent_id
        || run.session_id != seed.session_id
        || session.agent_id != run.agent_id
        || kind != "hermes"
    {
        return Err(AppError::conflict(
            "Hermes acceptance requires the concrete current primary agent",
        ));
    }
    let scoped = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM task_chat_bindings WHERE session_id=$1)
             OR EXISTS(SELECT 1 FROM pm_run_bindings WHERE session_run_id=$2) AS scoped",
            [run.session_id.into(), run.id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::internal("missing task/PM boundary check"))?;
    if scoped
        .try_get::<bool>("", "scoped")
        .map_err(AppError::database)?
    {
        return Err(AppError::conflict(
            "task-bound and PM runs require their own acceptance authority",
        ));
    }
    Ok(run)
}

async fn locked_outbox(
    txn: &sea_orm::DatabaseTransaction,
    message_id: Uuid,
) -> Result<(Uuid, String), AppError> {
    let outbox = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT agent_id,state FROM message_dispatch_outbox WHERE message_id=$1 FOR UPDATE",
            [message_id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::conflict("Hermes message has no dispatch outbox"))?;
    Ok((
        outbox.try_get("", "agent_id").map_err(AppError::database)?,
        outbox.try_get("", "state").map_err(AppError::database)?,
    ))
}

fn validate_message(
    run: &session_agent_run::Model,
    message: &session_message::Model,
    outbox: &(Uuid, String),
) -> Result<(), AppError> {
    if message.session_id != run.session_id
        || outbox.0 != run.agent_id
        || !matches!(message.message_kind.as_str(), "user_prompt" | "control")
    {
        return Err(AppError::conflict(
            "Hermes message/session/agent does not match prepared run",
        ));
    }
    Ok(())
}
