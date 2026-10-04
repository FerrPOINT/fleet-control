use super::*;
use app::{HermesDispatchDraft, HermesDispatchIntent};
use sea_orm::{DatabaseTransaction, FromQueryResult, QueryResult};

const MAX_REQUEST_BYTES: usize = 1_048_576;

fn database_error(error: sea_orm::DbErr) -> AppError {
    if matches!(
        error.sql_err(),
        Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
    ) {
        AppError::conflict("Hermes dispatch journal identity or capacity conflict")
    } else {
        AppError::Database("Hermes dispatch journal database operation failed".into())
    }
}

const JOURNAL_COLUMNS: &str = "j.message_id,j.run_id,j.session_id,j.agent_id,j.run_role,j.requested_session_id,
    j.request_body,j.request_hash,j.idempotency_key,j.origin,j.credential_fingerprint,j.capabilities,
    j.retention_seconds,j.created_at,j.recovery_deadline,j.state,j.submitted_at,j.accepted_at,
    j.recovery_deadline>clock_timestamp() AS recovery_allowed";

pub(super) async fn prepare(
    repo: &PostgresFleetRepository,
    draft: HermesDispatchDraft,
) -> Result<HermesDispatchIntent, AppError> {
    validate_draft(&draft)?;
    let txn = repo.db.begin().await.map_err(database_error)?;
    let agent = lock_agent(&txn, draft.agent_id).await?;
    let session = lock_session(&txn, draft.session_id, draft.agent_id).await?;
    let previous = journal(&txn, draft.message_id, false).await?;
    let run = if let Some(row) = &previous {
        if column::<Uuid>(row, "session_id")? != draft.session_id
            || column::<Uuid>(row, "agent_id")? != draft.agent_id
        {
            return Err(AppError::conflict("Hermes journal scope changed"));
        }
        locked_run(&txn, column(row, "run_id")?).await?
    } else {
        // Reuse the seeded pending run, keeping its model/provider/options in the exact request.
        let seeded = session_agent_run::Entity::find()
            .filter(session_agent_run::Column::SessionId.eq(draft.session_id))
            .filter(session_agent_run::Column::AgentId.eq(draft.agent_id))
            .filter(session_agent_run::Column::State.eq("pending"))
            .filter(session_agent_run::Column::RuntimeRunId.is_null())
            .filter(session_agent_run::Column::RuntimeSessionId.is_null())
            .order_by_asc(session_agent_run::Column::CreatedAt)
            .order_by_asc(session_agent_run::Column::Id)
            .lock_exclusive()
            .one(&txn)
            .await
            .map_err(database_error)?;
        match seeded {
            Some(run) => run,
            None => pending_session_run(
                draft.session_id,
                draft.agent_id,
                draft.run_role,
                db_now(&txn).await?,
            )
            .insert(&txn)
            .await
            .map_err(database_error)?,
        }
    };
    free_scope(&txn, &session, &run).await?;
    let (message, outbox) =
        lock_message(&txn, draft.message_id, draft.session_id, draft.agent_id).await?;
    if let Some(row) = previous {
        let body: Value = serde_json::from_str(&column::<String>(&row, "request_body")?)
            .map_err(|_| AppError::conflict("invalid persisted Hermes request"))?;
        if column::<String>(&row, "origin")? != draft.origin
            || column::<String>(&row, "credential_fingerprint")? != draft.credential_fingerprint
            || column::<String>(&row, "run_role")? != draft.run_role.as_str()
            || column::<String>(&row, "requested_session_id")? != draft.requested_session_id
            || column::<Value>(&row, "capabilities")? != draft.capabilities
            || body["input"].as_str() != Some(draft.input.as_str())
        {
            return Err(AppError::conflict("Hermes dispatch exact request changed"));
        }
        let intent = intent(row, run, agent.name)?;
        txn.commit().await.map_err(database_error)?;
        return Ok(intent);
    }
    validate_current(&txn, &agent, &session, &run, &draft.origin).await?;
    let expected_role =
        if agent.product_role == "leader" || session.leader_agent_id == Some(agent.id) {
            SessionRunRole::Leader
        } else {
            SessionRunRole::Primary
        };
    if draft.run_role != expected_role
        || draft.requested_session_id != format!("fleet:{}:{}", draft.session_id, draft.agent_id)
        || draft.input != runtime_input(&session, &message, agent.id)
        || message.runtime_message_id.is_some()
        || message.delivery_state != "pending"
        || !matches!(outbox.as_str(), "dispatching" | "uncertain")
    {
        return Err(AppError::conflict(
            "Hermes journal requires an unresolved free-chat dispatch",
        ));
    }
    validate_capabilities(&draft.capabilities)?;
    let mut body = json!({"input":draft.input,"session_id":draft.requested_session_id});
    if let Some(model) = &run.model {
        body["model"] = json!(model);
    }
    if let Some(provider) = &run.provider {
        body["provider"] = json!(provider);
    }
    if !matches!(&run.model_options, Value::Object(map) if map.is_empty()) {
        body["model_options"] = run.model_options.clone();
    }
    let request_body = serde_json::to_string(&body)
        .map_err(|_| AppError::internal("Hermes request serialization failed"))?;
    if request_body.len() > MAX_REQUEST_BYTES {
        return Err(AppError::validation("Hermes request exceeds journal bound"));
    }
    let request_hash = hex::encode(Sha256::digest(request_body.as_bytes()));
    let mut run = run.into_active_model();
    run.runtime_session_id = Set(Some(draft.requested_session_id.clone()));
    run.run_role = Set(draft.run_role.as_str().into());
    run.updated_at = Set(db_now(&txn).await?);
    let run = run.update(&txn).await.map_err(database_error)?;
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "WITH ts AS (SELECT clock_timestamp() AS value)
         INSERT INTO hermes_dispatch_journal(message_id,run_id,session_id,agent_id,run_role,requested_session_id,
             request_body,request_hash,idempotency_key,origin,credential_fingerprint,capabilities,created_at,recovery_deadline)
         SELECT $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,value,value+interval '86340 seconds' FROM ts",
        [draft.message_id.into(),run.id.into(),draft.session_id.into(),draft.agent_id.into(),
         draft.run_role.as_str().into(),draft.requested_session_id.into(),request_body.into(),request_hash.into(),
         draft.message_id.to_string().into(),draft.origin.into(),draft.credential_fingerprint.into(),draft.capabilities.into()]))
        .await.map_err(database_error)?;
    let row = journal(&txn, draft.message_id, false)
        .await?
        .ok_or_else(|| AppError::internal("missing inserted Hermes journal"))?;
    let result = intent(row, run, agent.name)?;
    txn.commit().await.map_err(database_error)?;
    Ok(result)
}

