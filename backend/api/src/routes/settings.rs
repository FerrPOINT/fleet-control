use app::AppContext;
use axum::{Extension, Json, extract::State};
use domain::{AuthSettings, IntegrationSettings, PortSettings, RuntimeSettings};
use shared::AppError;
use std::sync::Arc;

#[utoipa::path(get, path = "/api/v1/settings/runtime", tag = "settings", responses((status = 200, body = RuntimeSettings, description = "Effective startup settings")))]
pub async fn get_runtime_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<RuntimeSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(ctx.repo.get_runtime_settings(&ctx.config).await?))
}

#[utoipa::path(put, path = "/api/v1/settings/runtime", tag = "settings", request_body = RuntimeSettings, responses((status = 200, body = RuntimeSettings, description = "No-op when the request matches effective startup settings"), (status = 409, description = "Settings are managed by startup configuration")))]
pub async fn update_runtime_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Json(req): Json<RuntimeSettings>,
) -> Result<Json<RuntimeSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    let effective = ctx.repo.get_runtime_settings(&ctx.config).await?;
    unchanged_settings(req, effective)
}

#[utoipa::path(get, path = "/api/v1/settings/ports", tag = "settings", responses((status = 200, body = PortSettings, description = "Effective startup settings")))]
pub async fn get_port_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<PortSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(ctx.repo.get_port_settings(&ctx.config).await?))
}

#[utoipa::path(put, path = "/api/v1/settings/ports", tag = "settings", request_body = PortSettings, responses((status = 200, body = PortSettings, description = "No-op when the request matches effective startup settings"), (status = 409, description = "Settings are managed by startup configuration")))]
pub async fn update_port_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Json(req): Json<PortSettings>,
) -> Result<Json<PortSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    let effective = ctx.repo.get_port_settings(&ctx.config).await?;
    unchanged_settings(req, effective)
}

#[utoipa::path(get, path = "/api/v1/settings/integrations", tag = "settings", responses((status = 200, body = IntegrationSettings, description = "Effective startup settings")))]
pub async fn get_integration_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<IntegrationSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(ctx.repo.get_integration_settings(&ctx.config).await?))
}

#[utoipa::path(put, path = "/api/v1/settings/integrations", tag = "settings", request_body = IntegrationSettings, responses((status = 200, body = IntegrationSettings, description = "No-op when the request matches effective startup settings"), (status = 409, description = "Settings are managed by startup configuration")))]
pub async fn update_integration_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Json(req): Json<IntegrationSettings>,
) -> Result<Json<IntegrationSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    let effective = ctx.repo.get_integration_settings(&ctx.config).await?;
    unchanged_settings(req, effective)
}

#[utoipa::path(get, path = "/api/v1/settings/auth", tag = "settings", responses((status = 200, body = AuthSettings, description = "Effective startup settings")))]
pub async fn get_auth_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<AuthSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(ctx.repo.get_auth_settings(&ctx.config).await?))
}

#[utoipa::path(put, path = "/api/v1/settings/auth", tag = "settings", request_body = AuthSettings, responses((status = 200, body = AuthSettings, description = "No-op when the request matches effective startup settings"), (status = 409, description = "Settings are managed by startup configuration")))]
pub async fn update_auth_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Json(req): Json<AuthSettings>,
) -> Result<Json<AuthSettings>, AppError> {
    crate::middleware::require_operator(&user)?;
    let effective = ctx.repo.get_auth_settings(&ctx.config).await?;
    unchanged_settings(req, effective)
}

fn unchanged_settings<T: PartialEq>(requested: T, effective: T) -> Result<Json<T>, AppError> {
    if requested == effective {
        Ok(Json(effective))
    } else {
        Err(settings_are_read_only())
    }
}

fn settings_are_read_only() -> AppError {
    AppError::conflict(
        "settings are read-only; change the startup environment or deployment configuration and restart the service",
    )
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct RetentionReviewOutcomeDto {
    pub stale_agent_ids: Vec<uuid::Uuid>,
    pub stale_archived_days: u32,
    pub reviewed_at: String,
}

impl From<app::RetentionReviewOutcome> for RetentionReviewOutcomeDto {
    fn from(o: app::RetentionReviewOutcome) -> Self {
        Self {
            stale_agent_ids: o.stale_agent_ids,
            stale_archived_days: o.stale_archived_days,
            reviewed_at: o.reviewed_at.to_rfc3339(),
        }
    }
}

#[utoipa::path(post, path = "/api/v1/settings/retention/review", tag = "settings", responses((status = 200, body = RetentionReviewOutcomeDto)))]
/// Run one stale-folder review pass now (operator): lists archived agents
/// older than `fleet.retention.stale_archived_days`. Read-only.
pub async fn run_retention_review(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<RetentionReviewOutcomeDto>, AppError> {
    crate::middleware::require_operator(&user)?;
    let outcome = ctx.review_stale_agent_folders().await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "settings.retention.review",
            "settings",
            Some("retention".to_string()),
            serde_json::to_value(&outcome).map_err(AppError::internal)?,
        )
        .await?;
    Ok(Json(RetentionReviewOutcomeDto::from(outcome)))
}

#[cfg(test)]
mod tests {
    use super::{settings_are_read_only, unchanged_settings};
    use domain::RuntimeSettings;

    #[test]
    fn rejected_settings_write_explains_the_owner_contract() {
        let error = settings_are_read_only().to_string();

        assert!(error.contains("read-only"));
        assert!(error.contains("startup environment"));
        assert!(error.contains("restart"));
    }

    #[test]
    fn legacy_settings_write_is_only_an_idempotent_no_op() {
        let effective = RuntimeSettings {
            agents_root: "/agents".to_string(),
            hermes_source: "/src/hermes".to_string(),
            hermes_command: "hermes".to_string(),
            java_agent_source: "/src/java-agent".to_string(),
            java_agent_command: "java".to_string(),
        };

        assert!(unchanged_settings(effective.clone(), effective.clone()).is_ok());
        assert!(
            unchanged_settings(
                RuntimeSettings {
                    hermes_command: "different".to_string(),
                    ..effective.clone()
                },
                effective,
            )
            .is_err()
        );
    }
}
