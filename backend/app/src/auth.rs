use chrono::{Duration, Utc};
use domain::{AuthResponse, LoginRequest, RegisterRequest, SystemRole, UserResponse};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use shared::{AppError, AuthConfig};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UserRecord {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub refresh_token_hash: Option<String>,
    pub system_role: SystemRole,
    pub is_system_admin: bool,
    pub is_active: bool,
}

impl From<UserRecord> for UserResponse {
    fn from(value: UserRecord) -> Self {
        Self {
            id: value.id,
            email: value.email,
            username: value.username,
            display_name: value.display_name,
            system_role: value.system_role,
            is_system_admin: value.is_system_admin,
            is_active: value.is_active,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub exp: i64,
    #[serde(default)]
    pub iat: i64,
    #[serde(default)]
    pub aud: String,
    #[serde(default)]
    pub iss: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyClaims {
    sub: String,
    email: String,
    exp: i64,
}

#[derive(Debug, Clone)]
pub struct AuthTokens {
    pub response: AuthResponse,
    pub refresh_token: String,
    pub refresh_hash: String,
}

/// RS256 access-token validation against a remote OIDC provider's JWKS
/// (IMPLEMENTATION_PLAN: "Add OIDC/JWKS validation mode"). Keys are fetched
/// over HTTPS, cached in memory and refreshed on the configured interval or
/// on unknown-`kid` decode errors.
pub struct OidcValidator {
    jwks_url: String,
    issuer: String,
    audience: String,
    role_claim: String,
    client: reqwest::Client,
    keys: std::sync::RwLock<OidcKeyCache>,
}

struct OidcKeyCache {
    keys: Vec<Jwk>,
    fetched_at: std::time::Instant,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Jwk {
    kid: Option<String>,
    kty: String,
    n: Option<String>,
    e: Option<String>,
    /// Algorithm hint; informational only (RS256 is enforced by validation).
    #[allow(dead_code)]
    alg: Option<String>,
}

impl Jwk {
    fn to_decoding_key(&self) -> Option<jsonwebtoken::DecodingKey> {
        if self.kty != "RSA" {
            return None;
        }
        let n = self.n.as_deref()?;
        let e = self.e.as_deref()?;
        // Decode base64url modulus/exponent manually: no ring dependency
        // assumptions beyond what jsonwebtoken already uses.
        fn b64url(input: &str) -> Option<Vec<u8>> {
            use base64::Engine;
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(input)
                .ok()
        }
        let n = b64url(n)?;
        let e = b64url(e)?;
        let pem_input = rsa::RsaPublicKey::new(
            rsa::BigUint::from_bytes_be(&n),
            rsa::BigUint::from_bytes_be(&e),
        )
        .ok()?;
        use rsa::pkcs1::EncodeRsaPublicKey;
        let der = pem_input.to_pkcs1_der().ok()?.as_bytes().to_vec();
        Some(jsonwebtoken::DecodingKey::from_rsa_der(&der))
    }
}

impl OidcValidator {
    pub fn new(config: &AuthConfig) -> Self {
        let jwks_url = if config.oidc_jwks_url.trim().is_empty() {
            format!(
                "{}/keys",
                config.oidc_issuer_url.trim().trim_end_matches('/')
            )
        } else {
            config.oidc_jwks_url.trim().to_string()
        };
        Self {
            jwks_url,
            issuer: config
                .oidc_issuer_url
                .trim()
                .trim_end_matches('/')
                .to_string(),
            audience: config.oidc_audience.trim().to_string(),
            role_claim: config.oidc_role_claim.clone(),
            client: reqwest::Client::new(),
            keys: std::sync::RwLock::new(OidcKeyCache {
                keys: Vec::new(),
                fetched_at: std::time::Instant::now() - std::time::Duration::from_secs(24 * 3600),
            }),
        }
    }

    /// Test-only constructor with pre-seeded keys (no network).
    #[cfg(test)]
    pub fn with_keys(config: &AuthConfig, keys: Vec<Jwk>) -> Self {
        let validator = Self::new(config);
        *validator.keys.write().unwrap() = OidcKeyCache {
            keys,
            fetched_at: std::time::Instant::now(),
        };
        validator
    }

    async fn refresh_keys(&self) -> Result<(), AppError> {
        let jwks: serde_json::Value = self
            .client
            .get(&self.jwks_url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| {
                tracing::warn!("jwks fetch failed: {e}");
                AppError::Unauthorized
            })?
            .json()
            .await
            .map_err(|e| {
                tracing::warn!("jwks parse failed: {e}");
                AppError::Unauthorized
            })?;
        let keys: Vec<Jwk> = serde_json::from_value(
            jwks.get("keys")
                .cloned()
                .unwrap_or(serde_json::Value::Array(Vec::new())),
        )
        .unwrap_or_default();
        *self.keys.write().unwrap() = OidcKeyCache {
            keys,
            fetched_at: std::time::Instant::now(),
        };
        Ok(())
    }

    fn find_key(&self, kid: Option<&str>) -> Option<Jwk> {
        let cache = self.keys.read().unwrap();
        cache
            .keys
            .iter()
            .find(|k| match (kid, &k.kid) {
                (Some(want), Some(have)) => want == have,
                (None, _) | (_, None) => true,
            })
            .cloned()
    }

    fn cache_fresh(&self, refresh_secs: u64) -> bool {
        self.keys.read().unwrap().fetched_at.elapsed().as_secs() < refresh_secs
    }

    /// Validates an RS256 access token: signature against JWKS, `iss` and
    /// (when configured) `aud` claims, expiry. Returns the claims value.
    pub async fn validate_access_token(
        &self,
        token: &str,
        refresh_secs: u64,
    ) -> Result<serde_json::Value, AppError> {
        let header = jsonwebtoken::decode_header(token).map_err(|e| {
            tracing::warn!("invalid token header: {e}");
            AppError::Unauthorized
        })?;
        if header.alg != jsonwebtoken::Algorithm::RS256 {
            return Err(AppError::Unauthorized);
        }
        let kid = header.kid;
        if !self.cache_fresh(refresh_secs) {
            self.refresh_keys().await?;
        }
        let mut key = self.find_key(kid.as_deref());
        if key.is_none() {
            // unknown kid -> force one refresh and retry
            self.refresh_keys().await?;
            key = self.find_key(kid.as_deref());
        }
        let jwk = key.ok_or(AppError::Unauthorized)?;
        let decoding = jwk.to_decoding_key().ok_or(AppError::Unauthorized)?;

        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        if !self.audience.is_empty() {
            validation.set_audience(&[&self.audience]);
        } else {
            validation.validate_aud = false;
        }
        let data = jsonwebtoken::decode::<serde_json::Value>(token, &decoding, &validation)
            .map_err(|e| {
                tracing::warn!("invalid oidc token: {e}");
                AppError::Unauthorized
            })?;
        Ok(data.claims)
    }

    /// Maps the configured role claim to a SystemRole (unknown -> Viewer).
    pub fn role_from_claims(&self, claims: &serde_json::Value) -> SystemRole {
        let raw = claims
            .get(&self.role_claim)
            .and_then(Value::as_str)
            .unwrap_or("viewer");
        match raw.to_ascii_lowercase().as_str() {
            "admin" | "administrator" => SystemRole::Admin,
            "operator" | "maintainer" => SystemRole::Operator,
            _ => SystemRole::User,
        }
    }
}

pub struct AuthService {
    config: AuthConfig,
    oidc: Option<OidcValidator>,
}

impl AuthService {
    pub fn new(config: AuthConfig) -> Self {
        let oidc = if config.mode == "oidc" {
            Some(OidcValidator::new(&config))
        } else {
            None
        };
        Self { config, oidc }
    }

    /// True when the service validates provider-issued RS256 tokens only.
    pub fn is_oidc_mode(&self) -> bool {
        self.oidc.is_some()
    }

    pub fn hash_password(&self, password: &str) -> Result<String, AppError> {
        if password.len() < 8 {
            return Err(AppError::validation(
                "password must contain at least 8 characters",
            ));
        }
        sdlc_auth_core::password::hash_password(password).map_err(AppError::internal)
    }

    pub fn verify_password(&self, password: &str, hash: &str) -> Result<(), AppError> {
        if sdlc_auth_core::password::verify_password(password, hash).map_err(AppError::internal)? {
            Ok(())
        } else {
            Err(AppError::Unauthorized)
        }
    }

    pub fn issue_tokens(&self, user: &UserRecord) -> Result<AuthTokens, AppError> {
        let now = Utc::now();
        let exp = now + Duration::minutes(self.config.access_token_ttl_minutes as i64);
        let refresh_token = generate_refresh_token();
        let refresh_hash = hash_refresh_token(&refresh_token);
        let claims = Claims {
            sub: user.id.to_string(),
            email: user.email.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            aud: self.config.jwt_audience.clone(),
            iss: self.config.jwt_issuer.clone(),
            role: Some(user.system_role.as_str().to_string()),
            scopes: user.system_role.permissions(),
            sid: Some(Uuid::new_v4().to_string()),
        };
        let access_token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
        .map_err(AppError::internal)?;
        Ok(AuthTokens {
            response: AuthResponse {
                access_token,
                user_id: user.id,
                email: user.email.clone(),
                username: user.username.clone(),
                display_name: user.display_name.clone(),
                system_role: user.system_role,
                is_system_admin: user.is_system_admin,
            },
            refresh_token,
            refresh_hash,
        })
    }

    pub async fn validate_access_token(&self, token: &str) -> Result<Claims, AppError> {
        // OIDC mode: RS256 validation against the provider's JWKS. Local
        // HMAC tokens are NOT accepted in oidc mode (fail-closed switch).
        if let Some(validator) = &self.oidc {
            let claims = validator
                .validate_access_token(token, self.config.oidc_jwks_refresh_secs)
                .await?;
            let sub = claims
                .get("sub")
                .and_then(Value::as_str)
                .ok_or(AppError::Unauthorized)?
                .to_string();
            let role = validator.role_from_claims(&claims);
            let email = claims
                .get("email")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let scopes = claims
                .get("scope")
                .and_then(Value::as_str)
                .map(|scope| scope.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default();
            return Ok(Claims {
                sub,
                email,
                exp: claims
                    .get("exp")
                    .and_then(Value::as_i64)
                    .unwrap_or_default(),
                iat: claims
                    .get("iat")
                    .and_then(Value::as_i64)
                    .unwrap_or_default(),
                aud: claims
                    .get("aud")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                iss: claims
                    .get("iss")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                role: Some(role.to_string()),
                scopes,
                sid: claims
                    .get("sid")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
        match decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &self.fleet_validation(),
        ) {
            Ok(data) => Ok(data.claims),
            Err(_) if token_has_legacy_claim_shape(token) => self.validate_legacy_token(token),
            Err(_) => Err(AppError::Unauthorized),
        }
    }

    fn fleet_validation(&self) -> Validation {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&[self.config.jwt_audience.as_str()]);
        validation.set_issuer(&[self.config.jwt_issuer.as_str()]);
        validation.set_required_spec_claims(&["exp", "sub", "aud", "iss"]);
        validation
    }

    fn validate_legacy_token(&self, token: &str) -> Result<Claims, AppError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_aud = false;
        validation.set_required_spec_claims(&["exp", "sub"]);
        let data = decode::<LegacyClaims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|_| AppError::Unauthorized)?;
        Ok(Claims {
            sub: data.claims.sub,
            email: data.claims.email,
            exp: data.claims.exp,
            iat: 0,
            aud: String::new(),
            iss: String::new(),
            role: None,
            scopes: Vec::new(),
            sid: None,
        })
    }
}

fn token_has_legacy_claim_shape(token: &str) -> bool {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.insecure_disable_signature_validation();
    validation.validate_exp = false;
    validation.validate_nbf = false;
    validation.validate_aud = false;
    validation.set_required_spec_claims::<&str>(&[]);
    decode::<Value>(token, &DecodingKey::from_secret(&[]), &validation)
        .ok()
        .and_then(|data| data.claims.as_object().cloned())
        .is_some_and(|claims| !claims.contains_key("aud") && !claims.contains_key("iss"))
}

pub fn normalize_register(req: RegisterRequest) -> Result<RegisterRequest, AppError> {
    let email = req.email.trim().to_ascii_lowercase();
    let username = req.username.trim().to_ascii_lowercase();
    let display_name = req.display_name.trim().to_string();
    if !email.contains('@') || email.len() > 255 {
        return Err(AppError::validation("email must be valid"));
    }
    if username.len() < 3
        || username.len() > 64
        || !username
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(AppError::validation(
            "username must be 3-64 ASCII letters, numbers, '-' or '_'",
        ));
    }
    if display_name.is_empty() || display_name.len() > 128 {
        return Err(AppError::validation(
            "display_name must be 1-128 characters",
        ));
    }
    Ok(RegisterRequest {
        email,
        username,
        display_name,
        password: req.password,
    })
}

pub fn normalize_login(req: LoginRequest) -> LoginRequest {
    LoginRequest {
        email: req.email.trim().to_ascii_lowercase(),
        password: req.password,
    }
}

pub fn hash_refresh_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn generate_refresh_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> AuthConfig {
        AuthConfig {
            jwt_secret: "test-secret-long-enough".to_string(),
            access_token_ttl_minutes: 15,
            ..AuthConfig::default()
        }
    }

    fn user(role: SystemRole) -> UserRecord {
        UserRecord {
            id: Uuid::new_v4(),
            email: "user@example.test".to_string(),
            username: "user".to_string(),
            display_name: "Test User".to_string(),
            password_hash: "hash".to_string(),
            refresh_token_hash: None,
            system_role: role,
            is_system_admin: role.is_admin(),
            is_active: true,
        }
    }

    fn token_for_claims(config: &AuthConfig, claims: &Claims) -> String {
        encode(
            &Header::default(),
            claims,
            &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
        )
        .expect("token")
    }

    #[tokio::test]
    async fn issued_access_token_carries_fleet_compatible_claims() {
        let config = config();
        let service = AuthService::new(config);
        let user = user(SystemRole::Operator);

        let tokens = service.issue_tokens(&user).expect("tokens");
        let claims = service
            .validate_access_token(&tokens.response.access_token)
            .await
            .expect("claims");

        assert_eq!(claims.sub, user.id.to_string());
        assert_eq!(claims.email, user.email);
        assert_eq!(claims.aud, "sdlc");
        assert_eq!(claims.iss, "fleet-control");
        assert_eq!(claims.role.as_deref(), Some("operator"));
        assert!(claims.scopes.contains(&"runtime:manage".to_string()));
        assert!(claims.iat > 0);
        assert!(claims.sid.is_some());
    }

    #[tokio::test]
    async fn legacy_access_token_without_audience_and_issuer_is_accepted() {
        let config = config();
        let user_id = Uuid::new_v4();
        let legacy = LegacyClaims {
            sub: user_id.to_string(),
            email: "legacy@example.test".to_string(),
            exp: (Utc::now() + Duration::minutes(5)).timestamp(),
        };
        let token = encode(
            &Header::default(),
            &legacy,
            &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
        )
        .expect("legacy token");

        assert!(token_has_legacy_claim_shape(&token));
        let claims = AuthService::new(config)
            .validate_access_token(&token)
            .await
            .expect("legacy claims");

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, "legacy@example.test");
        assert!(claims.aud.is_empty());
        assert!(claims.iss.is_empty());
        assert!(claims.scopes.is_empty());
        assert!(claims.sid.is_none());
    }

    #[tokio::test]
    async fn wrong_audience_is_rejected_without_legacy_fallback() {
        let config = config();
        let service = AuthService::new(config.clone());
        let tokens = service
            .issue_tokens(&user(SystemRole::Admin))
            .expect("tokens");
        let mut claims = service
            .validate_access_token(&tokens.response.access_token)
            .await
            .expect("claims");
        claims.aud = "other".to_string();
        let token = token_for_claims(&config, &claims);

        assert!(!token_has_legacy_claim_shape(&token));
        assert!(matches!(
            service.validate_access_token(&token).await,
            Err(AppError::Unauthorized)
        ));
    }

    #[tokio::test]
    async fn wrong_issuer_is_rejected_without_legacy_fallback() {
        let config = config();
        let service = AuthService::new(config.clone());
        let tokens = service
            .issue_tokens(&user(SystemRole::Admin))
            .expect("tokens");
        let mut claims = service
            .validate_access_token(&tokens.response.access_token)
            .await
            .expect("claims");
        claims.iss = "other".to_string();
        let token = token_for_claims(&config, &claims);

        assert!(!token_has_legacy_claim_shape(&token));
        assert!(matches!(
            service.validate_access_token(&token).await,
            Err(AppError::Unauthorized)
        ));
    }

    // --- OIDC/JWKS validation (IMPLEMENTATION_PLAN auth item) ---

    fn oidc_config() -> AuthConfig {
        AuthConfig {
            mode: "oidc".to_string(),
            jwt_secret: "test".to_string(),
            jwt_issuer: "fleet".to_string(),
            jwt_audience: "fleet".to_string(),
            oidc_issuer_url: "https://idp.example.test".to_string(),
            oidc_jwks_url: String::new(),
            oidc_audience: "fleet-control".to_string(),
            oidc_role_claim: "role".to_string(),
            oidc_jwks_refresh_secs: 300,
            access_token_ttl_minutes: 15,
            refresh_token_ttl_days: 7,
            refresh_cookie_name: "refresh_token".to_string(),
            refresh_cookie_secure: true,
            refresh_cookie_same_site: "Lax".to_string(),
            refresh_cookie_domain: None,
            refresh_cookie_path: "/api/v1".to_string(),
        }
    }

    fn generate_jwk_pair() -> (Jwk, jsonwebtoken::EncodingKey) {
        use base64::Engine;
        use rsa::pkcs8::EncodePrivateKey;
        use rsa::traits::PublicKeyParts;
        let mut rng = rand_core::OsRng;
        let key = rsa::RsaPrivateKey::new(&mut rng, 2048).expect("rsa key");
        let b64 = |v: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v);
        let jwk = Jwk {
            kid: Some("test-key-1".to_string()),
            kty: "RSA".to_string(),
            n: Some(b64(&key.n().to_bytes_be())),
            e: Some(b64(&key.e().to_bytes_be())),
            alg: Some("RS256".to_string()),
        };
        let pkcs8 = key
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .expect("pkcs8 pem");
        (
            jwk,
            jsonwebtoken::EncodingKey::from_rsa_pem(pkcs8.as_bytes()).expect("encoding key"),
        )
    }

