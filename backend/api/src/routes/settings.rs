use app::AppContext;
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use domain::{
    ApplyManagedSettingsRequest, ApplyManagedSettingsResponse, AuthSettings, IntegrationSettings,
    ManagedSettingsPreview, ManagedSettingsPreviewRequest, ManagedSettingsState,
    ManagedSettingsVersion, PortSettings, RollbackManagedSettingsRequest, RuntimeSettings,
};
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

#[utoipa::path(get, path = "/api/v1/settings/managed", tag = "settings", responses((status = 200, body = ManagedSettingsState)))]
pub async fn get_managed_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<ManagedSettingsState>, AppError> {
    crate::middleware::require_operator(&user)?;
    let active = ctx.repo.get_active_managed_settings().await?;
    Ok(Json(ManagedSettingsState {
        active_version: active.as_ref().map(|version| version.version),
        snapshot: app::managed_settings_from_config(&ctx.config),
    }))
}

#[utoipa::path(post, path = "/api/v1/settings/managed/preview", tag = "settings", request_body = ManagedSettingsPreviewRequest, responses((status = 200, body = ManagedSettingsPreview), (status = 400, description = "Invalid settings")))]
pub async fn preview_managed_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Json(req): Json<ManagedSettingsPreviewRequest>,
) -> Result<Json<ManagedSettingsPreview>, AppError> {
    crate::middleware::require_operator(&user)?;
    let proposed = app::normalize_managed_settings(req.snapshot);
    app::validate_managed_settings(&proposed, &ctx.config)?;
    let active = ctx.repo.get_active_managed_settings().await?;
    let current = app::managed_settings_from_config(&ctx.config);
    let changes = app::managed_settings_changes(&current, &proposed)?;
    Ok(Json(ManagedSettingsPreview {
        active_version: active.as_ref().map(|version| version.version),
        restart_required: !changes.is_empty(),
        changes,
    }))
}

#[utoipa::path(post, path = "/api/v1/settings/managed/apply", tag = "settings", request_body = ApplyManagedSettingsRequest, responses((status = 200, body = ApplyManagedSettingsResponse), (status = 400, description = "Invalid settings or restart not confirmed"), (status = 409, description = "Active version changed concurrently")))]
pub async fn apply_managed_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Json(req): Json<ApplyManagedSettingsRequest>,
) -> Result<Json<ApplyManagedSettingsResponse>, AppError> {
    crate::middleware::require_operator(&user)?;
    let proposed = app::normalize_managed_settings(req.snapshot);
    app::validate_managed_settings(&proposed, &ctx.config)?;
    let active = ctx.repo.get_active_managed_settings().await?;
    ensure_expected_version(
        req.expected_active_version,
        active.as_ref().map(|version| version.version),
    )?;
    let current = app::managed_settings_from_config(&ctx.config);
    if app::managed_settings_changes(&current, &proposed)?.is_empty() {
        return Ok(Json(ApplyManagedSettingsResponse {
            version: active,
            restart_scheduled: false,
        }));
    }
    require_restart_confirmation(req.confirm_restart)?;
    let version = ctx
        .repo
        .activate_managed_settings(
            proposed,
            user.id,
            req.expected_active_version,
            None,
            "settings.apply",
        )
        .await?;
    ctx.schedule_restart();
    Ok(Json(ApplyManagedSettingsResponse {
        version: Some(version),
        restart_scheduled: true,
    }))
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct ManagedSettingsVersionsQuery {
    #[serde(default = "default_versions_limit")]
    pub limit: u64,
}

fn default_versions_limit() -> u64 {
    20
}

#[utoipa::path(get, path = "/api/v1/settings/managed/versions", tag = "settings", params(ManagedSettingsVersionsQuery), responses((status = 200, body = Vec<ManagedSettingsVersion>)))]
pub async fn list_managed_settings_versions(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Query(query): Query<ManagedSettingsVersionsQuery>,
) -> Result<Json<Vec<ManagedSettingsVersion>>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(
        ctx.repo.list_managed_settings_versions(query.limit).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/settings/managed/versions/{version}/rollback", tag = "settings", params(("version" = i64, Path)), request_body = RollbackManagedSettingsRequest, responses((status = 200, body = ApplyManagedSettingsResponse), (status = 400, description = "Invalid settings or restart not confirmed"), (status = 404, description = "Version not found"), (status = 409, description = "Active version changed concurrently")))]
pub async fn rollback_managed_settings(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(version): Path<i64>,
    Json(req): Json<RollbackManagedSettingsRequest>,
) -> Result<Json<ApplyManagedSettingsResponse>, AppError> {
    crate::middleware::require_operator(&user)?;
    let target = ctx.repo.get_managed_settings_version(version).await?;
    let proposed = app::normalize_managed_settings(target.snapshot);
    app::validate_managed_settings(&proposed, &ctx.config)?;
    let active = ctx.repo.get_active_managed_settings().await?;
    ensure_expected_version(
        req.expected_active_version,
        active.as_ref().map(|version| version.version),
    )?;
    let current = app::managed_settings_from_config(&ctx.config);
    if app::managed_settings_changes(&current, &proposed)?.is_empty() {
        return Ok(Json(ApplyManagedSettingsResponse {
            version: active,
            restart_scheduled: false,
        }));
    }
    require_restart_confirmation(req.confirm_restart)?;
    let rolled_back = ctx
        .repo
        .activate_managed_settings(
            proposed,
            user.id,
            req.expected_active_version,
            Some(version),
            "settings.rollback",
        )
        .await?;
    ctx.schedule_restart();
    Ok(Json(ApplyManagedSettingsResponse {
        version: Some(rolled_back),
        restart_scheduled: true,
    }))
}

fn ensure_expected_version(expected: Option<i64>, actual: Option<i64>) -> Result<(), AppError> {
    if expected == actual {
        Ok(())
    } else {
        Err(AppError::conflict(format!(
            "managed settings changed concurrently: expected version {expected:?}, active version is {actual:?}"
        )))
    }
}

fn require_restart_confirmation(confirmed: bool) -> Result<(), AppError> {
    if confirmed {
        Ok(())
    } else {
        Err(AppError::validation(
            "confirm_restart must be true before applying managed settings",
        ))
    }
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
    use super::{
        ensure_expected_version, require_restart_confirmation, settings_are_read_only,
        unchanged_settings,
    };
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

    #[test]
    fn managed_settings_require_confirmation_and_current_version() {
        assert!(require_restart_confirmation(false).is_err());
        assert!(require_restart_confirmation(true).is_ok());
        assert!(ensure_expected_version(Some(4), Some(4)).is_ok());
        assert!(ensure_expected_version(None, None).is_ok());
        assert!(ensure_expected_version(Some(4), Some(5)).is_err());
        assert!(ensure_expected_version(None, Some(1)).is_err());
    }
}
