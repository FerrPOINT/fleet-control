use app::AppContext;
use axum::{Extension, Json, extract::State};
use domain::Agent;
use shared::AppError;
use std::sync::Arc;

#[utoipa::path(get, path = "/api/v1/executors", tag = "executors", responses((status = 200, body = Vec<Agent>)))]
pub async fn list_executors(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<Vec<Agent>>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(
        ctx.repo
            .list_agents_by_product_role(domain::AgentProductRole::Executor)
            .await?,
    ))
}
