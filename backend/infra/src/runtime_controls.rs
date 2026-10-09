use super::*;
use domain::{
    RuntimeControlActor, RuntimeControlOperation, RuntimeControlReceipt, RuntimeControlReservation,
    RuntimeControlState,
};
use sea_orm::{DatabaseTransaction, QueryResult};

fn database_error(error: sea_orm::DbErr) -> AppError {
    if matches!(
        error.sql_err(),
        Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
    ) {
        AppError::conflict("another runtime control holds this run or command key")
    } else {
        AppError::Database("runtime control journal operation failed".into())
    }
}

const RECORD: &str = "jsonb_build_object('id',c.id,'session_id',c.session_id,
    'session_run_id',c.session_run_id,'agent_id',c.agent_id,'actor_user_id',c.actor_user_id,
    'operation',c.operation,'state',c.state,'acknowledgement',c.acknowledgement,
    'observed_run_state',c.observed_run_state,'created_at',c.created_at,'updated_at',c.updated_at)";

fn receipt(row: &QueryResult) -> Result<RuntimeControlReceipt, AppError> {
    serde_json::from_value(row.try_get::<Value>("", "record").map_err(database_error)?)
        .map_err(|_| AppError::Database("invalid runtime control receipt".into()))
}

async fn row<C: ConnectionTrait>(db: &C, id: Uuid, lock: bool) -> Result<QueryResult, AppError> {
    db.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        format!(
            "SELECT {RECORD} AS record,c.idempotency_key,c.payload_sha256,c.runtime_run_id,
            c.runtime_session_id,c.original_request_sha256,c.api_origin,c.credential_fingerprint
            FROM runtime_control_commands c WHERE c.id=$1 {}",
            if lock { "FOR UPDATE" } else { "" }
        ),
        [id.into()],
    ))
    .await
    .map_err(database_error)?
    .ok_or_else(|| AppError::not_found("runtime_control_command", id))
}

pub(super) async fn get(
    repo: &PostgresFleetRepository,
    session: Uuid,
    id: Uuid,
) -> Result<RuntimeControlReceipt, AppError> {
    let result = receipt(&row(&repo.db, id, false).await?)?;
    if result.session_id != session {
        return Err(AppError::not_found("runtime_control_command", id));
    }
    Ok(result)
}

pub(super) async fn list(
    repo: &PostgresFleetRepository,
    session: Uuid,
    run: Uuid,
) -> Result<Vec<RuntimeControlReceipt>, AppError> {
    repo.db.query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        format!("SELECT {RECORD} AS record FROM runtime_control_commands c
            WHERE c.session_id=$1 AND c.session_run_id=$2 ORDER BY c.created_at DESC,c.id LIMIT 100"),
        [session.into(),run.into()])).await.map_err(database_error)?.iter().map(receipt).collect()
}

pub(super) async fn find_by_key(
    repo: &PostgresFleetRepository,
    session: Uuid,
    run: Uuid,
    actor: &RuntimeControlActor,
) -> Result<Option<RuntimeControlReceipt>, AppError> {
    if !domain::valid_ref(&actor.idempotency_key, 128) {
        return Err(AppError::validation("invalid runtime control key"));
    }
    repo.db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!(
                "SELECT {RECORD} AS record FROM runtime_control_commands c
                 WHERE c.actor_user_id=$1 AND c.idempotency_key=$2
                   AND c.session_id=$3 AND c.session_run_id=$4 LIMIT 1"
            ),
            [
                actor.user_id.into(),
                actor.idempotency_key.clone().into(),
                session.into(),
                run.into(),
            ],
        ))
        .await
        .map_err(database_error)?
        .as_ref()
        .map(receipt)
        .transpose()
}

