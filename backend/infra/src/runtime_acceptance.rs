use super::*;

pub(super) async fn accept(
    repo: &PostgresFleetRepository,
    message_id: Uuid,
    run_id: Uuid,
    runtime_run_id: String,
    recovery_capabilities: Option<Value>,
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
            "SELECT run_id,state,capabilities,recovery_deadline>clock_timestamp() AS recovery_allowed
             FROM hermes_dispatch_journal WHERE message_id=$1 FOR UPDATE",
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
    if let Some(expected) = &recovery_capabilities {
        let original = intent
            .as_ref()
            .ok_or_else(|| AppError::conflict("Hermes recovery journal missing"))?;
        let facts: Value = original
            .try_get("", "capabilities")
            .map_err(AppError::database)?;
        let allowed: bool = original
            .try_get("", "recovery_allowed")
            .map_err(AppError::database)?;
        if facts != *expected
            || crate::runtime::recovery_wire::store_id(&facts)?.is_none()
            || (run.runtime_run_id.is_none() && !allowed)
        {
            return Err(AppError::conflict(
                "Hermes recovery context changed or expired",
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
        let changed = txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "UPDATE hermes_dispatch_journal SET state='accepted',accepted_at=clock_timestamp() WHERE message_id=$1
                AND (NOT $2 OR recovery_deadline>clock_timestamp())",
            [message_id.into(), recovery_capabilities.is_some().into()])).await
            .map_err(|_| AppError::Database("Hermes acceptance journal update failed".into()))?;
        if changed.rows_affected() != 1 {
            return Err(AppError::conflict(
                "Hermes recovery horizon expired before commit",
            ));
        }
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

pub(super) async fn terminal(
    repo: &PostgresFleetRepository,
    command: app::HermesTerminalCommit,
) -> Result<(SessionAgentRun, Option<SessionMessage>, bool), AppError> {
    if command.message_id.is_nil()
        || command.run_id.is_nil()
        || !pm_execution::valid_hermes_ref(&command.runtime_run_id)
        || !domain::valid_ref(&command.runtime_session_id, 512)
        || !matches!(
            command.state,
            SessionRunState::Completed | SessionRunState::Failed | SessionRunState::Cancelled
        )
        || (command.state != SessionRunState::Completed && command.body.is_some())
    {
        return Err(AppError::validation("invalid Hermes terminal packet"));
    }
    let txn = repo.db.begin().await.map_err(terminal_database_error)?;
    let run = locked_run(&txn, command.run_id).await?;
    let message = session_message::Entity::find_by_id(command.message_id)
        .lock_exclusive()
        .one(&txn)
        .await
        .map_err(terminal_database_error)?
        .ok_or_else(|| AppError::not_found("session_message", command.message_id))?;
    let outbox = locked_outbox(&txn, command.message_id).await?;
    validate_message(&run, &message, &outbox)?;
    let journal = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT run_id,session_id,agent_id,run_role,state FROM hermes_dispatch_journal
             WHERE message_id=$1 FOR UPDATE",
            [command.message_id.into()],
        ))
        .await
        .map_err(terminal_database_error)?
        .ok_or_else(|| AppError::conflict("Hermes terminal packet requires an accepted journal"))?;
    let journal_run: Uuid = journal
        .try_get("", "run_id")
        .map_err(terminal_database_error)?;
    let journal_session: Uuid = journal
        .try_get("", "session_id")
        .map_err(terminal_database_error)?;
    let journal_agent: Uuid = journal
        .try_get("", "agent_id")
        .map_err(terminal_database_error)?;
    let journal_role: String = journal
        .try_get("", "run_role")
        .map_err(terminal_database_error)?;
    let journal_state: String = journal
        .try_get("", "state")
        .map_err(terminal_database_error)?;
    let pm_bound = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM pm_run_bindings WHERE session_id=$1) AS bound",
            [run.session_id.into()],
        ))
        .await
        .map_err(terminal_database_error)?
        .ok_or_else(|| AppError::internal("missing Hermes terminal scope check"))?
        .try_get::<bool>("", "bound")
        .map_err(terminal_database_error)?;
    if pm_bound
        || run.runtime_run_id.as_deref() != Some(command.runtime_run_id.as_str())
        || run.runtime_session_id.as_deref() != Some(command.runtime_session_id.as_str())
        || run.state == "pending"
        || !matches!(run.run_role.as_str(), "primary" | "leader")
        || message.runtime_message_id.as_deref() != Some(command.runtime_run_id.as_str())
        || outbox.1 != "dispatched"
        || journal_state != "accepted"
        || journal_run != run.id
        || journal_session != run.session_id
        || journal_agent != run.agent_id
        || journal_role != run.run_role
    {
        return Err(AppError::conflict(
            "Hermes terminal packet identity or pin does not match",
        ));
    }
    let replay = matches!(run.state.as_str(), "completed" | "failed" | "cancelled");
    terminal_packet(txn, run, Some(message), command, replay).await
}

