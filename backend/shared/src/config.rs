use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::{env, path::Path};

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub fleet: FleetConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16,
    pub cors_allowed_origins: Vec<String>,
    #[serde(default = "default_auth_rate_burst")]
    pub auth_rate_burst: u32,
    #[serde(default = "default_auth_rate_period_secs")]
    pub auth_rate_period_secs: u64,
    #[serde(default = "default_general_rate_burst")]
    pub general_rate_burst: u32,
    #[serde(default = "default_general_rate_per_second")]
    pub general_rate_per_second: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub mode: String,
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    /// OIDC mode: issuer URL used for the `iss` claim check and default
    /// JWKS URL (`<issuer>/keys`).
    pub oidc_issuer_url: String,
    /// Override JWKS URL (defaults to `<issuer>/keys` when empty).
    pub oidc_jwks_url: String,
    /// Expected `aud` claim for provider-issued access tokens.
    pub oidc_audience: String,
    /// Claim carrying the FC role mapping ('role' by default).
    pub oidc_role_claim: String,
    /// JWKS cache refresh interval, seconds (default 300).
    pub oidc_jwks_refresh_secs: u64,
    pub access_token_ttl_minutes: u64,
    pub refresh_token_ttl_days: u64,
    pub refresh_cookie_name: String,
    pub refresh_cookie_secure: bool,
    pub refresh_cookie_same_site: String,
    pub refresh_cookie_domain: Option<String>,
    pub refresh_cookie_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Archived agents older than this many days become `stale` in review.
    pub stale_archived_days: u32,
    /// Run the scheduled stale-folder review with this period (seconds).
    pub review_interval_secs: u64,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            stale_archived_days: 30,
            review_interval_secs: 3600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetConfig {
    pub agents_root: String,
    pub hermes_source: String,
    pub hermes_command: String,
    pub java_agent_source: String,
    pub java_agent_command: String,
    pub runtime_token_secret: String,
    pub agent_port_base: u16,
    pub agent_port_stride: u16,
    /// Forge (CI-CD) base URL for deployment triggers, e.g. http://cicd-backend:22801.
    pub forge_api_url: Option<String>,
    /// Bearer token for Forge API when auth is enabled there.
    pub forge_api_token: Option<String>,
    /// Forge project name used for deployment triggers.
    pub forge_project: Option<String>,
    /// project-workflow base URL for namespace/workflow sync, e.g. http://pw-api:8811.
    pub project_workflow_url: Option<String>,
    /// Read-only machine token for the Project Workflow catalog bridge.
    pub project_workflow_catalog_token: Option<String>,
    /// Operator retention policy thresholds (docs/IMPLEMENTATION_PLAN.md Phase 3).
    pub retention: RetentionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub public: bool,
}

fn default_auth_rate_burst() -> u32 {
    5
}

fn default_auth_rate_period_secs() -> u64 {
    15
}

fn default_general_rate_burst() -> u32 {
    60
}

fn default_general_rate_per_second() -> u64 {
    60
}

