use app::{AppContext, fetch_workflow_catalog, workflow_rebind_selection};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use domain::{RebindWorkflowBindingRequest, WorkflowBinding, WorkflowCatalog};
use shared::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(get, path = "/api/v1/workflow-bindings", tag = "agents", responses((status = 200, body = Vec<WorkflowBinding>)))]
pub async fn list_workflow_bindings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<Vec<WorkflowBinding>>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(ctx.repo.list_workflow_bindings().await?))
}

#[utoipa::path(get, path = "/api/v1/workflow-catalog", tag = "agents", responses((status = 200, body = WorkflowCatalog)))]
pub async fn get_workflow_catalog(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<WorkflowCatalog>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(fetch_workflow_catalog(&ctx.config).await?))
}

#[utoipa::path(put, path = "/api/v1/workflow-bindings/{agent_id}", tag = "agents", params(("agent_id" = Uuid, Path)), request_body = RebindWorkflowBindingRequest, responses((status = 200, body = WorkflowBinding)))]
pub async fn rebind_workflow_binding(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(agent_id): Path<Uuid>,
    Json(request): Json<RebindWorkflowBindingRequest>,
) -> Result<Json<WorkflowBinding>, AppError> {
    crate::middleware::require_operator(&user)?;
    let catalog = fetch_workflow_catalog(&ctx.config).await?;
    let (namespace, workflow) =
        workflow_rebind_selection(&catalog, &request.namespace_id, &request.workflow_id)?;
    let binding = ctx
        .repo
        .rebind_workflow_binding(agent_id, namespace, workflow)
        .await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "workflow_binding.rebind",
            "workflow_binding",
            Some(agent_id.to_string()),
            serde_json::json!({
                "namespace_id": binding.namespace_id,
                "workflow_id": binding.workflow_id,
            }),
        )
        .await?;
    Ok(Json(binding))
}