pub(super) async fn claim(
    repo: &PostgresFleetRepository,
    message_id: Uuid,
    origin: String,
    credential_fingerprint: String,
) -> Result<Option<HermesDispatchIntent>, AppError> {
    let Some(seed) = journal(&repo.db, message_id, false).await? else {
        return Ok(None);
    };
    let txn = repo.db.begin().await.map_err(database_error)?;
    let agent = lock_agent(&txn, column(&seed, "agent_id")?).await?;
    let session = lock_session(&txn, column(&seed, "session_id")?, agent.id).await?;
    let run = locked_run(&txn, column(&seed, "run_id")?).await?;
    free_scope(&txn, &session, &run).await?;
    let (message, outbox) = lock_message(&txn, message_id, session.id, agent.id).await?;
    let row = journal(&txn, message_id, true)
        .await?
        .ok_or_else(|| AppError::internal("Hermes journal disappeared"))?;
    let frozen_role: String = column(&row, "run_role")?;
    let result = intent(row, run.clone(), agent.name.clone())?;
    if result.origin != origin || result.credential_fingerprint != credential_fingerprint {
        return Err(AppError::conflict(
            "Hermes submission origin or credential changed",
        ));
    }
    if result.submission_attempted {
        txn.commit().await.map_err(database_error)?;
        return Ok(None);
    }
    validate_current(&txn, &agent, &session, &run, &origin).await?;
    if run.run_role != frozen_role
        || run.runtime_session_id.as_deref()
            != Some(format!("fleet:{}:{}", session.id, agent.id).as_str())
        || message.runtime_message_id.is_some()
        || message.delivery_state != "pending"
        || !matches!(outbox.as_str(), "dispatching" | "uncertain")
    {
        return Err(AppError::conflict(
            "Hermes submission requires the exact pending claimed dispatch",
        ));
    }
    let claimed = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        format!("UPDATE hermes_dispatch_journal AS j SET state='submitted' WHERE message_id=$1 AND state='prepared'
             AND recovery_deadline>clock_timestamp() RETURNING {JOURNAL_COLUMNS}"),[message_id.into()]))
        .await.map_err(database_error)?;
    let result = claimed
        .map(|row| intent(row, run, agent.name))
        .transpose()?;
    if result.is_some() && outbox == "uncertain" {
        // Prepared proves no submission permit was consumed. Do not reset submitted receipts.
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE message_dispatch_outbox SET state='dispatching',updated_at=clock_timestamp()
             WHERE message_id=$1 AND state='uncertain'",
            [message_id.into()],
        ))
        .await
        .map_err(database_error)?;
    }
    txn.commit().await.map_err(database_error)?;
    Ok(result)
}