    #[tokio::test]
    async fn oidc_validator_accepts_provider_rs256_token() {
        let config = oidc_config();
        let (jwk, encoding) = generate_jwk_pair();
        let validator = OidcValidator::with_keys(&config, vec![jwk]);

        let header = jsonwebtoken::Header {
            alg: jsonwebtoken::Algorithm::RS256,
            kid: Some("test-key-1".to_string()),
            ..Default::default()
        };
        let claims = serde_json::json!({
            "sub": "user-1",
            "email": "user@example.test",
            "iss": "https://idp.example.test",
            "aud": "fleet-control",
            "exp": (Utc::now() + Duration::minutes(10)).timestamp(),
            "iat": Utc::now().timestamp(),
            "role": "admin",
        });
        let token = jsonwebtoken::encode(&header, &claims, &encoding).expect("sign");

        let validated = validator
            .validate_access_token(&token, 300)
            .await
            .expect("valid");
        assert_eq!(validated["sub"], "user-1");
        assert_eq!(validator.role_from_claims(&validated), SystemRole::Admin);
    }

    #[tokio::test]
    async fn oidc_validator_rejects_hmac_and_wrong_issuer() {
        let config = oidc_config();
        let (jwk, encoding) = generate_jwk_pair();
        let validator = OidcValidator::with_keys(&config, vec![jwk]);

        let hmac_token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &serde_json::json!({"sub": "x", "iss": "https://idp.example.test", "aud": "fleet-control", "exp": (Utc::now() + Duration::minutes(10)).timestamp()}),
            &jsonwebtoken::EncodingKey::from_secret(b"attacker-secret"),
        )
        .expect("hmac sign");
        assert!(
            validator
                .validate_access_token(&hmac_token, 300)
                .await
                .is_err()
        );

