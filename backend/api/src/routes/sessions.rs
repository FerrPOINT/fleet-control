use app::{AppContext, SessionListFilter};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use domain::{AgentSession, CreateSessionRequest, HandoffSessionRequest};
use serde::Deserialize;
use shared::{AppError, FleetEvent};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    pub agent_id: Option<Uuid>,
    pub user_id: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/sessions",
    tag = "sessions",
    params(
        ("agent_id" = Option<Uuid>, Query, description = "Limit sessions to one agent"),
        ("user_id" = Option<String>, Query, description = "Comma-separated user ids. Omit for all users.")
    ),
    responses((status = 200, body = Vec<AgentSession>))
)]
pub async fn list_sessions(
    State(ctx): State<Arc<AppContext>>,
    Query(query): Query<SessionQuery>,
) -> Result<Json<Vec<AgentSession>>, AppError> {
    Ok(Json(
        ctx.repo
            .list_sessions(SessionListFilter {
                agent_id: query.agent_id,
                user_ids: parse_user_ids(query.user_id.as_deref())?,
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
    let session = ctx.repo.create_session(req, user.id).await?;
    ctx.emit(FleetEvent::SessionChanged {
        session_id: session.id.to_string(),
        agent_id: session.agent_id.to_string(),
    });
    Ok(Json(session))
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
}

#[utoipa::path(get, path = "/api/v1/sessions/{session_id}", tag = "sessions", params(("session_id" = Uuid, Path)), responses((status = 200, body = AgentSession)))]
pub async fn get_session(
    State(ctx): State<Arc<AppContext>>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<AgentSession>, AppError> {
    Ok(Json(ctx.repo.get_session(session_id).await?))
}

#[utoipa::path(post, path = "/api/v1/sessions/{session_id}/handoff", tag = "sessions", params(("session_id" = Uuid, Path)), request_body = HandoffSessionRequest, responses((status = 200, body = AgentSession)))]
pub async fn handoff_session(
    State(ctx): State<Arc<AppContext>>,
    Path(session_id): Path<Uuid>,
    Json(req): Json<HandoffSessionRequest>,
) -> Result<Json<AgentSession>, AppError> {
    let session = ctx.repo.handoff_session(session_id, req).await?;
    ctx.emit(FleetEvent::SessionChanged {
        session_id: session.id.to_string(),
        agent_id: session.agent_id.to_string(),
    });
    Ok(Json(session))
}
