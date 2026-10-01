use app::{AppContext, SessionListFilter};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
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
use std::{collections::VecDeque, convert::Infallible, sync::Arc, time::Duration};
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
    if req.author_agent_id.is_some()
        || req.runtime_message_id.is_some()
        || req
            .message_kind
            .is_some_and(|kind| kind != domain::MessageKind::UserPrompt)
    {
        return Err(AppError::validation(
            "human messages cannot impersonate runtime agents or events",
        ));
    }
    let audit_payload = serde_json::json!({
        "author_agent_id": req.author_agent_id,
        "message_kind": req.message_kind,
        "body_length": req.body.chars().count(),
    });
    let session = ctx.repo.get_session(session_id).await?;
    ensure_session_write_access(&session, &user)?;
    let agent = ctx.repo.get_agent(session.primary_agent_id).await?;
    if agent.kind == domain::AgentKind::JavaAgent {
        return Err(AppError::validation(
            "Java Agent runtime chat is planned for phase 2",
        ));
    }
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

#[derive(Debug, Deserialize)]
pub struct StreamQuery {
    pub cursor: Option<i64>,
}

#[utoipa::path(get, path = "/api/v1/sessions/{session_id}/stream", tag = "sessions", params(("session_id" = Uuid, Path), ("cursor" = Option<i64>, Query, description = "Replay events after this session cursor; Last-Event-ID is also supported")), responses((status = 200, description = "Durable session-scoped SSE event stream")))]
pub async fn stream_session(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(session_id): Path<Uuid>,
    Query(query): Query<StreamQuery>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let session = ctx.repo.get_session(session_id).await?;
    ensure_session_read_access(&session, &user)?;
    let header_cursor = headers
        .get("last-event-id")
        .map(|value| {
            value
                .to_str()
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .ok_or_else(|| AppError::validation("invalid Last-Event-ID"))
        })
        .transpose()?;
    let current = ctx.repo.session_event_cursor(session_id).await?;
    let cursor = query.cursor.or(header_cursor).unwrap_or(current);
    if cursor < 0 || cursor > current {
        return Err(AppError::validation("session cursor is out of range"));
    }
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?
        .to_string();
    let mut queue = VecDeque::new();
    // Snapshot invalidation closes the gap between the initial detail load and stream open.
    queue.push_back(
        Event::default()
            .event("session")
            .id(cursor.to_string())
            .data(serde_json::json!({ "type": "snapshot", "session_id": session_id }).to_string()),
    );
    let stream = futures_util::stream::unfold(
        (ctx, user, token, cursor, queue),
        move |(ctx, user, token, mut cursor, mut queue)| async move {
            loop {
                let valid_token = match crate::middleware::central_auth::check_token(&token).await {
                    crate::middleware::central_auth::CentralCheck::Validated(central) => {
                        central.allows_service("fleet-control", "GET")
                    }
                    crate::middleware::central_auth::CentralCheck::FallThrough
                        if std::env::var_os("FLEET_CONTROL_AUTH__CENTRAL_JWKS_URI").is_none() =>
                    {
                        ctx.auth.validate_access_token(&token).await.is_ok()
                    }
                    _ => false,
                };
                let principal = ctx.repo.find_user_by_id(user.id).await.ok().flatten()?;
                if !valid_token || !principal.is_active {
                    return None;
                }
                let session = ctx.repo.get_session(session_id).await.ok()?;
                if session.user_id != user.id && !principal.system_role.can_read_all_sessions() {
                    return None;
                }
                if let Some(event) = queue.pop_front() {
                    return Some((
                        Ok::<_, Infallible>(event),
                        (ctx, user, token, cursor, queue),
                    ));
                }
                let events = ctx
                    .repo
                    .list_session_events(session_id, cursor)
                    .await
                    .ok()?;
                for event in events {
                    cursor = event.sequence;
                    queue.push_back(
                        Event::default()
                            .event("session")
                            .id(event.sequence.to_string())
                            .data(event.payload.to_string()),
                    );
                }
                if queue.is_empty() {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        },
    );
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

fn ensure_run_belongs_to_session(run: &SessionAgentRun, session_id: Uuid) -> Result<(), AppError> {
    if run.session_id != session_id {
        return Err(AppError::not_found("session_agent_run", run.id));
    }
    Ok(())
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