// Both authorities reach this writer only after validating their original custody.
pub(super) async fn terminal_packet(
    txn: sea_orm::DatabaseTransaction,
    run: session_agent_run::Model,
    message: Option<session_message::Model>,
    command: app::HermesTerminalCommit,
    replay: bool,
) -> Result<(SessionAgentRun, Option<SessionMessage>, bool), AppError> {
    let body = command
        .body
        .as_deref()
        .map(|body| redact_text(body.trim()))
        .filter(|body| !body.is_empty());
    let error = command.error.as_deref().map(redact_text);
    let delivery = if command.state == SessionRunState::Failed {
        MessageDeliveryState::Failed
    } else {
        MessageDeliveryState::Completed
    };
    let delivery_error = if command.state == SessionRunState::Failed {
        error.clone()
    } else {
        None
    };
    let mut assistants = session_message::Entity::find()
        .filter(session_message::Column::SessionId.eq(run.session_id))
        .filter(session_message::Column::MessageKind.eq("assistant_message"))
        .filter(session_message::Column::RuntimeMessageId.eq(command.runtime_run_id.clone()))
        .limit(2)
        .lock_exclusive()
        .all(&txn)
        .await
        .map_err(terminal_database_error)?;
    if assistants.len() > 1 {
        return Err(AppError::conflict(
            "Hermes terminal assistant mapping is ambiguous",
        ));
    }
    let mut assistant = assistants.pop();
    if let Some(previous) = &assistant {
        if previous.author_type != "agent"
            || previous.author_agent_id != Some(run.agent_id)
            || previous.author_user_id.is_some()
            || previous.delivery_state != "mirrored"
            || previous.delivery_error.is_some()
            || body.as_deref() != Some(previous.body.as_str())
        {
            return Err(AppError::conflict(
                "Hermes terminal assistant body or identity changed",
            ));
        }
    }
    if replay {
        if run.state != command.state.as_str()
            || run.last_error != error
            || message.as_ref().is_some_and(|message| {
                message.delivery_state != delivery.as_str()
                    || message.delivery_error != delivery_error
            })
            || body.is_some() != assistant.is_some()
        {
            return Err(AppError::conflict(
                "Hermes terminal packet contradicts committed outcome",
            ));
        }
    } else if !(matches!(run.state.as_str(), "running" | "waiting" | "stopping")
        || message.is_none() && run.state == command.state.as_str())
        || message
            .as_ref()
            .is_some_and(|message| message.delivery_state != "dispatched")
    {
        return Err(AppError::conflict(
            "Hermes terminal packet requires a pinned active run",
        ));
    }
    // Load presentation metadata before any commit; no fallible readback can hide a successful packet.
    let agent = agent::Entity::find_by_id(run.agent_id)
        .one(&txn)
        .await
        .map_err(terminal_database_error)?
        .ok_or_else(|| AppError::not_found("agent", run.agent_id))?;
    let run = if replay {
        run
    } else {
        let ts: shared::Timestamp = txn
            .query_one(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT clock_timestamp() AS ts".to_string(),
            ))
            .await
            .map_err(terminal_database_error)?
            .ok_or_else(|| AppError::internal("missing Hermes terminal DB clock"))?
            .try_get("", "ts")
            .map_err(terminal_database_error)?;
        if assistant.is_none()
            && let Some(body) = body
        {
            let row = session_message::ActiveModel {
                id: Set(Uuid::new_v4()),
                session_id: Set(run.session_id),
                author_type: Set("agent".into()),
                author_user_id: Set(None),
                author_agent_id: Set(Some(run.agent_id)),
                body: Set(body.clone()),
                message_kind: Set("assistant_message".into()),
                runtime_message_id: Set(Some(command.runtime_run_id.clone())),
                idempotency_key: Set(None),
                idempotency_payload_hash: Set(None),
                created_by_user_id: Set(None),
                delivery_state: Set("mirrored".into()),
                delivery_error: Set(None),
                created_at: Set(ts),
            }
            .insert(&txn)
            .await
            .map_err(terminal_database_error)?;
            assistant = Some(row);
            let mut session = agent_session::Entity::find_by_id(run.session_id)
                .one(&txn)
                .await
                .map_err(terminal_database_error)?
                .ok_or_else(|| AppError::not_found("agent_session", run.session_id))?
                .into_active_model();
            session.last_message_preview = Set(Some(body.chars().take(180).collect()));
            session.updated_at = Set(ts);
            session
                .update(&txn)
                .await
                .map_err(terminal_database_error)?;
        }
        if let Some(message) = message {
            let mut prompt = message.into_active_model();
            prompt.delivery_state = Set(delivery.as_str().to_string());
            prompt.delivery_error = Set(delivery_error);
            prompt.update(&txn).await.map_err(terminal_database_error)?;
        }
        let mut updated = run.into_active_model();
        updated.state = Set(command.state.as_str().to_string());
        updated.last_error = Set(error);
        updated.last_event_at = Set(Some(ts));
        updated.updated_at = Set(ts);
        updated
            .update(&txn)
            .await
            .map_err(terminal_database_error)?
    };
    let result = (
        terminal_run(run, &agent),
        assistant.map(|row| SessionMessage {
            id: row.id,
            session_id: row.session_id,
            author_type: MessageAuthorType::Agent,
            author_user_id: None,
            author_agent_id: row.author_agent_id,
            author_display_name: agent.display_name.clone(),
            body: row.body,
            message_kind: MessageKind::AssistantMessage,
            runtime_message_id: row.runtime_message_id,
            delivery_state: MessageDeliveryState::Mirrored,
            delivery_error: None,
            replayed: replay,
            request_payload_hash: None,
            created_at: api_ts(row.created_at),
        }),
        !replay,
    );
    txn.commit().await.map_err(terminal_database_error)?;
    Ok(result)
}

