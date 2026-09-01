use crate::middleware::CurrentUser;
use app::AppContext;
use axum::{Extension, Json, extract::State};
use domain::{UserListResponse, UserResponse};
use shared::AppError;
use std::sync::Arc;

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

#[utoipa::path(get, path = "/api/v1/users", tag = "auth", responses((status = 200, body = UserListResponse)))]
pub async fn list_users(
    State(ctx): State<Arc<AppContext>>,
) -> Result<Json<UserListResponse>, AppError> {
    Ok(Json(UserListResponse {
        users: ctx.repo.list_users().await?,
    }))
}
