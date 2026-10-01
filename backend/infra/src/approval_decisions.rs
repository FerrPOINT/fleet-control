use super::*;
use domain::{ApprovalDecision, ApprovalDecisionRequest, ReservedApprovalDecision};

async fn load<C: ConnectionTrait>(
    db: &C,
    session: Uuid,
    approval: Uuid,
) -> Result<ApprovalDecision, AppError> {
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('id',id,'session_id',session_id,'approval_id',approval_id,
            'session_run_id',session_run_id,'actor_user_id',actor_user_id,'choice',choice,'state',state,
            'created_at',created_at) AS record FROM runtime_approval_decisions WHERE session_id=$1 AND approval_id=$2",
        [session.into(), approval.into()])).await.map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("approval_decision", approval))?;
    serde_json::from_value(
        row.try_get::<Value>("", "record")
            .map_err(AppError::database)?,
    )
    .map_err(AppError::internal)
}

pub(super) async fn get(
    repo: &PostgresFleetRepository,
    session: Uuid,
    approval: Uuid,
) -> Result<ApprovalDecision, AppError> {
    load(&repo.db, session, approval).await
}

pub(super) async fn list(
    repo: &PostgresFleetRepository,
    session: Uuid,
) -> Result<Vec<RuntimeApprovalRequest>, AppError> {
    let rows = runtime_approval_request::Entity::find()
        .filter(runtime_approval_request::Column::SessionId.eq(session))
        .order_by_asc(runtime_approval_request::Column::CreatedAt)
        .order_by_asc(runtime_approval_request::Column::Id)
        .all(&repo.db)
        .await
        .map_err(AppError::database)?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let mut approval = runtime_approval_from_model(row);
            approval.prompt = redact_text(&approval.prompt);
            approval.detail = redact_json(approval.detail);
            approval
        })
        .collect())
}

pub(super) async fn reserve(
    repo: &PostgresFleetRepository,
    session: Uuid,
    approval_id: Uuid,
    actor: Uuid,
    req: ApprovalDecisionRequest,
) -> Result<ReservedApprovalDecision, AppError> {
    req.validate()?;
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    // Serialize a user's keys across approvals and recheck current authorization in the transaction.
    let user = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT is_active,system_role FROM users WHERE id=$1 FOR NO KEY UPDATE",
            [actor.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or(AppError::Forbidden)?;
    let owner = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT user_id FROM agent_sessions WHERE id=$1",
            [session.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("session", session))?;
    let role: String = user
        .try_get("", "system_role")
        .map_err(AppError::database)?;
    if !user
        .try_get::<bool>("", "is_active")
        .map_err(AppError::database)?
        || (owner
            .try_get::<Uuid>("", "user_id")
            .map_err(AppError::database)?
            != actor
            && !matches!(role.as_str(), "admin" | "operator"))
    {
        return Err(AppError::Forbidden);
    }
    let row = runtime_approval_request::Entity::find_by_id(approval_id)
        .filter(runtime_approval_request::Column::SessionId.eq(session))
        .lock_exclusive()
        .one(&txn)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("runtime_approval_request", approval_id))?;
    let approval = runtime_approval_from_model(row);
    if let Some(previous) = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT approval_id,session_id,choice,idempotency_key,actor_user_id FROM runtime_approval_decisions
          WHERE approval_id=$1 OR (actor_user_id=$2 AND idempotency_key=$3)",
        [approval_id.into(), actor.into(), req.idempotency_key.clone().into()])).await.map_err(AppError::database)? {
        if previous.try_get::<Uuid>("", "approval_id").map_err(AppError::database)? != approval_id
            || previous.try_get::<Uuid>("", "session_id").map_err(AppError::database)? != session
            || previous.try_get::<Uuid>("", "actor_user_id").map_err(AppError::database)? != actor
            || previous.try_get::<String>("", "choice").map_err(AppError::database)? != req.choice.as_str()
            || previous.try_get::<String>("", "idempotency_key").map_err(AppError::database)? != req.idempotency_key {
            return Err(AppError::conflict("approval already has a different decision or command key"));
        }
        return Ok(ReservedApprovalDecision { decision:load(&txn, session, approval_id).await?, approval, dispatch:false });
    }
    let request_id = approval
        .runtime_approval_id
        .as_deref()
        .ok_or_else(|| AppError::conflict("approval has no exact runtime request ID"))?;
    if !domain::valid_ref(request_id, 256)
        || approval.state != domain::RuntimeApprovalState::Pending
    {
        return Err(AppError::conflict(
            "approval is not pending or its request ID is invalid",
        ));
    }
    let run = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT session_id,agent_id,runtime_run_id,state FROM session_agent_runs WHERE id=$1 FOR UPDATE",
        [approval.session_run_id.into()])).await.map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("session_agent_run", approval.session_run_id))?;
    if run
        .try_get::<Uuid>("", "session_id")
        .map_err(AppError::database)?
        != session
        || run
            .try_get::<Uuid>("", "agent_id")
            .map_err(AppError::database)?
            != approval.agent_id
        || run
            .try_get::<Option<String>>("", "runtime_run_id")
            .map_err(AppError::database)?
            .as_deref()
            != Some(approval.runtime_run_id.as_str())
        || !matches!(
            run.try_get::<String>("", "state")
                .map_err(AppError::database)?
                .as_str(),
            "running" | "waiting"
        )
    {
        return Err(AppError::conflict(
            "approval does not belong to an active runtime run",
        ));
    }
    let id = Uuid::new_v4();
    // Unknown acceptance is durable BEFORE HTTP. A replay never dispatches again.
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_approval_decisions(id,session_id,approval_id,session_run_id,actor_user_id,choice,idempotency_key,state)
          VALUES($1,$2,$3,$4,$5,$6,$7,'uncertain')", [id.into(),session.into(),approval_id.into(),approval.session_run_id.into(),actor.into(),req.choice.as_str().into(),req.idempotency_key.into()]))
        .await.map_err(AppError::database)?;
    audit(&txn, actor, id, "approval.decision_reserved", json!({"approval_id":approval_id,"session_id":session,"run_id":approval.session_run_id,"choice":req.choice})).await?;
    let decision = load(&txn, session, approval_id).await?;
    txn.commit().await.map_err(AppError::database)?;
    Ok(ReservedApprovalDecision {
        decision,
        approval,
        dispatch: true,
    })
}