pub(super) async fn get(
    repo: &PostgresFleetRepository,
    message_id: Uuid,
) -> Result<Option<HermesDispatchIntent>, AppError> {
    // One statement observes the receipt and live run together; readback never renews/reset anything.
    let row = repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        format!("SELECT {JOURNAL_COLUMNS},a.name AS agent_name,
            r.id AS r_id,r.session_id AS r_session_id,r.agent_id AS r_agent_id,
            r.runtime_session_id AS r_runtime_session_id,r.runtime_run_id AS r_runtime_run_id,
            r.run_role AS r_run_role,r.state AS r_state,r.last_error AS r_last_error,
            r.last_event_at AS r_last_event_at,r.model AS r_model,r.provider AS r_provider,
            r.model_options AS r_model_options,r.created_at AS r_created_at,r.updated_at AS r_updated_at
         FROM hermes_dispatch_journal j
         JOIN session_agent_runs r ON r.id=j.run_id JOIN agents a ON a.id=j.agent_id WHERE j.message_id=$1"),
        [message_id.into()])).await.map_err(database_error)?;
    row.map(|row| {
        let run =
            session_agent_run::Model::from_query_result(&row, "r_").map_err(database_error)?;
        let name = column(&row, "agent_name")?;
        intent(row, run, name)
    })
    .transpose()
}

fn validate_draft(draft: &HermesDispatchDraft) -> Result<(), AppError> {
    if draft.message_id.is_nil()
        || draft.session_id.is_nil()
        || draft.agent_id.is_nil()
        || draft.input.is_empty()
        || draft.input.len() > MAX_REQUEST_BYTES
        || !domain::valid_ref(&draft.requested_session_id, 512)
        || !sha256_ref(&draft.credential_fingerprint)
        || serde_json::to_vec(&draft.capabilities)
            .map_err(AppError::internal)?
            .len()
            > 262_144
    {
        return Err(AppError::validation(
            "invalid bounded Hermes dispatch draft",
        ));
    }
    Ok(())
}

fn validate_capabilities(caps: &Value) -> Result<(), AppError> {
    crate::runtime::recovery_wire::store_id(caps)?;
    if caps["object"] != "hermes.api_server.capabilities"
        || caps["platform"] != "hermes-agent"
        || caps["auth"]["type"] != "bearer"
        || caps["auth"]["required"] != true
        || caps["runtime"]["mode"] != "server_agent"
        || caps["runtime"]["tool_execution"] != "server"
        || caps["runtime"]["split_runtime"] != false
        || caps["features"]["runs_idempotency"]["supported"] != true
        || caps["features"]["runs_idempotency"]["durable"] != true
        || caps["features"]["runs_idempotency"]["retention_seconds"].as_u64() != Some(86_400)
    {
        return Err(AppError::Unavailable(
            "Hermes durable dispatch protocol is not verified".into(),
        ));
    }
    for (feature, endpoint, method, path) in [
        ("run_submission", "runs", "POST", "/v1/runs"),
        ("run_status", "run_status", "GET", "/v1/runs/{run_id}"),
        (
            "run_events_sse",
            "run_events",
            "GET",
            "/v1/runs/{run_id}/events",
        ),
        ("run_stop", "run_stop", "POST", "/v1/runs/{run_id}/stop"),
    ] {
        if caps["features"][feature] != true
            || caps["endpoints"][endpoint]["method"] != method
            || caps["endpoints"][endpoint]["path"] != path
        {
            return Err(AppError::Unavailable(
                "Hermes dispatch protocol endpoints do not match".into(),
            ));
        }
    }
    Ok(())
}

