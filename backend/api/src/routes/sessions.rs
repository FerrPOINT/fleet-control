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
        ("user_id" = Option<String>, Query, description = "Comma-separated user ids, or all for central users and legacy operators/admins. Central private sessions remain owner-only. Omit for current user.")
    ),
    responses((status = 200, body = Vec<AgentSession>))
)]
pub async fn list_sessions(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Query(query): Query<SessionQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<AgentSession>>, AppError> {
    let (user_ids, include_all_users) = parse_user_filter(query.user_id.as_deref(), &user)?;
    let task_project_access =
        if ctx.config.tracker.url.is_empty() && ctx.config.tracker.instance_id.is_empty() {
            None
        } else {
            Some(super::project_access::authorized_projects(&ctx, &headers).await?)
        };
    Ok(Json(
        ctx.repo
            .list_sessions(SessionListFilter {
                agent_id: query.agent_id,
                user_ids,
                leader_agent_id: query.leader_agent_id,
                include_all_users,
                task_project_access,
                private_user_id: user.central_write.map(|_| user.id),
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
            central_write: None,
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
            central_write: None,
        };
        let err = parse_user_filter(Some("all"), &current).expect_err("forbidden");

        assert!(matches!(err, AppError::Forbidden));
    }

    #[test]
    fn central_user_can_select_other_users_without_a_local_role() {
        let current = crate::middleware::CurrentUser {
            id: Uuid::new_v4(),
            role: domain::SystemRole::User,
            is_system_admin: false,
            central_write: Some(true),
        };
        assert!(parse_user_filter(Some("all"), &current).unwrap().1);
        let other = Uuid::new_v4();
        assert_eq!(
            parse_user_filter(Some(&other.to_string()), &current)
                .unwrap()
                .0,
            vec![other]
        );
    }

    fn session(owner: Uuid, visibility: domain::SessionVisibility) -> AgentSession {
        let agent = Uuid::new_v4();
        AgentSession {
            id: Uuid::new_v4(),
            agent_id: agent,
            primary_agent_id: agent,
            agent_name: "agent1".into(),
            primary_agent_name: "agent1".into(),
            user_id: owner,
            user_email: "owner@example.test".into(),
            user_username: "owner".into(),
            user_display_name: "Owner".into(),
            leader_agent_id: None,
            leader_agent_name: None,
            parent_session_id: None,
            created_by_leader_agent_id: None,
            visibility,
            title: "Session".into(),
            task_key: None,
            state: domain::SessionState::Active,
            namespace_id: None,
            external_session_id: None,
            last_message_preview: None,
            pending_delivery: None,
            task_bound: None,
            created_at: "2026-10-06".into(),
            updated_at: "2026-10-06".into(),
        }
    }

    #[test]
    fn private_owner_boundary_does_not_depend_on_central_historical_role() {
        for role in [
            domain::SystemRole::User,
            domain::SystemRole::Operator,
            domain::SystemRole::Admin,
        ] {
            let current = crate::middleware::CurrentUser {
                id: Uuid::new_v4(),
                role,
                is_system_admin: role.is_admin(),
                central_write: Some(true),
            };
            let own = session(current.id, domain::SessionVisibility::Private);
            assert!(ensure_session_read_access(&own, &current).is_ok());
            assert!(ensure_session_write_access(&own, &current).is_ok());
            let private = session(Uuid::new_v4(), domain::SessionVisibility::Private);
            assert!(ensure_session_read_access(&private, &current).is_err());
            assert!(ensure_session_write_access(&private, &current).is_err());
            let shared = session(Uuid::new_v4(), domain::SessionVisibility::LeaderScoped);
            assert!(ensure_session_read_access(&shared, &current).is_ok());
            assert!(ensure_session_write_access(&shared, &current).is_ok());
        }
    }

    #[test]
    fn session_stream_principal_stays_active_and_bound_to_original_user() {
        let mut principal = app::auth::UserRecord {
            id: Uuid::new_v4(),
            email: "stream@example.test".into(),
            username: "stream".into(),
            display_name: "Stream".into(),
            password_hash: "!".into(),
            refresh_token_hash: None,
            system_role: domain::SystemRole::User,
            is_system_admin: false,
            is_active: true,
        };
        assert!(session_stream_user_matches(&principal, principal.id));
        assert!(!session_stream_user_matches(&principal, Uuid::new_v4()));
        principal.is_active = false;
        assert!(!session_stream_user_matches(&principal, principal.id));
    }

    #[test]
    fn session_stream_legacy_subject_must_match_original_user() {
        let id = Uuid::new_v4();
        assert!(session_stream_subject_matches(&id.to_string(), id));
        assert!(!session_stream_subject_matches(
            &Uuid::new_v4().to_string(),
            id
        ));
        assert!(!session_stream_subject_matches("not-a-uuid", id));
    }
}

#[utoipa::path(get, path = "/api/v1/sessions/{session_id}", tag = "sessions", params(("session_id" = Uuid, Path)), responses((status = 200, body = AgentSession)))]
pub async fn get_session(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(session_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<AgentSession>, AppError> {
    super::task_chats::require_project_access(&ctx, &user, session_id, &headers).await?;
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
    if ctx.repo.get_task_chat_binding(session_id).await?.is_some() {
        return Err(AppError::conflict("task-bound chat agent is immutable"));
    }
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
    headers: HeaderMap,
) -> Result<Json<Vec<SessionMessage>>, AppError> {
    super::task_chats::require_project_access(&ctx, &user, session_id, &headers).await?;
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
    if ctx.repo.get_task_chat_binding(session_id).await?.is_some() {
        return Err(AppError::conflict(
            "task-bound messages require a verified workflow assignment; ordinary prompts cannot resume clarification",
        ));
    }
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
    headers: HeaderMap,
) -> Result<Json<Vec<SessionParticipant>>, AppError> {
    super::task_chats::require_project_access(&ctx, &user, session_id, &headers).await?;
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
    if ctx.repo.get_task_chat_binding(session_id).await?.is_some() {
        return Err(AppError::conflict("task-bound chats cannot change leader"));
    }
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
    headers: HeaderMap,
) -> Result<Json<Vec<SessionAgentRun>>, AppError> {
    super::task_chats::require_project_access(&ctx, &user, session_id, &headers).await?;
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
    super::task_chats::require_project_access(&ctx, &user, session_id, &headers).await?;
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
        (ctx, user, token, headers, cursor, queue),
        move |(ctx, user, token, headers, mut cursor, mut queue)| async move {
            loop {
                let valid_token = match crate::middleware::central_auth::check_token(&token).await {
                    crate::middleware::central_auth::CentralCheck::Validated(central, name) => {
                        if !central.allows_service("fleet-control", "GET") {
                            return None;
                        }
                        let email = central.email.as_deref()?;
                        let principal = ctx
                            .repo
                            .find_or_create_central_user(&central.user_id, email, &name)
                            .await
                            .ok()?;
                        session_stream_user_matches(&principal, user.id)
                    }
                    crate::middleware::central_auth::CentralCheck::FallThrough
                        if std::env::var_os("FLEET_CONTROL_AUTH__CENTRAL_JWKS_URI").is_none() =>
                    {
                        ctx.auth
                            .validate_access_token(&token)
                            .await
                            .is_ok_and(|claims| {
                                session_stream_subject_matches(&claims.sub, user.id)
                            })
                    }
                    _ => false,
                };
                let principal = ctx.repo.find_user_by_id(user.id).await.ok().flatten()?;
                if !valid_token || !session_stream_user_matches(&principal, user.id) {
                    return None;
                }
                let session = ctx.repo.get_session(session_id).await.ok()?;
                if ensure_session_read_access(&session, &user).is_err()
                    || (user.central_write.is_none()
                        && session.user_id != user.id
                        && !principal.system_role.can_read_all_sessions())
                {
                    return None;
                }
                if super::task_chats::require_project_access(&ctx, &user, session_id, &headers)
                    .await
                    .is_err()
                {
                    return None;
                }
                if let Some(event) = queue.pop_front() {
                    return Some((
                        Ok::<_, Infallible>(event),
                        (ctx, user, token, headers, cursor, queue),
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
    if ctx.repo.get_task_chat_binding(session_id).await?.is_some() {
        return Err(AppError::conflict(
            "task-bound chat control requires a verified workflow assignment",
        ));
    }
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
    headers: HeaderMap,
) -> Result<Json<RuntimeRunControlResponse>, AppError> {
    let session = ctx.repo.get_session(session_id).await?;
    ensure_session_write_access(&session, &user)?;
    let run = ctx.repo.get_session_agent_run(run_id).await?;
    ensure_run_belongs_to_session(&run, session_id)?;
    let agent = ctx.repo.get_agent(run.agent_id).await?;
    super::task_chats::require_project_access(&ctx, &user, session_id, &headers).await?;
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

#[utoipa::path(post, path = "/api/v1/sessions/{session_id}/runs/{run_id}/approval", tag = "sessions", params(("session_id" = Uuid, Path), ("run_id" = Uuid, Path)), request_body = ResolveRuntimeApprovalRequest, responses((status = 409, description = "Use the exact approval request decision endpoint")))]
pub async fn resolve_session_run_approval(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path((session_id, run_id)): Path<(Uuid, Uuid)>,
    Json(_req): Json<ResolveRuntimeApprovalRequest>,
) -> Result<Json<RuntimeRunControlResponse>, AppError> {
    let session = ctx.repo.get_session(session_id).await?;
    ensure_session_write_access(&session, &user)?;
    let run = ctx.repo.get_session_agent_run(run_id).await?;
    ensure_run_belongs_to_session(&run, session_id)?;
    Err(AppError::conflict(
        "run-wide approval is disabled; use an exact approval request decision",
    ))
}

fn ensure_run_belongs_to_session(run: &SessionAgentRun, session_id: Uuid) -> Result<(), AppError> {
    if run.session_id != session_id {
        return Err(AppError::not_found("session_agent_run", run.id));
    }
    Ok(())
}

fn session_stream_user_matches(principal: &app::auth::UserRecord, expected_user_id: Uuid) -> bool {
    principal.id == expected_user_id && principal.is_active
}

fn session_stream_subject_matches(subject: &str, expected_user_id: Uuid) -> bool {
    subject.parse::<Uuid>().ok() == Some(expected_user_id)
}

pub(super) fn ensure_session_read_access(
    session: &AgentSession,
    user: &crate::middleware::CurrentUser,
) -> Result<(), AppError> {
    if user.central_write.is_some() && session.visibility == domain::SessionVisibility::Private {
        return if session.user_id == user.id {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        };
    }
    if user.can_read_all_sessions() || session.user_id == user.id {
        return Ok(());
    }
    Err(AppError::Forbidden)
}

fn ensure_session_write_access(
    session: &AgentSession,
    user: &crate::middleware::CurrentUser,
) -> Result<(), AppError> {
    if user.central_write.is_some() && session.visibility == domain::SessionVisibility::Private {
        return if session.user_id == user.id {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        };
    }
    if user.can_operate_fleet() || session.user_id == user.id {
        return Ok(());
    }
    Err(AppError::Forbidden)
}