pub(super) async fn deliver(
    repo: &PostgresFleetRepository,
    id: Uuid,
) -> Result<ApprovalDecision, AppError> {
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let refs = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT session_id,approval_id FROM runtime_approval_decisions WHERE id=$1",
            [id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("approval_decision", id))?;
    let session: Uuid = refs.try_get("", "session_id").map_err(AppError::database)?;
    let approval: Uuid = refs
        .try_get("", "approval_id")
        .map_err(AppError::database)?;
    let row = runtime_approval_request::Entity::find_by_id(approval)
        .lock_exclusive()
        .one(&txn)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("runtime_approval_request", approval))?;
    let previous = load(&txn, session, approval).await?;
    if previous.state == domain::ApprovalDecisionState::Delivered {
        return Ok(previous);
    }
    if row.state != "pending" {
        return Err(AppError::conflict(
            "approval was settled by a different control path",
        ));
    }
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_approval_requests SET state=$2,resolved_by_user_id=$3,resolved_at=now() WHERE id=$1",
        [approval.into(), if previous.choice == domain::ApprovalChoice::Once {"approved"} else {"denied"}.into(), previous.actor_user_id.into()])).await.map_err(AppError::database)?;
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE runtime_approval_decisions SET state='delivered',delivered_at=now() WHERE id=$1",
        [id.into()],
    ))
    .await
    .map_err(AppError::database)?;
    audit(&txn, previous.actor_user_id, id, "approval.decision_delivered", json!({"approval_id":approval,"session_id":session,"run_id":previous.session_run_id,"choice":previous.choice})).await?;
    let result = load(&txn, session, approval).await?;
    txn.commit().await.map_err(AppError::database)?;
    Ok(result)
}

async fn audit<C: ConnectionTrait>(
    db: &C,
    actor: Uuid,
    id: Uuid,
    action: &str,
    payload: Value,
) -> Result<(), AppError> {
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO audit_log(id,actor_user_id,action,entity_type,entity_id,payload,created_at)
         VALUES($1,$2,$3,'approval_decision',$4,$5,now())",
        [
            Uuid::new_v4().into(),
            actor.into(),
            action.into(),
            id.to_string().into(),
            payload.into(),
        ],
    ))
    .await
    .map_err(AppError::database)?;
    Ok(())
}