fn sha256_ref(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn runtime_input(
    session: &agent_session::Model,
    message: &session_message::Model,
    agent_id: Uuid,
) -> String {
    if message.author_type == "agent" && message.author_agent_id != Some(agent_id) {
        format!(
            "[Fleet Control]\nSession: {}\nTask: {}\nMessage from leader agent: {}\n\n{}",
            session.title,
            session.task_key.as_deref().unwrap_or("not set"),
            message
                .author_agent_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unknown".into()),
            message.body
        )
    } else {
        message.body.clone()
    }
}

async fn lock_agent(txn: &DatabaseTransaction, id: Uuid) -> Result<agent::Model, AppError> {
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id=$1 FOR NO KEY UPDATE",
        [id.into()],
    ))
    .await
    .map_err(database_error)?
    .ok_or_else(|| AppError::not_found("agent", id))?;
    agent::Entity::find_by_id(id)
        .one(txn)
        .await
        .map_err(database_error)?
        .ok_or_else(|| AppError::not_found("agent", id))
}

async fn lock_session(
    txn: &DatabaseTransaction,
    id: Uuid,
    agent_id: Uuid,
) -> Result<agent_session::Model, AppError> {
    let session = agent_session::Entity::find_by_id(id)
        .lock_exclusive()
        .one(txn)
        .await
        .map_err(database_error)?
        .ok_or_else(|| AppError::not_found("agent_session", id))?;
    if session.agent_id != agent_id {
        return Err(AppError::conflict(
            "Hermes dispatch requires concrete current primary",
        ));
    }
    Ok(session)
}

async fn locked_run(
    txn: &DatabaseTransaction,
    id: Uuid,
) -> Result<session_agent_run::Model, AppError> {
    session_agent_run::Entity::find_by_id(id)
        .lock_exclusive()
        .one(txn)
        .await
        .map_err(database_error)?
        .ok_or_else(|| AppError::not_found("session_agent_run", id))
}

async fn free_scope(
    txn: &DatabaseTransaction,
    session: &agent_session::Model,
    run: &session_agent_run::Model,
) -> Result<(), AppError> {
    if run.session_id != session.id || run.agent_id != session.agent_id {
        return Err(AppError::conflict("Hermes run scope changed"));
    }
    let row = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM task_chat_bindings WHERE session_id=$1)
            OR EXISTS(SELECT 1 FROM pm_run_bindings WHERE session_run_id=$2 OR session_id=$1) AS bound",
        [session.id.into(),run.id.into()])).await.map_err(database_error)?
        .ok_or_else(|| AppError::internal("missing Hermes scope check"))?;
    if column::<bool>(&row, "bound")? {
        return Err(AppError::conflict(
            "task-bound and PM dispatch require separate authority",
        ));
    }
    Ok(())
}

async fn lock_message(
    txn: &DatabaseTransaction,
    id: Uuid,
    session_id: Uuid,
    agent_id: Uuid,
) -> Result<(session_message::Model, String), AppError> {
    let message = session_message::Entity::find_by_id(id)
        .lock_exclusive()
        .one(txn)
        .await
        .map_err(database_error)?
        .ok_or_else(|| AppError::not_found("session_message", id))?;
    let row = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT agent_id,state FROM message_dispatch_outbox WHERE message_id=$1 FOR UPDATE",
            [id.into()],
        ))
        .await
        .map_err(database_error)?
        .ok_or_else(|| AppError::conflict("Hermes dispatch outbox missing"))?;
    if message.session_id != session_id
        || column::<Uuid>(&row, "agent_id")? != agent_id
        || !matches!(message.message_kind.as_str(), "user_prompt" | "control")
    {
        return Err(AppError::conflict("Hermes message/outbox scope mismatch"));
    }
    Ok((message, column(&row, "state")?))
}

