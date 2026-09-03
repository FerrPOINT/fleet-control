use app::AppContext;
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use domain::{Agent, LeaderExecutor, UpdateLeaderExecutorsRequest};
use shared::{AppError, FleetEvent};
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(get, path = "/api/v1/leaders", tag = "leaders", responses((status = 200, body = Vec<Agent>)))]
pub async fn list_leaders(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<Vec<Agent>>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(
        ctx.repo
            .list_agents_by_product_role(domain::AgentProductRole::Leader)
            .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/leaders/{leader_agent_id}/executors", tag = "leaders", params(("leader_agent_id" = Uuid, Path)), responses((status = 200, body = Vec<LeaderExecutor>)))]
pub async fn list_leader_executors(
    State(ctx): State<Arc<AppContext>>,
    Path(leader_agent_id): Path<Uuid>,
) -> Result<Json<Vec<LeaderExecutor>>, AppError> {
    Ok(Json(ctx.repo.list_leader_executors(leader_agent_id).await?))
}

#[utoipa::path(put, path = "/api/v1/leaders/{leader_agent_id}/executors", tag = "leaders", params(("leader_agent_id" = Uuid, Path)), request_body = UpdateLeaderExecutorsRequest, responses((status = 200, body = Vec<LeaderExecutor>)))]
pub async fn update_leader_executors(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(leader_agent_id): Path<Uuid>,
    Json(req): Json<UpdateLeaderExecutorsRequest>,
) -> Result<Json<Vec<LeaderExecutor>>, AppError> {
    crate::middleware::require_operator(&user)?;
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let executors = ctx
        .repo
        .replace_leader_executors(leader_agent_id, req, user.id)
        .await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "leader_executors.replace",
            "leader",
            Some(leader_agent_id.to_string()),
            audit_payload,
        )
        .await?;
    ctx.emit(FleetEvent::AgentUpdated {
        agent_id: leader_agent_id.to_string(),
        name: "leader team updated".to_string(),
    });
    Ok(Json(executors))
}