        let header = jsonwebtoken::Header {
            alg: jsonwebtoken::Algorithm::RS256,
            kid: Some("test-key-1".to_string()),
            ..Default::default()
        };
        let claims = serde_json::json!({
            "sub": "user-1",
            "iss": "https://evil.example.test",
            "aud": "fleet-control",
            "exp": (Utc::now() + Duration::minutes(10)).timestamp(),
        });
        let token = jsonwebtoken::encode(&header, &claims, &encoding).expect("sign");
        assert!(validator.validate_access_token(&token, 300).await.is_err());
    }

    #[tokio::test]
    async fn hmac_tokens_rejected_when_oidc_mode_active() {
        let (jwk, _encoding) = generate_jwk_pair();
        let service = AuthService {
            config: oidc_config(),
            oidc: Some(OidcValidator::with_keys(&oidc_config(), vec![jwk])),
        };
        assert!(service.is_oidc_mode());
        let hmac_token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &serde_json::json!({"sub": "u", "email": "u@x", "iss": "fleet", "aud": "fleet", "exp": (Utc::now() + Duration::minutes(5)).timestamp()}),
            &jsonwebtoken::EncodingKey::from_secret(b"test"),
        )
        .expect("sign");
        assert!(service.validate_access_token(&hmac_token).await.is_err());
    }
}
