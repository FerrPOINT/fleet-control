use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use chrono::{Duration, Utc};
use domain::{AuthResponse, LoginRequest, RegisterRequest, UserResponse};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
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
            is_system_admin: value.is_system_admin,
            is_active: value.is_active,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub exp: usize,
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
        let exp = Utc::now() + Duration::minutes(self.config.access_token_ttl_minutes as i64);
        let claims = Claims {
            sub: user.id.to_string(),
            email: user.email.clone(),
            exp: exp.timestamp() as usize,
        };
        let access_token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
        .map_err(AppError::internal)?;
        let refresh_token = generate_refresh_token();
        let refresh_hash = hash_refresh_token(&refresh_token);
        Ok(AuthTokens {
            response: AuthResponse {
                access_token,
                user_id: user.id,
                email: user.email.clone(),
                username: user.username.clone(),
                display_name: user.display_name.clone(),
                is_system_admin: user.is_system_admin,
            },
            refresh_token,
            refresh_hash,
        })
    }

    pub fn validate_access_token(&self, token: &str) -> Result<Claims, AppError> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|_| AppError::Unauthorized)
    }
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
