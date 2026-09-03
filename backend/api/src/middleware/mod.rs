use app::AppContext;
use axum::{
    body::Body,
    extract::State,
    http::{Request, header},
    middleware::Next,
    response::Response,
};
use domain::SystemRole;
use shared::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub id: Uuid,
    pub role: SystemRole,
    pub is_system_admin: bool,
}

impl CurrentUser {
    pub fn can_operate_fleet(&self) -> bool {
        self.role.can_operate_fleet()
    }

    pub fn can_read_all_sessions(&self) -> bool {
        self.role.can_read_all_sessions()
    }

    pub fn can_manage_users(&self) -> bool {
        self.role.is_admin()
    }
}

pub fn require_operator(user: &CurrentUser) -> Result<(), AppError> {
    if user.can_operate_fleet() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub fn require_admin(user: &CurrentUser) -> Result<(), AppError> {
    if user.can_manage_users() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub async fn require_auth(
    State(ctx): State<Arc<AppContext>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;
    let claims = ctx.auth.validate_access_token(token)?;
    let user_id = claims
        .sub
        .parse::<Uuid>()
        .map_err(|_| AppError::Unauthorized)?;
    let user = ctx
        .repo
        .find_user_by_id(user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    if !user.is_active {
        return Err(AppError::Forbidden);
    }
    req.extensions_mut().insert(CurrentUser {
        id: user.id,
        role: user.system_role,
        is_system_admin: user.is_system_admin,
    });
    Ok(next.run(req).await)
}
