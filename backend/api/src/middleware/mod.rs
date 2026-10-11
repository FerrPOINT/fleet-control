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
    // Some records a verified central identity and its write scope, never a local role grant.
    pub central_write: Option<bool>,
}

impl CurrentUser {
    pub fn can_operate_fleet(&self) -> bool {
        self.central_write.is_some() || self.role.can_operate_fleet()
    }

    pub fn can_read_all_sessions(&self) -> bool {
        self.central_write.is_some() || self.role.can_read_all_sessions()
    }

    pub fn can_manage_users(&self) -> bool {
        self.central_write.is_some() || self.role.is_admin()
    }

    pub fn permissions(&self) -> Vec<String> {
        let Some(can_write) = self.central_write else {
            return self.role.permissions();
        };
        let mut permissions = SystemRole::Admin.permissions();
        permissions.retain(|permission| {
            permission != "rbac:manage"
                && (can_write
                    || (!permission.ends_with(":manage") && permission != "sessions:write_own"))
        });
        permissions
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
    // Central validation is fail-closed; legacy tokens are accepted only without
    // central configuration. Request scopes precede human permission checks.
    match central_auth::check_token(token).await {
        central_auth::CentralCheck::Validated(central, display_name) => {
            if central.role.as_deref() == Some("service_account")
                || std::env::var("FLEET_CONTROL_NAMESPACE__MACHINE_SUBJECTS")
                    .unwrap_or_default()
                    .split(',')
                    .map(str::trim)
                    .any(|subject| !subject.is_empty() && subject == central.user_id)
            {
                return Err(AppError::Forbidden);
            }
            if !central.allows_service("fleet-control", req.method().as_str()) {
                return Err(AppError::Forbidden);
            }
            let user = resolve_central_user(&ctx, &central, display_name.as_deref()).await?;
            req.extensions_mut().insert(CurrentUser {
                id: user.id,
                role: user.system_role,
                is_system_admin: user.system_role.is_admin(),
                central_write: Some(central.allows_service("fleet-control", "POST")),
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
        central_write: None,
    });
    Ok(next.run(req).await)
}

/// Resolves the local user by the verified central subject, never by email,
/// creating a shadow account on first login (password_hash "!" — local
/// password verify always fails; the central server owns credentials).
pub async fn find_or_link_central_user_public(
    ctx: &Arc<AppContext>,
    central: &sdlc_auth_core::AuthContext,
    display_name: &str,
) -> Result<app::auth::UserRecord, AppError> {
    resolve_central_user(ctx, central, Some(display_name)).await
}

pub(crate) async fn resolve_central_user(
    ctx: &AppContext,
    central: &sdlc_auth_core::AuthContext,
    display_name: Option<&str>,
) -> Result<app::auth::UserRecord, AppError> {
    if central.role.as_deref() == Some("service_account")
        || std::env::var("FLEET_CONTROL_NAMESPACE__MACHINE_SUBJECTS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .any(|subject| !subject.is_empty() && subject == central.user_id)
    {
        return Err(AppError::Forbidden);
    }
    let Some(display_name) = display_name else {
        return ctx
            .repo
            .find_user_by_central_subject(&central.user_id)
            .await?
            .filter(|user| user.is_active)
            .ok_or(AppError::Unauthorized);
    };
    if display_name.trim().is_empty() {
        return Err(AppError::Unavailable(
            "Central Auth profile is unavailable".into(),
        ));
    }
    let email = central
        .email
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if email.is_empty() {
        return Err(AppError::Unauthorized);
    }
    ctx.repo
        .find_or_create_central_user(&central.user_id, &email, display_name)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(role: SystemRole, central_write: Option<bool>) -> CurrentUser {
        CurrentUser {
            id: Uuid::new_v4(),
            role,
            is_system_admin: role.is_admin(),
            central_write,
        }
    }

    #[test]
    fn verified_central_permissions_ignore_historical_roles_without_promoting_them() {
        for role in [SystemRole::User, SystemRole::Operator, SystemRole::Admin] {
            let user = principal(role, Some(true));
            assert!(require_operator(&user).is_ok());
            assert!(require_admin(&user).is_ok());
            assert!(user.can_read_all_sessions());
            assert!(user.permissions().contains(&"agents:manage".into()));
            assert!(!user.permissions().contains(&"rbac:manage".into()));
            assert_eq!(user.role, role);
            assert_eq!(user.is_system_admin, role.is_admin());
        }
    }

    #[test]
    fn central_read_permissions_never_advertise_write_actions() {
        let user = principal(SystemRole::User, Some(false));
        // Request scopes are checked before this principal is constructed.
        assert!(user.can_operate_fleet());
        assert!(user.can_read_all_sessions());
        let permissions = user.permissions();
        assert!(permissions.contains(&"sessions:read_all".into()));
        assert!(permissions.contains(&"logs:read".into()));
        assert!(!permissions.iter().any(|value| value.ends_with(":manage")));
        assert!(!permissions.contains(&"sessions:write_own".into()));
    }

    #[test]
    fn standalone_roles_keep_their_historical_checks() {
        let user = principal(SystemRole::User, None);
        assert!(require_operator(&user).is_err());
        assert!(require_admin(&user).is_err());
        assert!(!user.can_read_all_sessions());
        for role in [SystemRole::User, SystemRole::Operator, SystemRole::Admin] {
            assert_eq!(principal(role, None).permissions(), role.permissions());
        }
    }
}
