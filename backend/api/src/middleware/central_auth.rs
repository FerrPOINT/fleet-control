//! Fleet-control wiring of the shared central-auth bridge.
//!
//! All JWKS/login mechanics live in `sdlc_auth_core::service_bridge`;
//! this file only maps bridge outcomes to fleet-control types.

use sdlc_auth_core::service_bridge::{BridgeOutcome, ServiceBridge};

/// Env prefix: FLEET_CONTROL_AUTH__CENTRAL_{JWKS_URI,ISSUER,LOGIN_URL,TIMEOUT_SECS}.
pub static BRIDGE: ServiceBridge = ServiceBridge::new("FLEET_CONTROL_AUTH__CENTRAL");

/// Central-first bearer validation result, flattened for the middleware.
pub enum CentralCheck {
    /// Validated centrally — shadow user must be linked by the caller.
    Validated(sdlc_auth_core::AuthContext, String),
    /// Not a central token (or central not configured) — legacy path.
    FallThrough,
    /// Central token, expired.
    Expired,
    Unavailable,
}

pub async fn check_token(token: &str) -> CentralCheck {
    let (outcome, name) = BRIDGE.try_token_with_name(token).await;
    bridge_check(outcome, name)
}

fn bridge_check(outcome: BridgeOutcome, name: Option<String>) -> CentralCheck {
    match outcome {
        BridgeOutcome::Validated(ctx) => match name.filter(|name| !name.trim().is_empty()) {
            Some(name) => CentralCheck::Validated(ctx, name.trim().to_string()),
            None => CentralCheck::Unavailable,
        },
        BridgeOutcome::NotOurs | BridgeOutcome::NotConfigured => CentralCheck::FallThrough,
        BridgeOutcome::Expired => CentralCheck::Expired,
        BridgeOutcome::Invalid(reason) => {
            tracing::debug!(reason, "central token rejected");
            CentralCheck::Expired
        }
        BridgeOutcome::Unavailable => CentralCheck::Unavailable,
    }
}

/// Central login proxy; `None` = not configured / rejected / unreachable
/// (transport errors are logged, local login stays the fallback).
pub async fn try_login(
    email: &str,
    password: &str,
) -> Option<(
    sdlc_auth_core::service_bridge::CentralTokenPair,
    sdlc_auth_core::AuthContext,
    String,
)> {
    match BRIDGE.try_login(email, password).await {
        Ok(Some(pair)) => match check_token(&pair.access_token).await {
            CentralCheck::Validated(ctx, name) => Some((pair, ctx, name)),
            _ => None,
        },
        Ok(None) => None,
        Err(transport) => {
            tracing::warn!(%transport, "central login failed; local fallback");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal() -> sdlc_auth_core::AuthContext {
        sdlc_auth_core::AuthContext {
            user_id: "central-subject".into(),
            role: None,
            scopes: ["fleet-control:read".to_string()].into_iter().collect(),
            session_id: None,
            email: Some("email-prefix@example.test".into()),
            token: "fixture".into(),
        }
    }

    #[test]
    fn verified_name_is_preserved_without_changing_token_scopes() {
        let CentralCheck::Validated(ctx, name) = bridge_check(
            BridgeOutcome::Validated(principal()),
            Some(" Renamed Human ".into()),
        ) else {
            panic!("valid named principal was rejected");
        };
        assert_eq!(name, "Renamed Human");
        assert_eq!(ctx.user_id, "central-subject");
        assert!(ctx.allows_service("fleet-control", "GET"));
        assert!(!ctx.allows_service("fleet-control", "POST"));
        assert!(!ctx.allows_service("wiki", "GET"));
    }

    #[test]
    fn missing_verified_name_is_unavailable_not_an_email_fallback() {
        for name in [None, Some(String::new()), Some("  ".into())] {
            assert!(matches!(
                bridge_check(BridgeOutcome::Validated(principal()), name),
                CentralCheck::Unavailable
            ));
        }
    }

    #[test]
    fn rejected_central_tokens_never_become_local_fallbacks() {
        for outcome in [
            BridgeOutcome::Expired,
            BridgeOutcome::Invalid("invalid fixture".into()),
        ] {
            assert!(matches!(bridge_check(outcome, None), CentralCheck::Expired));
        }
        assert!(matches!(
            bridge_check(BridgeOutcome::Unavailable, None),
            CentralCheck::Unavailable
        ));
        assert!(matches!(
            bridge_check(BridgeOutcome::NotConfigured, None),
            CentralCheck::FallThrough
        ));
    }
}
