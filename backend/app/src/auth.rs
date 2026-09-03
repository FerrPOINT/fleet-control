use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
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

#[derive(Debug, Clone)]
pub struct AuthService {
    config: AuthConfig,
}

impl AuthService {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    pub fn hash_password(&self, password: &str) -> Result<String, AppError> {
        if password.len() < 8 {
            return Err(AppError::validation(
                "password must contain at least 8 characters",
            ));
        }
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(AppError::internal)
    }

    pub fn verify_password(&self, password: &str, hash: &str) -> Result<(), AppError> {
        let parsed = PasswordHash::new(hash).map_err(AppError::internal)?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| AppError::Unauthorized)
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

    pub fn validate_access_token(&self, token: &str) -> Result<Claims, AppError> {
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

    #[test]
    fn issued_access_token_carries_fleet_compatible_claims() {
        let config = config();
        let service = AuthService::new(config);
        let user = user(SystemRole::Operator);

        let tokens = service.issue_tokens(&user).expect("tokens");
        let claims = service
            .validate_access_token(&tokens.response.access_token)
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

    #[test]
    fn legacy_access_token_without_audience_and_issuer_is_accepted() {
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
            .expect("legacy claims");

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, "legacy@example.test");
        assert!(claims.aud.is_empty());
        assert!(claims.iss.is_empty());
        assert!(claims.scopes.is_empty());
        assert!(claims.sid.is_none());
    }

    #[test]
    fn wrong_audience_is_rejected_without_legacy_fallback() {
        let config = config();
        let service = AuthService::new(config.clone());
        let mut claims = service
            .issue_tokens(&user(SystemRole::Admin))
            .and_then(|tokens| service.validate_access_token(&tokens.response.access_token))
            .expect("claims");
        claims.aud = "other".to_string();
        let token = token_for_claims(&config, &claims);

        assert!(!token_has_legacy_claim_shape(&token));
        assert!(matches!(
            service.validate_access_token(&token),
            Err(AppError::Unauthorized)
        ));
    }

    #[test]
    fn wrong_issuer_is_rejected_without_legacy_fallback() {
        let config = config();
        let service = AuthService::new(config.clone());
        let mut claims = service
            .issue_tokens(&user(SystemRole::Admin))
            .and_then(|tokens| service.validate_access_token(&tokens.response.access_token))
            .expect("claims");
        claims.iss = "other".to_string();
        let token = token_for_claims(&config, &claims);

        assert!(!token_has_legacy_claim_shape(&token));
        assert!(matches!(
            service.validate_access_token(&token),
            Err(AppError::Unauthorized)
        ));
    }
}
