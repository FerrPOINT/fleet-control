use app::AppContext;
use axum::{
    Json,
    extract::{Query, State},
};
use domain::AgentLogEntry;
use serde::Deserialize;
use shared::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub agent_id: Option<Uuid>,
    pub limit: Option<u64>,
}

#[utoipa::path(get, path = "/api/v1/logs", tag = "runtime", responses((status = 200, body = Vec<AgentLogEntry>)))]
pub async fn list_logs(
    State(ctx): State<Arc<AppContext>>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<Vec<AgentLogEntry>>, AppError> {
    Ok(Json(
        ctx.repo
            .list_logs(query.agent_id, query.limit.unwrap_or(100).min(500))
            .await?,
    ))
}
