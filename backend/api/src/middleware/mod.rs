pub mod central_auth;

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
    // Central fleet auth-server first (ES256 via JWKS); legacy HS256 access
    // tokens remain valid during the migration window.
    if let Some(central) = central_auth::try_central(token).await.ok().flatten() {
        let user = find_or_link_central_user(&ctx, &central).await?;
        req.extensions_mut().insert(CurrentUser {
            id: user.id,
            role: user.system_role,
            is_system_admin: user.is_system_admin,
        });
        return Ok(next.run(req).await);
    }

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

/// Resolves the local user for a central identity by its verified email,
/// creating a shadow account on first login (password_hash "!" — local
/// password verify always fails; the central server owns credentials).
pub async fn find_or_link_central_user_public(
    ctx: &Arc<AppContext>,
    central: &sdlc_auth_core::AuthContext,
) -> Result<app::auth::UserRecord, AppError> {
    find_or_link_central_user(ctx, central).await
}

async fn find_or_link_central_user(
    ctx: &Arc<AppContext>,
    central: &sdlc_auth_core::AuthContext,
) -> Result<app::auth::UserRecord, AppError> {
    let email = central
        .email
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if email.is_empty() {
        return Err(AppError::Unauthorized);
    }
    if let Some(existing) = ctx.repo.find_user_by_email(&email).await? {
        if !existing.is_active {
            return Err(AppError::Forbidden);
        }
        return Ok(existing);
    }
    let username = email.split('@').next().unwrap_or("central");
    if username.len() < 3 {
        return Err(AppError::Unauthorized);
    }
    let request = domain::RegisterRequest {
        email: email.clone(),
        username: username.to_string(),
        display_name: username.to_string(),
        password: String::new(), // shadow user: local login impossible ("!" hash)
    };
    ctx.repo.create_user(request, "!".to_string(), false).await
}