fn terminal_database_error(_: sea_orm::DbErr) -> AppError {
    AppError::Database("Hermes terminal packet database operation failed".into())
}

fn terminal_run(row: session_agent_run::Model, agent: &agent::Model) -> SessionAgentRun {
    SessionAgentRun {
        id: row.id,
        session_id: row.session_id,
        agent_id: row.agent_id,
        agent_name: agent.name.clone(),
        runtime_session_id: row.runtime_session_id,
        runtime_run_id: row.runtime_run_id,
        run_role: parse_run_role(&row.run_role),
        state: parse_run_state(&row.state),
        last_error: row.last_error,
        last_event_at: api_ts_opt(row.last_event_at),
        model: row.model,
        provider: row.provider,
        model_options: row.model_options,
        created_at: api_ts(row.created_at),
        updated_at: api_ts(row.updated_at),
    }
}

async fn locked_run(
    txn: &sea_orm::DatabaseTransaction,
    run_id: Uuid,
) -> Result<session_agent_run::Model, AppError> {
    let run = locked_primary_run(txn, run_id).await?;
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

pub(super) async fn locked_primary_run(
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
    // Serialize dedup without blocking event writers' session FK key-share while they hold a run/message.
    let session = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT agent_id FROM agent_sessions WHERE id=$1 FOR NO KEY UPDATE",
            [seed.session_id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent_session", seed.session_id))?;
    let session_agent_id: Uuid = session
        .try_get("", "agent_id")
        .map_err(AppError::database)?;
    let run = session_agent_run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(txn)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("session_agent_run", run_id))?;
    let kind: String = agent.try_get("", "kind").map_err(AppError::database)?;
    if run.agent_id != seed.agent_id
        || run.session_id != seed.session_id
        || session_agent_id != run.agent_id
        || kind != "hermes"
    {
        return Err(AppError::conflict(
            "Hermes acceptance requires the concrete current primary agent",
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
