use app::AppContext;
use axum::{
    Json,
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
}

#[utoipa::path(get, path = "/api/v1/sessions", tag = "sessions", responses((status = 200, body = Vec<AgentSession>)))]
pub async fn list_sessions(
    State(ctx): State<Arc<AppContext>>,
    Query(query): Query<SessionQuery>,
) -> Result<Json<Vec<AgentSession>>, AppError> {
    Ok(Json(ctx.repo.list_sessions(query.agent_id).await?))
}

#[utoipa::path(post, path = "/api/v1/sessions", tag = "sessions", request_body = CreateSessionRequest, responses((status = 200, body = AgentSession)))]
pub async fn create_session(
    State(ctx): State<Arc<AppContext>>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<AgentSession>, AppError> {
    let session = ctx.repo.create_session(req).await?;
    ctx.emit(FleetEvent::SessionChanged {
        session_id: session.id.to_string(),
        agent_id: session.agent_id.to_string(),
    });
    Ok(Json(session))
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
