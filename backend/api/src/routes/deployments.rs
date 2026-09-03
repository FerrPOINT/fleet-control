use app::AppContext;
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use domain::{CreateDeploymentJobRequest, DeploymentJob, RuntimeTemplate};
use serde::Deserialize;
use shared::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct DeploymentJobsQuery {
    pub limit: Option<u64>,
}

#[utoipa::path(get, path = "/api/v1/runtime-templates", tag = "runtime", responses((status = 200, body = Vec<RuntimeTemplate>)))]
pub async fn list_runtime_templates(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<Vec<RuntimeTemplate>>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(ctx.repo.list_runtime_templates().await?))
}

#[utoipa::path(get, path = "/api/v1/deployments/jobs", tag = "runtime", responses((status = 200, body = Vec<DeploymentJob>)))]
pub async fn list_deployment_jobs(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Query(query): Query<DeploymentJobsQuery>,
) -> Result<Json<Vec<DeploymentJob>>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(
        ctx.repo
            .list_deployment_jobs(query.limit.unwrap_or(100))
            .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/deployments/jobs/{job_id}", tag = "runtime", params(("job_id" = Uuid, Path)), responses((status = 200, body = DeploymentJob)))]
pub async fn get_deployment_job(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<DeploymentJob>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(ctx.repo.get_deployment_job(job_id).await?))
}

#[utoipa::path(post, path = "/api/v1/deployments/jobs", tag = "runtime", request_body = CreateDeploymentJobRequest, responses((status = 200, body = DeploymentJob)))]
pub async fn create_deployment_job(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Json(req): Json<CreateDeploymentJobRequest>,
) -> Result<Json<DeploymentJob>, AppError> {
    crate::middleware::require_operator(&user)?;
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let job = ctx.repo.create_deployment_job(req, user.id).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "deployment_job.create",
            "deployment_job",
            Some(job.id.to_string()),
            audit_payload,
        )
        .await?;
    Ok(Json(job))
}

#[utoipa::path(post, path = "/api/v1/deployments/jobs/{job_id}/cancel", tag = "runtime", params(("job_id" = Uuid, Path)), responses((status = 200, body = DeploymentJob)))]
pub async fn cancel_deployment_job(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<DeploymentJob>, AppError> {
    crate::middleware::require_operator(&user)?;
    let job = ctx.repo.cancel_deployment_job(job_id, user.id).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "deployment_job.cancel",
            "deployment_job",
            Some(job.id.to_string()),
            serde_json::json!({ "state": job.state.as_str() }),
        )
        .await?;
    Ok(Json(job))
}