async fn authorize(txn: &DatabaseTransaction, actor: Uuid, session: Uuid) -> Result<(), AppError> {
    let user = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT is_active,system_role FROM users WHERE id=$1 FOR NO KEY UPDATE",
            [actor.into()],
        ))
        .await
        .map_err(database_error)?
        .ok_or(AppError::Forbidden)?;
    let owner = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT user_id FROM agent_sessions WHERE id=$1",
            [session.into()],
        ))
        .await
        .map_err(database_error)?
        .ok_or_else(|| AppError::not_found("session", session))?;
    if !user
        .try_get::<bool>("", "is_active")
        .map_err(database_error)?
        || (owner
            .try_get::<Uuid>("", "user_id")
            .map_err(database_error)?
            != actor
            && !matches!(
                user.try_get::<String>("", "system_role")
                    .map_err(database_error)?
                    .as_str(),
                "admin" | "operator"
            ))
    {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

async fn current(
    txn: &DatabaseTransaction,
    expected: &SessionAgentRun,
    op: RuntimeControlOperation,
) -> Result<QueryResult, AppError> {
    // Match the terminal/dispatch lock order; key-share event writers must not deadlock.
    let agent = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT kind,status,archived_at,api_port FROM agents WHERE id=$1 FOR NO KEY UPDATE",
            [expected.agent_id.into()],
        ))
        .await
        .map_err(database_error)?
        .ok_or_else(|| AppError::not_found("agent", expected.agent_id))?;
    let session = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT agent_id FROM agent_sessions WHERE id=$1 FOR NO KEY UPDATE",
            [expected.session_id.into()],
        ))
        .await
        .map_err(database_error)?
        .ok_or_else(|| AppError::not_found("session", expected.session_id))?;
    let run = session_agent_run::Entity::find_by_id(expected.id)
        .lock_exclusive()
        .one(txn)
        .await
        .map_err(database_error)?
        .ok_or_else(|| AppError::not_found("session_agent_run", expected.id))?;
    if agent
        .try_get::<String>("", "kind")
        .map_err(database_error)?
        != "hermes"
        || agent
            .try_get::<String>("", "status")
            .map_err(database_error)?
            == "archived"
        || agent
            .try_get::<Option<shared::Timestamp>>("", "archived_at")
            .map_err(database_error)?
            .is_some()
        || session
            .try_get::<Uuid>("", "agent_id")
            .map_err(database_error)?
            != expected.agent_id
        || run.session_id != expected.session_id
        || run.agent_id != expected.agent_id
        || run.runtime_run_id != expected.runtime_run_id
        || run.runtime_session_id != expected.runtime_session_id
        || !matches!(run.state.as_str(), "running" | "waiting" | "stopping")
        || (op == RuntimeControlOperation::Steer && run.state != "running")
    {
        return Err(AppError::conflict(
            "runtime control no longer matches an active primary run",
        ));
    }
    let scoped = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM task_chat_bindings WHERE session_id=$1)
            OR EXISTS(SELECT 1 FROM pm_run_bindings WHERE session_run_id=$2) AS scoped",
            [expected.session_id.into(), expected.id.into()],
        ))
        .await
        .map_err(database_error)?
        .unwrap();
    if scoped
        .try_get::<bool>("", "scoped")
        .map_err(database_error)?
    {
        return Err(AppError::conflict("task control admission is not verified"));
    }
    let journal = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT state,request_hash,origin,credential_fingerprint,session_id,agent_id
            FROM hermes_dispatch_journal WHERE run_id=$1",
            [expected.id.into()],
        ))
        .await
        .map_err(database_error)?
        .ok_or_else(|| AppError::conflict("original runtime context is missing"))?;
    if journal
        .try_get::<String>("", "state")
        .map_err(database_error)?
        != "accepted"
        || journal
            .try_get::<Uuid>("", "session_id")
            .map_err(database_error)?
            != expected.session_id
        || journal
            .try_get::<Uuid>("", "agent_id")
            .map_err(database_error)?
            != expected.agent_id
        || expected.runtime_run_id.is_none()
        || expected.runtime_session_id.is_none()
        || journal
            .try_get::<String>("", "origin")
            .map_err(database_error)?
            != format!(
                "http://127.0.0.1:{}",
                agent
                    .try_get::<Option<i32>>("", "api_port")
                    .map_err(database_error)?
                    .ok_or_else(|| AppError::conflict("runtime control origin is missing"))?
            )
    {
        return Err(AppError::conflict(
            "original runtime context is not accepted",
        ));
    }
    Ok(journal)
}

