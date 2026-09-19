use crate::middleware::{CurrentUser, require_admin};
use app::AppContext;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, header},
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
    headers: HeaderMap,
) -> Result<Json<UserListResponse>, AppError> {
    if let Ok(jwks) = std::env::var("FLEET_CONTROL_AUTH__CENTRAL_JWKS_URI") {
        let token = headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;
        let mut url = reqwest::Url::parse(&jwks)
            .map_err(|_| AppError::Unavailable("Central Auth URL is invalid".into()))?;
        url.set_path("/auth/users");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|_| AppError::Unavailable("Central Auth is unavailable".into()))?;
        let mut users = Vec::new();
        for page in 0..100 {
            let response = client
                .get(url.clone())
                .query(&[("offset", page * 100)])
                .bearer_auth(token)
                .send()
                .await
                .map_err(|_| AppError::Unavailable("Central Auth is unavailable".into()))?;
            if !response.status().is_success() {
                return Err(AppError::Unavailable(
                    "Central Auth directory is unavailable".into(),
                ));
            }
            let batch = response
                .json::<Vec<CentralDirectoryUser>>()
                .await
                .map_err(|_| AppError::Unavailable("Central Auth directory is invalid".into()))?;
            let count = batch.len();
            for entry in batch {
                if entry.status == "disabled" {
                    continue;
                }
                let mut user: UserResponse = ctx
                    .repo
                    .find_or_create_central_user(&entry.id, &entry.email, &entry.display_name)
                    .await?
                    .into();
                user.display_name = entry.display_name;
                users.push(user);
            }
            if count < 100 {
                return Ok(Json(UserListResponse { users }));
            }
        }
        return Err(AppError::Unavailable(
            "Central Auth directory is too large".into(),
        ));
    }
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

#[derive(serde::Deserialize)]
struct CentralDirectoryUser {
    id: String,
    email: String,
    display_name: String,
    status: String,
}

#[utoipa::path(patch, path = "/api/v1/users/{user_id}/role", tag = "auth", params(("user_id" = Uuid, Path)), request_body = UpdateUserRoleRequest, responses((status = 200, body = UserResponse)))]
pub async fn update_user_role(
    State(ctx): State<Arc<AppContext>>,
    Extension(current): Extension<CurrentUser>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserRoleRequest>,
) -> Result<Json<UserResponse>, AppError> {
    if std::env::var_os("FLEET_CONTROL_AUTH__CENTRAL_JWKS_URI").is_some() {
        return Err(AppError::Forbidden);
    }
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
