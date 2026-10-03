use crate::middleware::CurrentUser;
use app::AppContext;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::HeaderMap,
};
use domain::{ApprovalDecision, ApprovalDecisionRequest, RuntimeApprovalRequest};
use shared::AppError;
use std::sync::Arc;
use uuid::Uuid;

async fn access(
    ctx: &Arc<AppContext>,
    user: &CurrentUser,
    session: Uuid,
    headers: &HeaderMap,
) -> Result<Option<domain::TrackerTaskContext>, AppError> {
    let chat = ctx.repo.get_session(session).await?;
    if chat.user_id != user.id && !user.can_operate_fleet() {
        return Err(AppError::Forbidden);
    }
    if ctx.repo.get_task_chat_binding(session).await?.is_some() {
        return Ok(
            super::task_chats::load_task_context(ctx, user, session, headers, false)
                .await?
                .tracker,
        );
    }
    Ok(None)
}

#[utoipa::path(get, path="/api/v1/sessions/{session_id}/approvals", tag="sessions", params(("session_id"=Uuid,Path)), responses((status=200,body=Vec<RuntimeApprovalRequest>)))]
pub async fn list(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(session): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<RuntimeApprovalRequest>>, AppError> {
    access(&ctx, &user, session, &headers).await?;
    Ok(Json(ctx.repo.list_session_approvals(session).await?))
}

#[utoipa::path(get, path="/api/v1/sessions/{session_id}/approvals/{approval_id}/decision", tag="sessions", params(("session_id"=Uuid,Path),("approval_id"=Uuid,Path)), responses((status=200,body=ApprovalDecision),(status=404)))]
pub async fn read(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path((session, approval)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<ApprovalDecision>, AppError> {
    access(&ctx, &user, session, &headers).await?;
    Ok(Json(ctx.repo.approval_decision(session, approval).await?))
}

#[utoipa::path(post, path="/api/v1/sessions/{session_id}/approvals/{approval_id}/decision", tag="sessions", params(("session_id"=Uuid,Path),("approval_id"=Uuid,Path)), request_body=ApprovalDecisionRequest, responses((status=200,body=ApprovalDecision),(status=409)))]
pub async fn decide(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path((session, approval)): Path<(Uuid, Uuid)>,
    human: Option<Extension<crate::middleware::VerifiedHumanSession>>,
    headers: HeaderMap,
    Json(req): Json<ApprovalDecisionRequest>,
) -> Result<Json<ApprovalDecision>, AppError> {
    if human.is_none() {
        return Err(AppError::Forbidden);
    }
    let context = access(&ctx, &user, session, &headers).await?;
    if let Some(context) = context {
        match ctx.repo.approval_decision(session, approval).await {
            Ok(_) => (), // Historical commands may be replayed after assignment changes, never redispatched.
            Err(AppError::NotFound(_)) => {
                let target = ctx
                    .repo
                    .list_session_approvals(session)
                    .await?
                    .into_iter()
                    .find(|item| item.id == approval)
                    .ok_or_else(|| AppError::not_found("runtime_approval_request", approval))?;
                let record = ctx.repo.get_pm_run(target.session_run_id).await?;
                ensure_current_assignment(&context, &record)?;
            }
            Err(error) => return Err(error),
        }
    }
    let reserved = ctx
        .repo
        .reserve_approval_decision(session, approval, user.id, req)
        .await?;
    if !reserved.dispatch {
        return Ok(Json(reserved.decision));
    }
    let run = ctx
        .repo
        .get_session_agent_run(reserved.approval.session_run_id)
        .await?;
    let agent = ctx.repo.get_agent(reserved.approval.agent_id).await?;
    // Reservation may have waited for a lock. Revalidate remote permissions and
    // the exact assignment after that wait, immediately before any runtime side effect.
    let revalidation = async {
        if let Some(context) = access(&ctx, &user, session, &headers).await? {
            let record = ctx
                .repo
                .get_pm_run(reserved.approval.session_run_id)
                .await?;
            ensure_current_assignment(&context, &record)?;
        }
        Ok::<(), AppError>(())
    }
    .await;
    if let Err(error) = revalidation {
        ctx.repo
            .fail_undispatched_approval_decision(reserved.decision.id)
            .await?;
        return Err(error);
    }
    // The runtime does not expose idempotent approval commands. Lost acceptance stays
    // uncertain; neither reconnect nor a repeated POST is permission to send again.
    if ctx
        .runtime
        .resolve_targeted_approval(&agent, &run, &reserved.approval, reserved.decision.choice)
        .await
        .is_err()
    {
        return Ok(Json(reserved.decision));
    }
    Ok(Json(
        ctx.repo
            .deliver_approval_decision(reserved.decision.id)
            .await?,
    ))
}

fn ensure_current_assignment(
    context: &domain::TrackerTaskContext,
    record: &domain::PmRunRecord,
) -> Result<(), AppError> {
    let identity = &record.reservation.identity;
    if record.terminal_status.is_some() {
        return Err(AppError::conflict("PM run is already terminal"));
    }
    let assignment = context
        .assignment
        .as_ref()
        .ok_or_else(|| AppError::conflict("task has no current PM assignment"))?;
    if identity.assignment_ref != assignment.assignment_id.to_string()
        || identity.tracker_instance_ref != context.tracker_instance_id
        || identity.tracker_project_ref != context.project_id.to_string()
        || identity.task_ref != context.task_id.to_string()
        || identity.root_ref != context.root_task_id.to_string()
        || identity.execution_ref != assignment.execution_id.to_string()
        || identity.agent_ref != assignment.agent_id.to_string()
        || identity.assignment_revision != assignment.version
    {
        return Err(AppError::conflict(
            "approval belongs to a stale PM assignment",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn task_approval_requires_current_nonterminal_assignment() {
        let task = Uuid::new_v4();
        let agent = Uuid::new_v4();
        let assignment = Uuid::new_v4();
        let execution = Uuid::new_v4();
        let context = domain::TrackerTaskContext {
            contract_version: 1,
            tracker_instance_id: "tracker".into(),
            project_id: Uuid::new_v4(),
            task_id: task,
            root_task_id: task,
            owner_subject: "owner".into(),
            stage: domain::TrackerStage::Draft,
            requirement_revision: None,
            waiting_reason: None,
            permissions: domain::TrackerPermissions {
                can_answer: false,
                can_confirm: false,
            },
            assignment: Some(domain::TrackerPmAssignment {
                assignment_id: assignment,
                execution_id: execution,
                agent_id: agent,
                version: 1,
                machine_subject: "machine".into(),
            }),
        };
        let record = domain::PmRunRecord {
            reservation: domain::PmRunReservation {
                session_id: Uuid::new_v4(),
                session_run_id: Uuid::new_v4(),
                identity: domain::PmExecutionIdentity {
                    task: "SDLC-1".into(),
                    execution_ref: execution.to_string(),
                    tracker_instance_ref: context.tracker_instance_id.clone(),
                    tracker_project_ref: context.project_id.to_string(),
                    task_ref: task.to_string(),
                    root_ref: task.to_string(),
                    agent_ref: agent.to_string(),
                    assignment_operation_key: "assign".into(),
                    assignment_ref: assignment.to_string(),
                    assignment_revision: 1,
                },
                binding_ref: "binding".into(),
                dispatch_operation_key: "dispatch".into(),
                checkpoint_ref: None,
                fence: 1,
            },
            hermes_run_ref: Some("run_one".into()),
            hermes_session_ref: Some("session_one".into()),
            terminal_status: None,
        };
        assert!(ensure_current_assignment(&context, &record).is_ok());
        let mut stale = context.clone();
        stale.assignment.as_mut().unwrap().version = 2;
        assert!(ensure_current_assignment(&stale, &record).is_err());
        stale = context.clone();
        stale.assignment.as_mut().unwrap().execution_id = Uuid::new_v4();
        assert!(ensure_current_assignment(&stale, &record).is_err());
        stale = context.clone();
        stale.assignment.as_mut().unwrap().agent_id = Uuid::new_v4();
        assert!(ensure_current_assignment(&stale, &record).is_err());
        stale = context.clone();
        stale.assignment = None;
        assert!(ensure_current_assignment(&stale, &record).is_err());
        let mut terminal = record;
        terminal.terminal_status = Some(domain::PmRuntimeStatus::Completed);
        assert!(ensure_current_assignment(&context, &terminal).is_err());
    }
}
