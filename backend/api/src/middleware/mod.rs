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
    match central_auth::check_token(token).await {
        central_auth::CentralCheck::Validated(central) => {
            if !central.allows_service("fleet-control", req.method().as_str()) {
                return Err(AppError::Forbidden);
            }
            let user = find_or_link_central_user(&ctx, &central).await?;
            req.extensions_mut().insert(CurrentUser {
                id: user.id,
                role: user.system_role,
                is_system_admin: user.system_role.is_admin(),
            });
            return Ok(next.run(req).await);
        }
        central_auth::CentralCheck::Expired => return Err(AppError::Unauthorized),
        central_auth::CentralCheck::Unavailable => {
            return Err(AppError::Unavailable(
                "Central Auth is temporarily unavailable".into(),
            ));
        }
        central_auth::CentralCheck::FallThrough => {}
    }

    if std::env::var_os("FLEET_CONTROL_AUTH__CENTRAL_JWKS_URI").is_some() {
        return Err(AppError::Unauthorized);
    }

    let claims = ctx.auth.validate_access_token(token).await?;
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

/// Resolves the local user by the verified central subject, never by email,
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
    let user = ctx
        .repo
        .find_or_create_central_user(
            &central.user_id,
            &email,
            email.split('@').next().unwrap_or(&email),
        )
        .await?;
    if std::env::var("FLEET_CONTROL_AUTH__BOOTSTRAP_ADMIN_SUB")
        .ok()
        .as_deref()
        == Some(central.user_id.as_str())
        && !ctx
            .repo
            .list_users()
            .await?
            .iter()
            .any(|user| user.is_active && user.system_role == SystemRole::Admin)
    {
        ctx.repo
            .update_user_role(
                user.id,
                domain::UpdateUserRoleRequest {
                    role: SystemRole::Admin,
                },
            )
            .await?;
        ctx.repo
            .insert_audit(
                Some(user.id),
                "rbac.bootstrap",
                "user",
                Some(user.id.to_string()),
                serde_json::json!({"source":"configured_central_subject"}),
            )
            .await?;
        return ctx
            .repo
            .find_user_by_id(user.id)
            .await?
            .ok_or(AppError::Unauthorized);
    }
    Ok(user)
}
