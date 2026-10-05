use super::*;
use app::ApprovalOutcomeIntent;
use domain::{ApprovalDecision, ApprovalDecisionState};
use sea_orm::DatabaseTransaction;

fn failure(_: sea_orm::DbErr) -> AppError {
    AppError::Database("approval outcome journal operation failed".into())
}

async fn decision<C: ConnectionTrait>(db: &C, id: Uuid) -> Result<ApprovalDecision, AppError> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT session_id,approval_id FROM runtime_approval_decisions WHERE id=$1",
            [id.into()],
        ))
        .await
        .map_err(failure)?
        .ok_or_else(|| AppError::not_found("approval_decision", id))?;
    approval_decisions::load(
        db,
        row.try_get("", "session_id").map_err(failure)?,
        row.try_get("", "approval_id").map_err(failure)?,
    )
    .await
}

async fn approval<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
) -> Result<RuntimeApprovalRequest, AppError> {
    runtime_approval_request::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(failure)?
        .map(runtime_approval_from_model)
        .ok_or_else(|| AppError::not_found("runtime_approval_request", id))
}

pub(super) async fn get(
    repo: &PostgresFleetRepository,
    id: Uuid,
) -> Result<Option<ApprovalOutcomeIntent>, AppError> {
    let Some(row) = repo
        .db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT context FROM runtime_approval_outcomes WHERE decision_id=$1",
            [id.into()],
        ))
        .await
        .map_err(failure)?
    else {
        return Ok(None);
    };
    let decision = decision(&repo.db, id).await?;
    Ok(Some(ApprovalOutcomeIntent {
        approval: approval(&repo.db, decision.approval_id).await?,
        decision,
        context: row.try_get("", "context").map_err(failure)?,
    }))
}

pub(super) async fn list(
    repo: &PostgresFleetRepository,
    after: Option<Uuid>,
) -> Result<Vec<ApprovalOutcomeIntent>, AppError> {
    let rows = repo.db.query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT o.decision_id FROM runtime_approval_outcomes o JOIN runtime_approval_decisions d ON d.id=o.decision_id
         WHERE o.state='submitted' AND d.state='uncertain' AND d.outcome_required AND d.submission_claimed
            AND ($1::uuid IS NULL OR o.decision_id>$1) ORDER BY o.decision_id LIMIT 100", [after.into()]))
        .await.map_err(failure)?;
    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        if let Some(intent) = get(repo, row.try_get("", "decision_id").map_err(failure)?).await? {
            result.push(intent);
        }
    }
    Ok(result)
}

async fn lock_session(txn: &DatabaseTransaction, session: Uuid) -> Result<Uuid, AppError> {
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT user_id FROM agent_sessions WHERE id=$1 FOR NO KEY UPDATE",
        [session.into()],
    ))
    .await
    .map_err(failure)?
    .ok_or_else(|| AppError::not_found("session", session))?
    .try_get("", "user_id")
    .map_err(failure)
}