pub(super) async fn reserve(
    repo: &PostgresFleetRepository,
    run: &SessionAgentRun,
    actor: &RuntimeControlActor,
    op: RuntimeControlOperation,
    input: Option<&str>,
) -> Result<RuntimeControlReservation, AppError> {
    if actor.user_id.is_nil()
        || !domain::valid_ref(&actor.idempotency_key, 128)
        || (op == RuntimeControlOperation::Steer
            && input.is_none_or(|s| s.trim().is_empty() || s.len() > 64 * 1024))
        || (op == RuntimeControlOperation::Stop && input.is_some())
    {
        return Err(AppError::validation("invalid runtime control command"));
    }
    let hash = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&json!({"operation":op,"input":input}))
                .map_err(AppError::internal)?
        )
    );
    let txn = repo.db.begin().await.map_err(database_error)?;
    authorize(&txn, actor.user_id, run.session_id).await?;
    if let Some(previous) = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!(
                "SELECT {RECORD} AS record,c.payload_sha256 FROM runtime_control_commands c
            WHERE c.actor_user_id=$1 AND c.idempotency_key=$2 FOR UPDATE"
            ),
            [actor.user_id.into(), actor.idempotency_key.clone().into()],
        ))
        .await
        .map_err(database_error)?
    {
        let prior = receipt(&previous)?;
        if prior.session_id != run.session_id
            || prior.session_run_id != run.id
            || prior.agent_id != run.agent_id
            || prior.operation != op
            || previous
                .try_get::<String>("", "payload_sha256")
                .map_err(database_error)?
                != hash
        {
            return Err(AppError::conflict(
                "runtime control key has a different payload or identity",
            ));
        }
        return Ok(RuntimeControlReservation {
            dispatch: prior.state == RuntimeControlState::Reserved,
            receipt: prior,
        });
    }
    let journal = current(&txn, run, op).await?;
    let id = Uuid::new_v4();
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_control_commands(id,session_id,session_run_id,agent_id,actor_user_id,operation,
            idempotency_key,payload_sha256,runtime_run_id,runtime_session_id,original_request_sha256,api_origin,credential_fingerprint)
            VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        [id.into(),run.session_id.into(),run.id.into(),run.agent_id.into(),actor.user_id.into(),op.as_str().into(),
            actor.idempotency_key.clone().into(),hash.into(),run.runtime_run_id.clone().unwrap().into(),
            run.runtime_session_id.clone().unwrap().into(),journal.try_get::<String>("","request_hash").map_err(database_error)?.into(),
            journal.try_get::<String>("","origin").map_err(database_error)?.into(),
            journal.try_get::<String>("","credential_fingerprint").map_err(database_error)?.into()]))
        .await.map_err(database_error)?;
    let result = receipt(&row(&txn, id, false).await?)?;
    audit(&txn, &result, "runtime_control.reserved").await?;
    txn.commit().await.map_err(database_error)?;
    Ok(RuntimeControlReservation {
        receipt: result,
        dispatch: true,
    })
}

pub(super) async fn claim(repo: &PostgresFleetRepository, id: Uuid) -> Result<bool, AppError> {
    let seed = receipt(&row(&repo.db, id, false).await?)?;
    let txn = repo.db.begin().await.map_err(database_error)?;
    authorize(&txn, seed.actor_user_id, seed.session_id).await?;
    if receipt(&row(&txn, id, false).await?)?.state != RuntimeControlState::Reserved {
        return Ok(false);
    }
    let expected = repo.get_session_agent_run(seed.session_run_id).await?;
    let journal = current(&txn, &expected, seed.operation).await?;
    let record = row(&txn, id, true).await?;
    let prior = receipt(&record)?;
    if prior.state != RuntimeControlState::Reserved {
        return Ok(false);
    }
    for (field, jfield) in [
        ("original_request_sha256", "request_hash"),
        ("api_origin", "origin"),
        ("credential_fingerprint", "credential_fingerprint"),
    ] {
        if record
            .try_get::<String>("", field)
            .map_err(database_error)?
            != journal
                .try_get::<String>("", jfield)
                .map_err(database_error)?
        {
            return Err(AppError::conflict(
                "runtime control original context changed",
            ));
        }
    }
    if record
        .try_get::<String>("", "runtime_run_id")
        .map_err(database_error)?
        .as_str()
        != expected.runtime_run_id.as_deref().unwrap_or("")
        || record
            .try_get::<String>("", "runtime_session_id")
            .map_err(database_error)?
            .as_str()
            != expected.runtime_session_id.as_deref().unwrap_or("")
    {
        return Err(AppError::conflict("runtime control native pin changed"));
    }
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE runtime_control_commands SET state='submitted',updated_at=now() WHERE id=$1",
        [id.into()],
    ))
    .await
    .map_err(database_error)?;
    audit(&txn, &prior, "runtime_control.submitted").await?;
    txn.commit().await.map_err(database_error)?;
    Ok(true)
}

