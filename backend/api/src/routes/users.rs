use crate::middleware::{CurrentUser, require_admin};
use app::AppContext;
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use domain::{UpdateUserRoleRequest, UserListResponse, UserPermissionsResponse, UserResponse};
use shared::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(get, path = "/api/v1/users/me", tag = "auth", responses((status = 200, body = UserResponse)))]
pub async fn get_me(
    State(ctx): State<Arc<AppContext>>,
    Extension(current): Extension<CurrentUser>,
) -> Result<Json<UserResponse>, AppError> {
    let user = ctx
        .repo
        .find_user_by_id(current.id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    Ok(Json(user.into()))
}

#[utoipa::path(get, path = "/api/v1/users/me/permissions", tag = "auth", responses((status = 200, body = UserPermissionsResponse)))]
pub async fn get_permissions(
    Extension(current): Extension<CurrentUser>,
) -> Result<Json<UserPermissionsResponse>, AppError> {
    Ok(Json(UserPermissionsResponse {
        user_id: current.id,
        role: current.role,
        is_system_admin: current.is_system_admin,
        permissions: current.role.permissions(),
    }))
}

#[utoipa::path(get, path = "/api/v1/users", tag = "auth", responses((status = 200, body = UserListResponse)))]
pub async fn list_users(
    State(ctx): State<Arc<AppContext>>,
    Extension(current): Extension<CurrentUser>,
) -> Result<Json<UserListResponse>, AppError> {
    if !current.can_read_all_sessions() {
        let user = ctx
            .repo
            .find_user_by_id(current.id)
            .await?
            .ok_or(AppError::Unauthorized)?;
        return Ok(Json(UserListResponse {
            users: vec![user.into()],
        }));
    }
    Ok(Json(UserListResponse {
        users: ctx.repo.list_users().await?,
    }))
}

#[utoipa::path(patch, path = "/api/v1/users/{user_id}/role", tag = "auth", params(("user_id" = Uuid, Path)), request_body = UpdateUserRoleRequest, responses((status = 200, body = UserResponse)))]
pub async fn update_user_role(
    State(ctx): State<Arc<AppContext>>,
    Extension(current): Extension<CurrentUser>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserRoleRequest>,
) -> Result<Json<UserResponse>, AppError> {
    require_admin(&current)?;
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let user = ctx.repo.update_user_role(user_id, req).await?;
    ctx.repo
        .insert_audit(
            Some(current.id),
            "user.role.update",
            "user",
            Some(user.id.to_string()),
            audit_payload,
        )
        .await?;
    Ok(Json(user))
}
