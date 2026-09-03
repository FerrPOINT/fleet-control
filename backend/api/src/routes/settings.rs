use app::AppContext;
use axum::{Extension, Json, extract::State};
use domain::{AuthSettings, IntegrationSettings, PortSettings, RuntimeSettings};
use shared::AppError;
use std::sync::Arc;

#[utoipa::path(get, path = "/api/v1/settings/runtime", tag = "settings", responses((status = 200, body = RuntimeSettings)))]
pub async fn get_runtime_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<RuntimeSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(ctx.repo.get_runtime_settings(&ctx.config).await?))
}

#[utoipa::path(put, path = "/api/v1/settings/runtime", tag = "settings", request_body = RuntimeSettings, responses((status = 200, body = RuntimeSettings)))]
pub async fn update_runtime_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Json(req): Json<RuntimeSettings>,
) -> Result<Json<RuntimeSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let settings = ctx.repo.update_runtime_settings(req, user.id).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "settings.runtime.update",
            "settings",
            Some("runtime".to_string()),
            audit_payload,
        )
        .await?;
    Ok(Json(settings))
}

#[utoipa::path(get, path = "/api/v1/settings/ports", tag = "settings", responses((status = 200, body = PortSettings)))]
pub async fn get_port_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<PortSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(ctx.repo.get_port_settings(&ctx.config).await?))
}

#[utoipa::path(put, path = "/api/v1/settings/ports", tag = "settings", request_body = PortSettings, responses((status = 200, body = PortSettings)))]
pub async fn update_port_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Json(req): Json<PortSettings>,
) -> Result<Json<PortSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let settings = ctx.repo.update_port_settings(req, user.id).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "settings.ports.update",
            "settings",
            Some("ports".to_string()),
            audit_payload,
        )
        .await?;
    Ok(Json(settings))
}

#[utoipa::path(get, path = "/api/v1/settings/integrations", tag = "settings", responses((status = 200, body = IntegrationSettings)))]
pub async fn get_integration_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<IntegrationSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(ctx.repo.get_integration_settings().await?))
}

#[utoipa::path(put, path = "/api/v1/settings/integrations", tag = "settings", request_body = IntegrationSettings, responses((status = 200, body = IntegrationSettings)))]
pub async fn update_integration_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Json(req): Json<IntegrationSettings>,
) -> Result<Json<IntegrationSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let settings = ctx.repo.update_integration_settings(req, user.id).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "settings.integrations.update",
            "settings",
            Some("integrations".to_string()),
            audit_payload,
        )
        .await?;
    Ok(Json(settings))
}

#[utoipa::path(get, path = "/api/v1/settings/auth", tag = "settings", responses((status = 200, body = AuthSettings)))]
pub async fn get_auth_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<AuthSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(ctx.repo.get_auth_settings(&ctx.config).await?))
}

#[utoipa::path(put, path = "/api/v1/settings/auth", tag = "settings", request_body = AuthSettings, responses((status = 200, body = AuthSettings)))]
pub async fn update_auth_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Json(req): Json<AuthSettings>,
) -> Result<Json<AuthSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let settings = ctx.repo.update_auth_settings(req, user.id).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "settings.auth.update",
            "settings",
            Some("auth".to_string()),
            audit_payload,
        )
        .await?;
    Ok(Json(settings))
}