pub(super) async fn finish(
    repo: &PostgresFleetRepository,
    id: Uuid,
    ack: &str,
    input: Option<&str>,
) -> Result<RuntimeControlReceipt, AppError> {
    let seed = receipt(&row(&repo.db, id, false).await?)?;
    if !matches!(
        (seed.operation, ack),
        (RuntimeControlOperation::Steer, "steered")
            | (
                RuntimeControlOperation::Stop,
                "stopping" | "already_terminal"
            )
    ) {
        return Err(AppError::validation(
            "invalid runtime control acknowledgement",
        ));
    }
    let txn = repo.db.begin().await.map_err(database_error)?;
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id=$1 FOR NO KEY UPDATE",
        [seed.agent_id.into()],
    ))
    .await
    .map_err(database_error)?;
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agent_sessions WHERE id=$1 FOR NO KEY UPDATE",
        [seed.session_id.into()],
    ))
    .await
    .map_err(database_error)?;
    let run = session_agent_run::Entity::find_by_id(seed.session_run_id)
        .lock_exclusive()
        .one(&txn)
        .await
        .map_err(database_error)?
        .ok_or_else(|| AppError::not_found("session_agent_run", seed.session_run_id))?;
    let record = row(&txn, id, true).await?;
    let prior = receipt(&record)?;
    let hash = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&json!({"operation":prior.operation,"input":input}))
                .map_err(AppError::internal)?
        )
    );
    if record
        .try_get::<String>("", "payload_sha256")
        .map_err(database_error)?
        != hash
    {
        return Err(AppError::conflict("runtime control ACK payload changed"));
    }
    if prior.state == RuntimeControlState::Acknowledged
        && prior.acknowledgement.as_deref() == Some(ack)
    {
        mirror_steer(&txn, &prior, input, &hash).await?;
        txn.commit().await.map_err(database_error)?;
        return Ok(prior);
    }
    if prior.state != RuntimeControlState::Submitted {
        return Err(AppError::conflict("runtime control is not awaiting an ACK"));
    }
    if run.session_id != seed.session_id
        || run.agent_id != seed.agent_id
        || run.runtime_run_id.as_deref()
            != Some(
                record
                    .try_get::<String>("", "runtime_run_id")
                    .map_err(database_error)?
                    .as_str(),
            )
        || run.runtime_session_id.as_deref()
            != Some(
                record
                    .try_get::<String>("", "runtime_session_id")
                    .map_err(database_error)?
                    .as_str(),
            )
    {
        return Err(AppError::conflict(
            "runtime control changed before ACK commit",
        ));
    }
    if ack == "stopping" && matches!(run.state.as_str(), "running" | "waiting" | "stopping") {
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE session_agent_runs SET state='stopping',updated_at=now() WHERE id=$1",
            [run.id.into()],
        ))
        .await
        .map_err(database_error)?;
    }
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_control_commands SET state='acknowledged',acknowledgement=$2,updated_at=now() WHERE id=$1",
        [id.into(),ack.into()])).await.map_err(database_error)?;
    mirror_steer(&txn, &prior, input, &hash).await?;
    audit(&txn, &prior, "runtime_control.acknowledged").await?;
    let result = receipt(&row(&txn, id, false).await?)?;
    txn.commit().await.map_err(database_error)?;
    Ok(result)
}