pub(super) async fn claim(
    repo: &PostgresFleetRepository,
    id: Uuid,
    context: Value,
) -> Result<bool, AppError> {
    let typed: runtime::control_outcome_wire::Context = serde_json::from_value(context.clone())
        .map_err(|_| AppError::conflict("invalid original approval context"))?;
    let txn = repo.db.begin().await.map_err(failure)?;
    let d = decision(&txn, id).await?;
    let a = approval(&txn, d.approval_id).await?;
    let user = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT is_active,system_role FROM users WHERE id=$1 FOR NO KEY UPDATE",
            [d.actor_user_id.into()],
        ))
        .await
        .map_err(failure)?
        .ok_or(AppError::Forbidden)?;
    let agent = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT kind,status FROM agents WHERE id=$1 FOR NO KEY UPDATE",
            [a.agent_id.into()],
        ))
        .await
        .map_err(failure)?
        .ok_or_else(|| AppError::not_found("agent", a.agent_id))?;
    let owner = lock_session(&txn, d.session_id).await?;
    let primary: Uuid = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT agent_id FROM agent_sessions WHERE id=$1",
            [d.session_id.into()],
        ))
        .await
        .map_err(failure)?
        .ok_or_else(|| AppError::not_found("session", d.session_id))?
        .try_get("", "agent_id")
        .map_err(failure)?;
    if !user.try_get::<bool>("", "is_active").map_err(failure)?
        || (owner != d.actor_user_id
            && !matches!(
                user.try_get::<String>("", "system_role")
                    .map_err(failure)?
                    .as_str(),
                "admin" | "operator"
            ))
    {
        return Err(AppError::Forbidden);
    }
    let run = session_agent_run::Entity::find_by_id(d.session_run_id)
        .lock_exclusive()
        .one(&txn)
        .await
        .map_err(failure)?
        .ok_or_else(|| AppError::not_found("session_agent_run", d.session_run_id))?;
    runtime_approval_request::Entity::find_by_id(d.approval_id)
        .lock_exclusive()
        .one(&txn)
        .await
        .map_err(failure)?;
    let mode = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT outcome_required,submission_claimed FROM runtime_approval_decisions WHERE id=$1 FOR UPDATE", [id.into()]))
        .await.map_err(failure)?.ok_or_else(|| AppError::not_found("approval_decision", id))?;
    let d = decision(&txn, id).await?;
    let a = approval(&txn, d.approval_id).await?;
    let journal = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT session_id,agent_id,state,origin,credential_fingerprint,submitted_at FROM hermes_dispatch_journal WHERE run_id=$1",
        [d.session_run_id.into()])).await.map_err(failure)?
        .ok_or_else(|| AppError::conflict("original accepted approval run is missing"))?;
    typed.verify_approval(
        &d,
        &a,
        &journal.try_get::<String>("", "origin").map_err(failure)?,
        &journal
            .try_get::<String>("", "credential_fingerprint")
            .map_err(failure)?,
    )?;
    if !mode
        .try_get::<bool>("", "outcome_required")
        .map_err(failure)?
    {
        return Err(AppError::conflict(
            "legacy approval cannot acquire an original outcome",
        ));
    }
    if mode
        .try_get::<bool>("", "submission_claimed")
        .map_err(failure)?
    {
        let original: Value = txn
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT context FROM runtime_approval_outcomes WHERE decision_id=$1",
                [id.into()],
            ))
            .await
            .map_err(failure)?
            .ok_or_else(|| AppError::conflict("original approval context is missing"))?
            .try_get("", "context")
            .map_err(failure)?;
        if original != context {
            return Err(AppError::conflict(
                "original approval context cannot change",
            ));
        }
        return Ok(false);
    }
    let scoped: bool = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM task_chat_bindings WHERE session_id=$1)
            OR EXISTS(SELECT 1 FROM pm_run_bindings WHERE session_run_id=$2) AS scoped",
            [d.session_id.into(), d.session_run_id.into()],
        ))
        .await
        .map_err(failure)?
        .unwrap()
        .try_get("", "scoped")
        .map_err(failure)?;
    if scoped
        || d.state != ApprovalDecisionState::Uncertain
        || a.state != RuntimeApprovalState::Pending
        || agent.try_get::<String>("", "kind").map_err(failure)? != "hermes"
        || agent.try_get::<String>("", "status").map_err(failure)? == "archived"
        || primary != a.agent_id
        || run.session_id != d.session_id
        || run.agent_id != a.agent_id
        || run.runtime_run_id.as_deref() != Some(a.runtime_run_id.as_str())
        || run.runtime_session_id.is_none()
        || !matches!(run.state.as_str(), "running" | "waiting")
        || journal.try_get::<String>("", "state").map_err(failure)? != "accepted"
        || journal.try_get::<Uuid>("", "session_id").map_err(failure)? != d.session_id
        || journal.try_get::<Uuid>("", "agent_id").map_err(failure)? != a.agent_id
        || journal
            .try_get::<Option<shared::Timestamp>>("", "submitted_at")
            .map_err(failure)?
            .is_none()
    {
        return Err(AppError::conflict(
            "approval no longer belongs to an accepted active free chat",
        ));
    }
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE runtime_approval_decisions SET submission_claimed=true WHERE id=$1",
        [id.into()],
    ))
    .await
    .map_err(failure)?;
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO runtime_approval_outcomes(decision_id,context) VALUES($1,$2)",
        [id.into(), context.into()],
    ))
    .await
    .map_err(failure)?;
    approval_decisions::audit(&txn, d.actor_user_id, id, "approval.decision_submitted",
        json!({"approval_id":d.approval_id,"session_id":d.session_id,"run_id":d.session_run_id,"choice":d.choice})).await?;
    txn.commit().await.map_err(failure)?;
    Ok(true)
}

pub(super) async fn finish(
    repo: &PostgresFleetRepository,
    id: Uuid,
    context: Value,
) -> Result<ApprovalDecision, AppError> {
    let txn = repo.db.begin().await.map_err(failure)?;
    let refs = decision(&txn, id).await?;
    lock_session(&txn, refs.session_id).await?;
    session_agent_run::Entity::find_by_id(refs.session_run_id)
        .lock_exclusive()
        .one(&txn)
        .await
        .map_err(failure)?;
    runtime_approval_request::Entity::find_by_id(refs.approval_id)
        .lock_exclusive()
        .one(&txn)
        .await
        .map_err(failure)?;
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM runtime_approval_decisions WHERE id=$1 FOR UPDATE",
        [id.into()],
    ))
    .await
    .map_err(failure)?;
    let row = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT context,state FROM runtime_approval_outcomes WHERE decision_id=$1 FOR UPDATE",
            [id.into()],
        ))
        .await
        .map_err(failure)?
        .ok_or_else(|| AppError::conflict("original approval context is missing"))?;
    let original: Value = row.try_get("", "context").map_err(failure)?;
    if original != context {
        return Err(AppError::conflict(
            "approval ACK does not match original context",
        ));
    }
    let d = decision(&txn, id).await?;
    if d.state == ApprovalDecisionState::Delivered
        && row.try_get::<String>("", "state").map_err(failure)? == "acknowledged"
    {
        return Ok(d);
    }
    if d.state != ApprovalDecisionState::Uncertain {
        return Err(AppError::conflict("approval outcome is not pending"));
    }
    // A terminal observer may have cancelled the request; that history is not reopened by a late ACK.
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_approval_requests SET state=$2,resolved_by_user_id=$3,resolved_at=now() WHERE id=$1 AND state='pending'",
        [d.approval_id.into(), if d.choice==domain::ApprovalChoice::Once {"approved"} else {"denied"}.into(),d.actor_user_id.into()]))
        .await.map_err(failure)?;
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_approval_outcomes SET state='acknowledged',acknowledged_at=now() WHERE decision_id=$1", [id.into()]))
        .await.map_err(failure)?;
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE runtime_approval_decisions SET state='delivered',delivered_at=now() WHERE id=$1",
        [id.into()],
    ))
    .await
    .map_err(failure)?;
    approval_decisions::audit(&txn, d.actor_user_id, id, "approval.decision_delivered",
        json!({"approval_id":d.approval_id,"session_id":d.session_id,"run_id":d.session_run_id,"choice":d.choice})).await?;
    let result = decision(&txn, id).await?;
    txn.commit().await.map_err(failure)?;
    Ok(result)
}
