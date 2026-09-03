use app::{AppContext, SessionListFilter};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    response::sse::{Event, KeepAlive, Sse},
};
use domain::{
    AgentSession, AssignSessionLeaderRequest, CreateSessionDelegationRequest,
    CreateSessionMessageRequest, CreateSessionRequest, HandoffSessionRequest,
    ResolveRuntimeApprovalRequest, RuntimeRunControlResponse, SessionAgentRun, SessionMessage,
    SessionParticipant, SteerSessionRunRequest,
};
use futures_util::stream::Stream;
use serde::Deserialize;
use shared::{AppError, FleetEvent};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    pub agent_id: Option<Uuid>,
    pub user_id: Option<String>,
    pub leader_agent_id: Option<Uuid>,
}

#[utoipa::path(
    get,
    path = "/api/v1/sessions",
    tag = "sessions",
    params(
        ("agent_id" = Option<Uuid>, Query, description = "Limit sessions to one agent"),
        ("leader_agent_id" = Option<Uuid>, Query, description = "Limit sessions to one leader"),
        ("user_id" = Option<String>, Query, description = "Comma-separated user ids, or all for admin. Omit for current user.")
    ),
    responses((status = 200, body = Vec<AgentSession>))
)]
pub async fn list_sessions(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Query(query): Query<SessionQuery>,
) -> Result<Json<Vec<AgentSession>>, AppError> {
    let (user_ids, include_all_users) = parse_user_filter(query.user_id.as_deref(), &user)?;
    Ok(Json(
        ctx.repo
            .list_sessions(SessionListFilter {
                agent_id: query.agent_id,
                user_ids,
                leader_agent_id: query.leader_agent_id,
                include_all_users,
            })
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/sessions", tag = "sessions", request_body = CreateSessionRequest, responses((status = 200, body = AgentSession)))]
pub async fn create_session(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<AgentSession>, AppError> {
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let session = ctx.repo.create_session(req, user.id).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "session.create",
            "session",
            Some(session.id.to_string()),
            audit_payload,
        )
        .await?;
    ctx.emit(FleetEvent::SessionChanged {
        session_id: session.id.to_string(),
        agent_id: session.primary_agent_id.to_string(),
    });
    Ok(Json(session))
}

fn parse_user_filter(
    value: Option<&str>,
    current: &crate::middleware::CurrentUser,
) -> Result<(Vec<Uuid>, bool), AppError> {
    let Some(value) = value else {
        return Ok((vec![current.id], false));
    };
    let value = value.trim();
    if value.eq_ignore_ascii_case("all") || value.is_empty() {
        if !current.can_read_all_sessions() {
            return Err(AppError::Forbidden);
        }
        return Ok((Vec::new(), true));
    }
    let ids = parse_user_ids(Some(value))?;
    if ids.iter().any(|id| *id != current.id) && !current.can_read_all_sessions() {
        return Err(AppError::Forbidden);
    }
    Ok((ids, false))
}

fn parse_user_ids(value: Option<&str>) -> Result<Vec<Uuid>, AppError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| Uuid::parse_str(item).map_err(|_| AppError::validation("invalid user_id")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiple_user_ids() {
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let parsed = parse_user_ids(Some(&format!("{first},{second}"))).expect("parsed ids");

        assert_eq!(parsed, vec![first, second]);
    }

    #[test]
    fn rejects_invalid_user_id() {
        let err = parse_user_ids(Some("not-a-uuid")).expect_err("invalid id");

        assert!(err.to_string().contains("invalid user_id"));
    }

    #[test]
    fn omitted_user_filter_defaults_to_current_user() {
        let current = crate::middleware::CurrentUser {
            id: Uuid::new_v4(),
            role: domain::SystemRole::User,
            is_system_admin: false,
        };
        let (ids, include_all) = parse_user_filter(None, &current).expect("filter");

        assert_eq!(ids, vec![current.id]);
        assert!(!include_all);
    }

    #[test]
    fn all_user_filter_requires_admin() {
        let current = crate::middleware::CurrentUser {
            id: Uuid::new_v4(),
            role: domain::SystemRole::User,
            is_system_admin: false,
        };
        let err = parse_user_filter(Some("all"), &current).expect_err("forbidden");

        assert!(matches!(err, AppError::Forbidden));
    }
}

#[utoipa::path(get, path = "/api/v1/sessions/{session_id}", tag = "sessions", params(("session_id" = Uuid, Path)), responses((status = 200, body = AgentSession)))]
pub async fn get_session(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<AgentSession>, AppError> {
    let session = ctx.repo.get_session(session_id).await?;
    ensure_session_read_access(&session, &user)?;
    Ok(Json(session))
}

#[utoipa::path(post, path = "/api/v1/sessions/{session_id}/handoff", tag = "sessions", params(("session_id" = Uuid, Path)), request_body = HandoffSessionRequest, responses((status = 200, body = AgentSession)))]
pub async fn handoff_session(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(session_id): Path<Uuid>,
    Json(req): Json<HandoffSessionRequest>,
) -> Result<Json<AgentSession>, AppError> {
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let before = ctx.repo.get_session(session_id).await?;
    ensure_session_write_access(&before, &user)?;
    let session = ctx.repo.handoff_session(session_id, req).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "session.handoff",
            "session",
            Some(session.id.to_string()),
            audit_payload,
        )
        .await?;
    ctx.emit(FleetEvent::SessionChanged {
        session_id: session.id.to_string(),
        agent_id: session.primary_agent_id.to_string(),
    });
    Ok(Json(session))
}

#[utoipa::path(get, path = "/api/v1/sessions/{session_id}/messages", tag = "sessions", params(("session_id" = Uuid, Path)), responses((status = 200, body = Vec<SessionMessage>)))]
pub async fn list_session_messages(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<SessionMessage>>, AppError> {
    let session = ctx.repo.get_session(session_id).await?;
    ensure_session_read_access(&session, &user)?;
    Ok(Json(ctx.repo.list_session_messages(session_id).await?))
}

#[utoipa::path(post, path = "/api/v1/sessions/{session_id}/messages", tag = "sessions", params(("session_id" = Uuid, Path)), request_body = CreateSessionMessageRequest, responses((status = 200, body = SessionMessage)))]
pub async fn create_session_message(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(session_id): Path<Uuid>,
    Json(req): Json<CreateSessionMessageRequest>,
) -> Result<Json<SessionMessage>, AppError> {
    let audit_payload = serde_json::json!({
        "author_agent_id": req.author_agent_id,
        "message_kind": req.message_kind,
        "body_length": req.body.chars().count(),
    });
    let session = ctx.repo.get_session(session_id).await?;
    ensure_session_write_access(&session, &user)?;
    let message = ctx
        .repo
        .create_session_message(session_id, req, user.id)
        .await?;
    if !message.replayed {
        ctx.repo
            .insert_audit(
                Some(user.id),
                "session_message.create",
                "session",
                Some(session.id.to_string()),
                audit_payload,
            )
            .await?;
        dispatch_session_message(&ctx, &session, &message).await?;
    }
    if !message.replayed {
        ctx.emit(FleetEvent::SessionChanged {
            session_id: session.id.to_string(),
            agent_id: session.primary_agent_id.to_string(),
        });
    }
    Ok(Json(message))
}

#[utoipa::path(get, path = "/api/v1/sessions/{session_id}/participants", tag = "sessions", params(("session_id" = Uuid, Path)), responses((status = 200, body = Vec<SessionParticipant>)))]
pub async fn list_session_participants(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<SessionParticipant>>, AppError> {
    let session = ctx.repo.get_session(session_id).await?;
    ensure_session_read_access(&session, &user)?;
    Ok(Json(ctx.repo.list_session_participants(session_id).await?))
}

#[utoipa::path(post, path = "/api/v1/sessions/{session_id}/delegations", tag = "sessions", params(("session_id" = Uuid, Path)), request_body = CreateSessionDelegationRequest, responses((status = 200, body = AgentSession)))]
pub async fn create_session_delegation(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(session_id): Path<Uuid>,
    Json(req): Json<CreateSessionDelegationRequest>,
) -> Result<Json<AgentSession>, AppError> {
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let dispatch_initial = req
        .initial_message
        .as_deref()
        .map(str::trim)
        .is_some_and(|body| !body.is_empty());
    let parent = ctx.repo.get_session(session_id).await?;
    ensure_session_write_access(&parent, &user)?;
    let child = ctx
        .repo
        .create_session_delegation(session_id, req, user.id)
        .await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "session.delegation.create",
            "session",
            Some(child.id.to_string()),
            audit_payload,
        )
        .await?;
    ctx.emit(FleetEvent::SessionChanged {
        session_id: child.id.to_string(),
        agent_id: child.primary_agent_id.to_string(),
    });
    if dispatch_initial
        && let Some(message) = ctx
            .repo
            .list_session_messages(child.id)
            .await?
            .into_iter()
            .rev()
            .find(|message| {
                message.message_kind == domain::MessageKind::UserPrompt
                    && message.delivery_state == domain::MessageDeliveryState::Pending
            })
    {
        dispatch_session_message(&ctx, &child, &message).await?;
    }
    Ok(Json(child))
}

#[utoipa::path(put, path = "/api/v1/sessions/{session_id}/leader", tag = "sessions", params(("session_id" = Uuid, Path)), request_body = AssignSessionLeaderRequest, responses((status = 200, body = AgentSession)))]
pub async fn assign_session_leader(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(session_id): Path<Uuid>,
    Json(req): Json<AssignSessionLeaderRequest>,
) -> Result<Json<AgentSession>, AppError> {
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let before = ctx.repo.get_session(session_id).await?;
    ensure_session_write_access(&before, &user)?;
    let session = ctx
        .repo
        .assign_session_leader(session_id, req, user.id)
        .await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "session.leader.assign",
            "session",
            Some(session.id.to_string()),
            audit_payload,
        )
        .await?;
    ctx.emit(FleetEvent::SessionChanged {
        session_id: session.id.to_string(),
        agent_id: session.primary_agent_id.to_string(),
    });
    Ok(Json(session))
}

#[utoipa::path(get, path = "/api/v1/sessions/{session_id}/runs", tag = "sessions", params(("session_id" = Uuid, Path)), responses((status = 200, body = Vec<SessionAgentRun>)))]
pub async fn list_session_agent_runs(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<SessionAgentRun>>, AppError> {
    let session = ctx.repo.get_session(session_id).await?;
    ensure_session_read_access(&session, &user)?;
    Ok(Json(ctx.repo.list_session_agent_runs(session_id).await?))
}

#[utoipa::path(get, path = "/api/v1/sessions/{session_id}/stream", tag = "sessions", params(("session_id" = Uuid, Path)), responses((status = 200, description = "Session-scoped SSE event stream")))]
pub async fn stream_session(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(session_id): Path<Uuid>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let session = ctx.repo.get_session(session_id).await?;
    ensure_session_read_access(&session, &user)?;
    let expected = session_id.to_string();
    let stream = BroadcastStream::new(ctx.events.subscribe()).filter_map(move |event| {
        let expected = expected.clone();
        match event {
            Ok(event) if event_session_id(&event).as_deref() == Some(expected.as_str()) => {
                serde_json::to_string(&event)
                    .ok()
                    .map(|json| Ok(Event::default().event("session").data(json)))
            }
            _ => None,
        }
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(20))))
}

#[utoipa::path(post, path = "/api/v1/sessions/{session_id}/runs/{run_id}/steer", tag = "sessions", params(("session_id" = Uuid, Path), ("run_id" = Uuid, Path)), request_body = SteerSessionRunRequest, responses((status = 200, body = RuntimeRunControlResponse)))]
pub async fn steer_session_run(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path((session_id, run_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<SteerSessionRunRequest>,
) -> Result<Json<RuntimeRunControlResponse>, AppError> {
    let session = ctx.repo.get_session(session_id).await?;
    ensure_session_write_access(&session, &user)?;
    let run = ctx.repo.get_session_agent_run(run_id).await?;
    ensure_run_belongs_to_session(&run, session_id)?;
    let agent = ctx.repo.get_agent(run.agent_id).await?;
    let response = ctx.runtime.steer_run(&agent, &run, req).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "session_run.steer",
            "session_run",
            Some(run.id.to_string()),
            serde_json::json!({ "session_id": session_id, "runtime_run_id": run.runtime_run_id }),
        )
        .await?;
    Ok(Json(response))
}

#[utoipa::path(post, path = "/api/v1/sessions/{session_id}/runs/{run_id}/stop", tag = "sessions", params(("session_id" = Uuid, Path), ("run_id" = Uuid, Path)), responses((status = 200, body = RuntimeRunControlResponse)))]
pub async fn stop_session_run(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path((session_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RuntimeRunControlResponse>, AppError> {
    let session = ctx.repo.get_session(session_id).await?;
    ensure_session_write_access(&session, &user)?;
    let run = ctx.repo.get_session_agent_run(run_id).await?;
    ensure_run_belongs_to_session(&run, session_id)?;
    let agent = ctx.repo.get_agent(run.agent_id).await?;
    let response = ctx.runtime.stop_run(&agent, &run).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "session_run.stop",
            "session_run",
            Some(run.id.to_string()),
            serde_json::json!({ "session_id": session_id, "runtime_run_id": run.runtime_run_id }),
        )
        .await?;
    Ok(Json(response))
}

#[utoipa::path(post, path = "/api/v1/sessions/{session_id}/runs/{run_id}/approval", tag = "sessions", params(("session_id" = Uuid, Path), ("run_id" = Uuid, Path)), request_body = ResolveRuntimeApprovalRequest, responses((status = 200, body = RuntimeRunControlResponse)))]
pub async fn resolve_session_run_approval(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path((session_id, run_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<ResolveRuntimeApprovalRequest>,
) -> Result<Json<RuntimeRunControlResponse>, AppError> {
    let session = ctx.repo.get_session(session_id).await?;
    ensure_session_write_access(&session, &user)?;
    let run = ctx.repo.get_session_agent_run(run_id).await?;
    ensure_run_belongs_to_session(&run, session_id)?;
    let agent = ctx.repo.get_agent(run.agent_id).await?;
    let audit_payload = serde_json::json!({
        "session_id": session_id,
        "runtime_run_id": run.runtime_run_id,
        "choice": req.choice,
        "resolve_all": req.resolve_all,
    });
    let response = ctx
        .runtime
        .resolve_approval(&agent, &run, req.clone())
        .await?;
    let resolved_count = ctx
        .repo
        .resolve_runtime_approval_requests_for_run(run.id, req, user.id)
        .await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "session_run.approval",
            "session_run",
            Some(run.id.to_string()),
            serde_json::json!({
                "request": audit_payload,
                "resolved_approval_requests": resolved_count,
            }),
        )
        .await?;
    Ok(Json(response))
}

async fn dispatch_session_message(
    ctx: &Arc<AppContext>,
    session: &AgentSession,
    message: &SessionMessage,
) -> Result<(), AppError> {
    let target_agent_id = session.primary_agent_id;
    let agent = ctx.repo.get_agent(target_agent_id).await?;
    let response = ctx.runtime.send_message(&agent, session, message).await?;
    if response.status == domain::AgentStatus::Failed {
        ctx.repo
            .insert_event(
                Some(agent.id),
                "runtime_message_dispatch_failed",
                &response.message,
                serde_json::json!({ "session_id": session.id, "message_id": message.id }),
            )
            .await?;
    }
    Ok(())
}

fn ensure_run_belongs_to_session(run: &SessionAgentRun, session_id: Uuid) -> Result<(), AppError> {
    if run.session_id != session_id {
        return Err(AppError::not_found("session_agent_run", run.id));
    }
    Ok(())
}

fn event_session_id(event: &FleetEvent) -> Option<String> {
    match event {
        FleetEvent::SessionChanged { session_id, .. }
        | FleetEvent::SessionMessageChanged { session_id, .. }
        | FleetEvent::SessionRunChanged { session_id, .. }
        | FleetEvent::SessionRunDelta { session_id, .. }
        | FleetEvent::RuntimeApprovalRequested { session_id, .. } => Some(session_id.clone()),
        _ => None,
    }
}

fn ensure_session_read_access(
    session: &AgentSession,
    user: &crate::middleware::CurrentUser,
) -> Result<(), AppError> {
    if user.can_read_all_sessions() || session.user_id == user.id {
        return Ok(());
    }
    Err(AppError::Forbidden)
}

fn ensure_session_write_access(
    session: &AgentSession,
    user: &crate::middleware::CurrentUser,
) -> Result<(), AppError> {
    if user.can_operate_fleet() || session.user_id == user.id {
        return Ok(());
    }
    Err(AppError::Forbidden)
}