async fn mirror_steer(
    txn: &DatabaseTransaction,
    command: &RuntimeControlReceipt,
    input: Option<&str>,
    hash: &str,
) -> Result<(), AppError> {
    if command.operation != RuntimeControlOperation::Steer {
        return Ok(());
    }
    let body = redact_text(
        input
            .ok_or_else(|| AppError::validation("steer input is required"))?
            .trim(),
    );
    let runtime_id = format!(
        "fleet-control:{}:{}:steer",
        command.session_run_id, command.id
    );
    // The receipt primary key is also the mirror key; never overwrite an unrelated message.
    if let Some(existing) = session_message::Entity::find_by_id(command.id)
        .one(txn)
        .await
        .map_err(database_error)?
    {
        if existing.session_id != command.session_id
            || existing.author_type != "user"
            || existing.author_user_id != Some(command.actor_user_id)
            || existing.author_agent_id.is_some()
            || existing.created_by_user_id != Some(command.actor_user_id)
            || existing.message_kind != "control"
            || existing.delivery_state != "mirrored"
            || existing.delivery_error.is_some()
            || existing.runtime_message_id.as_deref() != Some(runtime_id.as_str())
            || existing.idempotency_key.is_some()
            || existing.idempotency_payload_hash.as_deref() != Some(hash)
            || existing.body != body
        {
            return Err(AppError::conflict(
                "runtime control transcript identity changed",
            ));
        }
        return Ok(());
    }
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO session_messages(id,session_id,author_type,author_user_id,body,message_kind,
            runtime_message_id,created_by_user_id,idempotency_payload_hash,delivery_state)
         VALUES($1,$2,'user',$3,$4,'control',$5,$3,$6,'mirrored')",
        [
            command.id.into(),
            command.session_id.into(),
            command.actor_user_id.into(),
            body.clone().into(),
            runtime_id.into(),
            hash.into(),
        ],
    ))
    .await
    .map_err(database_error)?;
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agent_sessions SET last_message_preview=$2,updated_at=now() WHERE id=$1",
        [
            command.session_id.into(),
            body.chars().take(180).collect::<String>().into(),
        ],
    ))
    .await
    .map_err(database_error)?;
    Ok(())
}

pub(super) async fn retire(
    repo: &PostgresFleetRepository,
    id: Uuid,
    submitted: bool,
) -> Result<RuntimeControlReceipt, AppError> {
    let txn = repo.db.begin().await.map_err(database_error)?;
    let prior = receipt(&row(&txn, id, true).await?)?;
    let (expected, state) = if submitted {
        (RuntimeControlState::Submitted, "uncertain")
    } else {
        (RuntimeControlState::Reserved, "rejected")
    };
    if prior.state != expected {
        return Ok(prior);
    }
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE runtime_control_commands SET state=$2,updated_at=now() WHERE id=$1",
        [id.into(), state.into()],
    ))
    .await
    .map_err(database_error)?;
    audit(
        &txn,
        &prior,
        if submitted {
            "runtime_control.uncertain"
        } else {
            "runtime_control.not_dispatched"
        },
    )
    .await?;
    let result = receipt(&row(&txn, id, false).await?)?;
    txn.commit().await.map_err(database_error)?;
    Ok(result)
}

