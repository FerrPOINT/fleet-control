use app::AppContext;
use axum::{Json, extract::State};
use domain::RuntimeTemplate;
use shared::AppError;
use std::sync::Arc;

#[utoipa::path(get, path = "/api/v1/runtime-templates", tag = "runtime", responses((status = 200, body = Vec<RuntimeTemplate>)))]
pub async fn list_runtime_templates(
    State(ctx): State<Arc<AppContext>>,
) -> Result<Json<Vec<RuntimeTemplate>>, AppError> {
    Ok(Json(ctx.repo.list_runtime_templates().await?))
}