async fn validate_current(
    txn: &DatabaseTransaction,
    agent: &agent::Model,
    session: &agent_session::Model,
    run: &session_agent_run::Model,
    origin: &str,
) -> Result<(), AppError> {
    if agent.kind != "hermes"
        || agent.status != "running"
        || agent.archived_at.is_some()
        || agent
            .api_port
            .is_none_or(|port| !(1024..=65535).contains(&port))
        || origin != format!("http://127.0.0.1:{}", agent.api_port.unwrap_or_default())
        || run.state != "pending"
        || run.runtime_run_id.is_some()
        || session.agent_id != agent.id
        || run.agent_id != agent.id
        || run.session_id != session.id
    {
        return Err(AppError::conflict(
            "Hermes runtime/dispatch identity is no longer current",
        ));
    }
    let row = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM agent_config_heads WHERE agent_id=$1 AND draining)
            OR EXISTS(SELECT 1 FROM session_agent_runs WHERE agent_id=$1 AND id<>$2
                AND state IN ('pending','running','waiting','stopping') AND runtime_session_id IS NOT NULL)
            OR EXISTS(SELECT 1 FROM hermes_dispatch_journal WHERE agent_id=$1 AND run_id<>$2 AND state IN ('prepared','submitted')) AS busy",
        [agent.id.into(),run.id.into()])).await.map_err(database_error)?
        .ok_or_else(|| AppError::internal("missing Hermes capacity check"))?;
    if column::<bool>(&row, "busy")? {
        return Err(AppError::conflict(
            "Hermes capacity held or configuration draining",
        ));
    }
    Ok(())
}

async fn journal<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
    lock: bool,
) -> Result<Option<QueryResult>, AppError> {
    db.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        format!(
            "SELECT {JOURNAL_COLUMNS} FROM hermes_dispatch_journal AS j WHERE message_id=$1{}",
            if lock { " FOR UPDATE" } else { "" }
        ),
        [id.into()],
    ))
    .await
    .map_err(database_error)
}

async fn db_now(txn: &DatabaseTransaction) -> Result<shared::Timestamp, AppError> {
    let row = txn
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT clock_timestamp() AS ts",
        ))
        .await
        .map_err(database_error)?
        .ok_or_else(|| AppError::internal("missing DB clock"))?;
    column(&row, "ts")
}

fn column<T: sea_orm::TryGetable>(row: &QueryResult, key: &str) -> Result<T, AppError> {
    row.try_get("", key).map_err(database_error)
}

fn intent(
    row: QueryResult,
    run: session_agent_run::Model,
    agent_name: String,
) -> Result<HermesDispatchIntent, AppError> {
    let message_id: Uuid = column(&row, "message_id")?;
    let idempotency_key: String = column(&row, "idempotency_key")?;
    let request_body: String = column(&row, "request_body")?;
    let request_hash: String = column(&row, "request_hash")?;
    if request_hash != hex::encode(Sha256::digest(request_body.as_bytes()))
        || idempotency_key != message_id.to_string()
        || column::<Uuid>(&row, "run_id")? != run.id
        || column::<Uuid>(&row, "session_id")? != run.session_id
        || column::<Uuid>(&row, "agent_id")? != run.agent_id
    {
        return Err(AppError::conflict(
            "Hermes journal bytes or run scope do not match persisted identity",
        ));
    }
    let state: String = column(&row, "state")?;
    Ok(HermesDispatchIntent {
        message_id,
        run: SessionAgentRun {
            id: run.id,
            session_id: run.session_id,
            agent_id: run.agent_id,
            agent_name,
            runtime_session_id: run.runtime_session_id,
            runtime_run_id: run.runtime_run_id,
            run_role: parse_run_role(&run.run_role),
            state: parse_run_state(&run.state),
            last_error: run.last_error,
            last_event_at: api_ts_opt(run.last_event_at),
            model: run.model,
            provider: run.provider,
            model_options: run.model_options,
            created_at: api_ts(run.created_at),
            updated_at: api_ts(run.updated_at),
        },
        request_body,
        request_hash,
        idempotency_key,
        origin: column(&row, "origin")?,
        credential_fingerprint: column(&row, "credential_fingerprint")?,
        capabilities: column(&row, "capabilities")?,
        submission_attempted: matches!(state.as_str(), "submitted" | "accepted"),
        state,
        submitted_at: column(&row, "submitted_at")?,
        recovery_deadline: column(&row, "recovery_deadline")?,
        recovery_allowed: column(&row, "recovery_allowed")?,
    })
}
