//! Bridge between fleet-control auth and the central fleet auth-server
//! (services-base/auth-server, ES256 + JWKS, audience `sdlc`).
//!
//! When `FLEET_CONTROL_AUTH__CENTRAL_JWKS_URI` is configured the bearer
//! middleware tries central validation first; the verified email claim links
//! a local shadow user (created on demand, unusable password hash). Legacy
//! HS256 access tokens keep working, enabling a zero-downtime cutover.

use sdlc_auth_core::{AuthContext, JwksCache, Validator};
use std::sync::Arc;
use tokio::sync::OnceCell;

pub struct CentralAuth {
    validator: Validator,
    #[allow(dead_code)] // kept for future direct JWKS access (rotation checks)
    jwks: Arc<JwksCache>,
}

static CENTRAL: OnceCell<Option<CentralAuth>> = OnceCell::const_new();

/// Reads `FLEET_CONTROL_AUTH__CENTRAL_JWKS_URI` / `__CENTRAL_ISSUER` once.
/// `None` when central auth is not configured (legacy-only mode).
pub async fn central() -> Option<&'static CentralAuth> {
    CENTRAL
        .get_or_init(|| async {
            let uri = std::env::var("FLEET_CONTROL_AUTH__CENTRAL_JWKS_URI").ok()?;
            let issuer: Arc<String> = Arc::new(
                std::env::var("FLEET_CONTROL_AUTH__CENTRAL_ISSUER")
                    .unwrap_or_else(|_| "http://127.0.0.1:7701".into()),
            );
            match JwksCache::connect(&uri).await {
                Ok(jwks) => {
                    let jwks = Arc::new(jwks);
                    let validator = Validator::Jwks {
                        jwks: jwks.clone(),
                        issuer,
                    };
                    jwks.clone().spawn_refresh(std::time::Duration::from_secs(3600));
                    tracing::info!(jwks_uri = %uri, "central auth enabled");
                    Some(CentralAuth { validator, jwks })
                }
                Err(error) => {
                    tracing::warn!(%error, jwks_uri = %uri, "central auth unavailable; falling back to legacy sessions");
                    None
                }
            }
        })
        .await
        .as_ref()
}

/// Attempts central-token validation. `Ok(None)` = not a central token
/// (caller falls back to the legacy HS256 path).
pub async fn try_central(token: &str) -> Result<Option<AuthContext>, shared::AppError> {
    let Some(central) = central().await else {
        return Ok(None);
    };
    match central.validator.validate(token) {
        Ok(ctx) => Ok(Some(ctx)),
        // kid resolution failure = legacy token, not ours
        Err(sdlc_auth_core::AuthError::Jwks(_)) => Ok(None),
        Err(sdlc_auth_core::AuthError::Expired) => Err(shared::AppError::Unauthorized),
        Err(other) => {
            tracing::warn!(error = %other, "central token validation failed");
            Ok(None)
        }
    }
}

/// Login proxy to the central auth-server.
#[derive(serde::Deserialize)]
pub struct CentralAuthPair {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default)]
    pub token_type: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
}

pub struct CentralLoginConfig {
    login_url: String,
    timeout_secs: u64,
}

pub fn central_login_config() -> Option<CentralLoginConfig> {
    let url = std::env::var("FLEET_CONTROL_AUTH__CENTRAL_LOGIN_URL").ok()?;
    if url.trim().is_empty() {
        return None;
    }
    Some(CentralLoginConfig {
        login_url: url,
        timeout_secs: std::env::var("FLEET_CONTROL_AUTH__CENTRAL_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5),
    })
}

/// `Ok(None)` — central not configured. `Err(None)` — central rejected the
/// credentials (fall back to local). `Err(Some(err))` — transport error.
pub async fn try_central_login(
    config: &CentralLoginConfig,
    email: &str,
    password: &str,
) -> Result<Option<CentralAuthPair>, Option<String>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .build()
        .map_err(|e| Some(e.to_string()))?;
    let response = client
        .post(&config.login_url)
        .json(&serde_json::json!({ "email": email, "password": password }))
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, url = %config.login_url, "central login unreachable");
            Some(e.to_string())
        })?;
    if !response.status().is_success() {
        return Err(None);
    }
    let pair = response
        .json::<CentralAuthPair>()
        .await
        .map_err(|e| Some(e.to_string()))?;
    Ok(Some(pair))
}
