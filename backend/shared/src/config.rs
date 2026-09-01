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
    pub jwt_secret: String,
    pub access_token_ttl_minutes: u64,
    pub refresh_token_ttl_days: u64,
    pub refresh_cookie_name: String,
    pub refresh_cookie_secure: bool,
    pub refresh_cookie_same_site: String,
    pub refresh_cookie_domain: Option<String>,
    pub refresh_cookie_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetConfig {
    pub agents_root: String,
    pub hermes_source: String,
    pub hermes_command: String,
    pub java_agent_source: String,
    pub java_agent_command: String,
    pub agent_port_base: u16,
    pub agent_port_stride: u16,
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
            .set_default("auth.jwt_secret", "[CHANGE_ME]")?
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
            .set_default("fleet.agent_port_base", 29000u16)?
            .set_default("fleet.agent_port_stride", 10u16)?
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

        if cfg.auth.jwt_secret == "[CHANGE_ME]" {
            return Err(ConfigError::Message(
                "auth.jwt_secret must be changed from default [CHANGE_ME]".to_string(),
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
            jwt_secret: "[CHANGE_ME]".to_string(),
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
            agent_port_base: 29000,
            agent_port_stride: 10,
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self { public: true }
    }
}