impl AppConfig {
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.address, self.server.port)
    }

    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_path("config/default.toml")
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let defaults = Config::builder()
            .set_default("database.url", "")?
            .set_default("database.max_connections", 20u64)?
            .set_default("database.min_connections", 5u64)?
            .set_default("database.connect_timeout_seconds", 10u64)?
            .set_default("database.idle_timeout_seconds", 600u64)?
            .set_default("server.address", "0.0.0.0")?
            .set_default("server.port", 23801u16)?
            .set_default(
                "server.cors_allowed_origins",
                vec![
                    "http://localhost:23802",
                    "http://localhost:4173",
                    "http://localhost:5173",
                    "http://127.0.0.1:23802",
                    "http://127.0.0.1:4173",
                    "http://127.0.0.1:5173",
                ],
            )?
            .set_default("server.auth_rate_burst", 5u32)?
            .set_default("server.auth_rate_period_secs", 15u64)?
            .set_default("server.general_rate_burst", 60u32)?
            .set_default("server.general_rate_per_second", 60u64)?
            .set_default("auth.mode", "hmac")?
            .set_default("auth.oidc_issuer_url", "")?
            .set_default("auth.oidc_jwks_url", "")?
            .set_default("auth.oidc_audience", "")?
            .set_default("auth.oidc_role_claim", "role")?
            .set_default("auth.oidc_jwks_refresh_secs", 300)?
            .set_default("auth.jwt_secret", "[CHANGE_ME]")?
            .set_default("auth.jwt_issuer", "fleet-control")?
            .set_default("auth.jwt_audience", "sdlc")?
            .set_default("auth.access_token_ttl_minutes", 15u64)?
            .set_default("auth.refresh_token_ttl_days", 7u64)?
            .set_default("auth.refresh_cookie_name", "refresh_token")?
            .set_default("auth.refresh_cookie_secure", true)?
            .set_default("auth.refresh_cookie_same_site", "Lax")?
            .set_default("auth.refresh_cookie_domain", Option::<String>::None)?
            .set_default("auth.refresh_cookie_path", "/api/v1/auth")?
            .set_default("fleet.agents_root", "./data/agents")?
            .set_default("fleet.hermes_source", "../прототипы/hermes")?
            .set_default("fleet.hermes_command", "hermes")?
            .set_default("fleet.java_agent_source", "../java-agent")?
            .set_default("fleet.java_agent_command", "java")?
            .set_default("fleet.runtime_token_secret", "[CHANGE_ME]")?
            .set_default("fleet.agent_port_base", 29000u16)?
            .set_default("fleet.agent_port_stride", 10u16)?
            .set_default("fleet.forge_api_url", Option::<String>::None)?
            .set_default("fleet.forge_api_token", Option::<String>::None)?
            .set_default("fleet.forge_project", Option::<String>::None)?
            .set_default("fleet.project_workflow_url", Option::<String>::None)?
            .set_default(
                "fleet.project_workflow_catalog_token",
                Option::<String>::None,
            )?
            .set_default("fleet.retention.stale_archived_days", 30u64)?
            .set_default("fleet.retention.review_interval_secs", 3600u64)?
            .set_default("metrics.public", true)?
            .build()?;

        let mut cfg: AppConfig = Config::builder()
            .add_source(defaults)
            .add_source(File::from(path.as_ref()).required(false))
            .add_source(
                Environment::with_prefix("FLEET_CONTROL")
                    .separator("__")
                    .prefix_separator("_")
                    .try_parsing(true),
            )
            .build()?
            .try_deserialize()?;

        if let Ok(secret) = env::var("FLEET_CONTROL_JWT_SECRET") {
            cfg.auth.jwt_secret = secret;
        }
        if let Ok(secret) = env::var("FLEET_CONTROL_RUNTIME_TOKEN_SECRET") {
            cfg.fleet.runtime_token_secret = secret;
        }

        cfg.auth.mode = cfg.auth.mode.trim().to_ascii_lowercase();
        cfg.auth.jwt_issuer = cfg.auth.jwt_issuer.trim().to_string();
        cfg.auth.jwt_audience = cfg.auth.jwt_audience.trim().to_string();

        if cfg.auth.jwt_secret == "[CHANGE_ME]" {
            return Err(ConfigError::Message(
                "auth.jwt_secret must be changed from default [CHANGE_ME]".to_string(),
            ));
        }
        if cfg.fleet.runtime_token_secret == "[CHANGE_ME]" {
            return Err(ConfigError::Message(
                "fleet.runtime_token_secret must be changed from default [CHANGE_ME]".to_string(),
            ));
        }
        if cfg.auth.mode == "oidc" && cfg.auth.oidc_issuer_url.trim().is_empty() {
            return Err(ConfigError::Message(
                "auth.mode=oidc requires auth.oidc_issuer_url".to_string(),
            ));
        }
        if cfg.auth.mode != "hmac" && cfg.auth.mode != "oidc" {
            return Err(ConfigError::Message(
                "auth.mode supports only hmac or oidc".to_string(),
            ));
        }
        if cfg.auth.jwt_issuer.trim().is_empty() {
            return Err(ConfigError::Message(
                "auth.jwt_issuer must not be empty".to_string(),
            ));
        }
        if cfg.auth.jwt_audience.trim().is_empty() {
            return Err(ConfigError::Message(
                "auth.jwt_audience must not be empty".to_string(),
            ));
        }
        if cfg.server.auth_rate_period_secs == 0 || cfg.server.general_rate_per_second == 0 {
            return Err(ConfigError::Message(
                "server rate-limit periods must be greater than zero".to_string(),
            ));
        }
        if cfg.server.general_rate_per_second > 1_000_000_000 {
            return Err(ConfigError::Message(
                "server.general_rate_per_second must not exceed 1000000000".to_string(),
            ));
        }
        if cfg.server.auth_rate_burst == 0 || cfg.server.general_rate_burst == 0 {
            return Err(ConfigError::Message(
                "server rate-limit bursts must be at least 1".to_string(),
            ));
        }
        if cfg.fleet.agent_port_stride < 4 {
            return Err(ConfigError::Message(
                "fleet.agent_port_stride must be at least 4".to_string(),
            ));
        }

        Ok(cfg)
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_connections: 20,
            min_connections: 5,
            connect_timeout_seconds: 10,
            idle_timeout_seconds: 600,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: "0.0.0.0".to_string(),
            port: 23801,
            cors_allowed_origins: vec![
                "http://localhost:23802".to_string(),
                "http://localhost:4173".to_string(),
                "http://localhost:5173".to_string(),
                "http://127.0.0.1:23802".to_string(),
                "http://127.0.0.1:4173".to_string(),
                "http://127.0.0.1:5173".to_string(),
            ],
            auth_rate_burst: default_auth_rate_burst(),
            auth_rate_period_secs: default_auth_rate_period_secs(),
            general_rate_burst: default_general_rate_burst(),
            general_rate_per_second: default_general_rate_per_second(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: "hmac".to_string(),
            jwt_secret: "[CHANGE_ME]".to_string(),
            jwt_issuer: "fleet-control".to_string(),
            jwt_audience: "sdlc".to_string(),
            oidc_issuer_url: String::new(),
            oidc_jwks_url: String::new(),
            oidc_audience: String::new(),
            oidc_role_claim: "role".to_string(),
            oidc_jwks_refresh_secs: 300,
            access_token_ttl_minutes: 15,
            refresh_token_ttl_days: 7,
            refresh_cookie_name: "refresh_token".to_string(),
            refresh_cookie_secure: true,
            refresh_cookie_same_site: "Lax".to_string(),
            refresh_cookie_domain: None,
            refresh_cookie_path: "/api/v1/auth".to_string(),
        }
    }
}

impl Default for FleetConfig {
    fn default() -> Self {
        Self {
            agents_root: "./data/agents".to_string(),
            hermes_source: "../прототипы/hermes".to_string(),
            hermes_command: "hermes".to_string(),
            java_agent_source: "../java-agent".to_string(),
            java_agent_command: "java".to_string(),
            forge_api_url: None,
            forge_api_token: None,
            forge_project: None,
            project_workflow_url: None,
            project_workflow_catalog_token: None,
            runtime_token_secret: "[CHANGE_ME]".to_string(),
            agent_port_base: 29000,
            agent_port_stride: 10,
            retention: RetentionConfig::default(),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self { public: true }
    }
}
