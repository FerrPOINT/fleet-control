use app::AppContext;
use axum::{Extension, Json, extract::State};
use domain::WorkflowBinding;
use shared::AppError;
use std::sync::Arc;

#[utoipa::path(get, path = "/api/v1/workflow-bindings", tag = "agents", responses((status = 200, body = Vec<WorkflowBinding>)))]
pub async fn list_workflow_bindings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<Vec<WorkflowBinding>>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(ctx.repo.list_workflow_bindings().await?))
}
