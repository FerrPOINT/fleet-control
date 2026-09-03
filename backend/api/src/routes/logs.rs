use app::{AppContext, AuditLogFilter};
use axum::{
    Extension, Json,
    extract::{Query, State},
};
use chrono::{DateTime, FixedOffset};
use domain::{AgentLogEntry, AuditLogEntry};
use serde::Deserialize;
use shared::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub agent_id: Option<Uuid>,
    pub limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub actor_user_id: Option<Uuid>,
    pub action: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub limit: Option<u64>,
}

#[utoipa::path(get, path = "/api/v1/logs", tag = "runtime", responses((status = 200, body = Vec<AgentLogEntry>)))]
pub async fn list_logs(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<Vec<AgentLogEntry>>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(
        ctx.repo
            .list_logs(query.agent_id, query.limit.unwrap_or(100).min(500))
            .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/audit-log", tag = "runtime", responses((status = 200, body = Vec<AuditLogEntry>)))]
pub async fn list_audit_log(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<Vec<AuditLogEntry>>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(
        ctx.repo
            .list_audit_log(AuditLogFilter {
                actor_user_id: query.actor_user_id,
                action: query.action.filter(|value| !value.trim().is_empty()),
                entity_type: query.entity_type.filter(|value| !value.trim().is_empty()),
                entity_id: query.entity_id.filter(|value| !value.trim().is_empty()),
                date_from: parse_rfc3339(query.date_from.as_deref())?,
                date_to: parse_rfc3339(query.date_to.as_deref())?,
                limit: query.limit.unwrap_or(100),
            })
            .await?,
    ))
}

fn parse_rfc3339(value: Option<&str>) -> Result<Option<DateTime<FixedOffset>>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    DateTime::parse_from_rfc3339(value)
        .map(Some)
        .map_err(|_| AppError::validation("date filter must be RFC3339"))
}