pub(super) async fn reconcile(repo: &PostgresFleetRepository) -> Result<u64, AppError> {
    // Only an independently committed terminal mirror releases an unresolved control hold.
    // This does NOT prove whether guidance/interrupt was accepted; that fact stays unknown.
    let ids = repo.db.query_all(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT c.id,c.session_id,c.session_run_id FROM runtime_control_commands c
         JOIN session_agent_runs r ON r.id=c.session_run_id
         JOIN hermes_dispatch_journal j ON j.run_id=r.id AND j.state='accepted'
         JOIN session_messages m ON m.id=j.message_id AND m.runtime_message_id=r.runtime_run_id
         WHERE c.state IN ('reserved','submitted','uncertain') AND r.state IN ('completed','failed','cancelled')
            AND r.last_event_at IS NOT NULL
            AND m.delivery_state=CASE WHEN r.state='failed' THEN 'failed' ELSE 'completed' END
         ORDER BY c.id LIMIT 100".to_owned())).await.map_err(database_error)?;
    let mut changed = 0;
    for seed in ids {
        let id: Uuid = seed.try_get("", "id").map_err(database_error)?;
        let run_id: Uuid = seed.try_get("", "session_run_id").map_err(database_error)?;
        let txn = repo.db.begin().await.map_err(database_error)?;
        let run = session_agent_run::Entity::find_by_id(run_id)
            .lock_exclusive()
            .one(&txn)
            .await
            .map_err(database_error)?
            .ok_or_else(|| AppError::not_found("session_agent_run", run_id))?;
        let proof = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM hermes_dispatch_journal j JOIN session_messages m ON m.id=j.message_id
                WHERE j.run_id=$1 AND j.state='accepted' AND m.runtime_message_id=$2
                AND m.delivery_state=$3) AS valid",
            [run_id.into(),run.runtime_run_id.clone().into(),if run.state == "failed" {"failed"} else {"completed"}.into()]))
            .await.map_err(database_error)?.unwrap();
        if run.last_event_at.is_none()
            || !proof.try_get::<bool>("", "valid").map_err(database_error)?
        {
            continue;
        }
        let record = row(&txn, id, true).await?;
        let prior = receipt(&record)?;
        if !matches!(
            prior.state,
            RuntimeControlState::Reserved
                | RuntimeControlState::Submitted
                | RuntimeControlState::Uncertain
        ) || !matches!(run.state.as_str(), "completed" | "failed" | "cancelled")
            || run.runtime_run_id.as_deref()
                != Some(
                    record
                        .try_get::<String>("", "runtime_run_id")
                        .map_err(database_error)?
                        .as_str(),
                )
            || run.runtime_session_id.as_deref()
                != Some(
                    record
                        .try_get::<String>("", "runtime_session_id")
                        .map_err(database_error)?
                        .as_str(),
                )
        {
            continue;
        }
        if prior.state == RuntimeControlState::Reserved {
            txn.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE runtime_control_commands SET state='rejected',updated_at=now() WHERE id=$1",
                [id.into()],
            ))
            .await
            .map_err(database_error)?;
        } else {
            txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "UPDATE runtime_control_commands SET state='terminal_observed',observed_run_state=$2,updated_at=now() WHERE id=$1",
                [id.into(),run.state.into()])).await.map_err(database_error)?;
        }
        audit(&txn, &prior, "runtime_control.terminal_observed").await?;
        txn.commit().await.map_err(database_error)?;
        changed += 1;
    }
    Ok(changed)
}

async fn audit<C: ConnectionTrait>(
    db: &C,
    receipt: &RuntimeControlReceipt,
    action: &str,
) -> Result<(), AppError> {
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO audit_log(id,actor_user_id,action,entity_type,entity_id,payload,created_at)
         VALUES($1,$2,$3,'runtime_control_command',$4,$5,now())",
        [Uuid::new_v4().into(),receipt.actor_user_id.into(),action.into(),receipt.id.to_string().into(),
            json!({"session_id":receipt.session_id,"run_id":receipt.session_run_id,"operation":receipt.operation}).into()]))
        .await.map_err(database_error)?;
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "WITH cursor AS (INSERT INTO session_event_cursors(session_id,sequence) VALUES($1,1)
            ON CONFLICT(session_id) DO UPDATE SET sequence=session_event_cursors.sequence+1 RETURNING sequence)
         INSERT INTO session_events(session_id,sequence,event_type,payload)
            SELECT $1,sequence,'runtime_control.changed',$2 FROM cursor",
        [receipt.session_id.into(),json!({"type":"runtime_control_changed","command_id":receipt.id,
            "run_id":receipt.session_run_id,"action":action}).into()])).await.map_err(database_error)?;
    Ok(())
}
