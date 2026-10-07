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
    #[serde(default)]
    pub pm: PmConfig,
    #[serde(default)]
    pub tracker: TrackerConfig,
    #[serde(default)]
    pub sdlc: SdlcConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SdlcConfig {
    pub configuration_readback_enabled: bool,
    pub auth_url: String,
    pub configuration_reader_subject: String,
    pub configuration_reader_agent_ids: String,
    pub workflow_binding: SdlcWorkflowConfig,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SdlcWorkflowConfig {
    pub url: String,
    #[serde(skip_serializing)]
    pub read_pat: String,
}

impl std::fmt::Debug for SdlcWorkflowConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdlcWorkflowConfig")
            .field("url", &self.url)
            .field("read_pat", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackerConfig {
    pub url: String,
    pub instance_id: String,
    pub pm_draft_creation_enabled: bool,
    pub pm_draft_project_ids: Vec<uuid::Uuid>,
    pub events: TrackerEventsConfig,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackerEventsConfig {
    pub enabled: bool,
    pub auth_url: String,
    pub machine_subject: String,
    #[serde(skip_serializing)]
    pub read_pat: String,
    pub poll_interval_seconds: u64,
}

impl Default for TrackerEventsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auth_url: String::new(),
            machine_subject: String::new(),
            read_pat: String::new(),
            poll_interval_seconds: 5,
        }
    }
}

impl std::fmt::Debug for TrackerEventsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackerEventsConfig")
            .field("enabled", &self.enabled)
            .field("auth_url", &self.auth_url)
            .field("machine_subject", &self.machine_subject)
            .field("read_pat", &"[REDACTED]")
            .field("poll_interval_seconds", &self.poll_interval_seconds)
            .finish()
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PmConfig {
    #[serde(skip_serializing)]
    pub readback_token: String,
    #[serde(skip_serializing)]
    pub namespace_read_pat: String,
    pub namespace_authority_issuer: String,
    pub namespace_provisioner_subject: String,
    pub credentials: PmCredentialsConfig,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PmCredentialsConfig {
    pub enabled: bool,
    pub auth_url: String,
    pub machine_subject: String,
    #[serde(skip_serializing)]
    pub parent_pat: String,
    pub ttl_seconds: i64,
}

impl Default for PmCredentialsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auth_url: String::new(),
            machine_subject: String::new(),
            parent_pat: String::new(),
            ttl_seconds: 300,
        }
    }
}

impl std::fmt::Debug for PmCredentialsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PmCredentialsConfig")
            .field("enabled", &self.enabled)
            .field("auth_url", &self.auth_url)
            .field("machine_subject", &self.machine_subject)
            .field("parent_pat", &"[REDACTED]")
            .field("ttl_seconds", &self.ttl_seconds)
            .finish()
    }
}

impl std::fmt::Debug for PmConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PmConfig")
            .field("readback_token", &"[REDACTED]")
            .field("namespace_read_pat", &"[REDACTED]")
            .field(
                "namespace_authority_issuer",
                &self.namespace_authority_issuer,
            )
            .field(
                "namespace_provisioner_subject",
                &self.namespace_provisioner_subject,
            )
            .field("credentials", &self.credentials)
            .finish()
    }
}

pub use sdlc_shared::DatabaseConfig;

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
#[serde(deny_unknown_fields)]
pub struct ContainerControlConfig {
    pub python: String,
    pub base_root: String,
    pub source_sha256: [String; 3],
    pub context: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provisioning: Option<ContainerProvisioningConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_controller: Option<BridgeControllerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeControllerConfig {
    pub container_id: String,
    pub image_id: String,
    pub service: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerProvisioningConfig {
    pub project: String,
    pub image_id: String,
    pub user: String,
    pub entrypoint: Vec<String>,
    pub pids_limit: u32,
    pub memory_bytes: u64,
    pub nano_cpus: u64,
    pub network_internal: bool,
    pub task: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetConfig {
    /// Operator-owned local Git object cache. Empty disables Base package preparation.
    #[serde(default)]
    pub base_package_checkout: String,
    pub agents_root: String,
    /// Private operator-provisioned controller storage, never mounted into an agent.
    #[serde(default)]
    pub controller_root: String,
    /// Resume original controller custody only; not a model or SDLC admission flag.
    #[serde(default)]
    pub controller_recovery_enabled: bool,
    /// Opt-in reconciliation of signed configuration journals; never grants task admission.
    #[serde(default)]
    pub configuration_recovery_enabled: bool,
    /// Operator opt-in. Missing Docker preparation never falls back to a host process.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_control: Option<ContainerControlConfig>,
    pub hermes_source: String,
    pub hermes_command: String,
    /// Require the opt-in, source-pinned original-key recovery extension.
    #[serde(default)]
    pub hermes_recovery_extension_enabled: bool,
    /// Require durable original stop/steer outcomes. Never backfill legacy commands.
    #[serde(default)]
    pub hermes_control_outcome_enabled: bool,
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
    /// Pulse API health endpoint reachable from Fleet, checked after Forge completes.
    pub pulse_health_url: Option<String>,
    pub pulse_ui_url: Option<String>,
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
            .set_default("fleet.controller_root", "")?
            .set_default("fleet.controller_recovery_enabled", false)?
            .set_default("fleet.configuration_recovery_enabled", false)?
            .set_default("fleet.hermes_source", "../прототипы/hermes")?
            .set_default("fleet.hermes_command", "hermes")?
            .set_default("fleet.hermes_recovery_extension_enabled", false)?
            .set_default("fleet.hermes_control_outcome_enabled", false)?
            .set_default("fleet.java_agent_source", "../java-agent")?
            .set_default("fleet.java_agent_command", "java")?
            .set_default("fleet.runtime_token_secret", "[CHANGE_ME]")?
            .set_default("fleet.agent_port_base", 29000u16)?
            .set_default("fleet.agent_port_stride", 10u16)?
            .set_default("fleet.forge_api_url", Option::<String>::None)?
            .set_default("fleet.forge_api_token", Option::<String>::None)?
            .set_default("fleet.forge_project", Option::<String>::None)?
            .set_default("fleet.pulse_health_url", Option::<String>::None)?
            .set_default("fleet.pulse_ui_url", Option::<String>::None)?
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
            base_package_checkout: String::new(),
            agents_root: "./data/agents".to_string(),
            controller_root: String::new(),
            controller_recovery_enabled: false,
            configuration_recovery_enabled: false,
            container_control: None,
            hermes_source: "../прототипы/hermes".to_string(),
            hermes_command: "hermes".to_string(),
            hermes_recovery_extension_enabled: false,
            hermes_control_outcome_enabled: false,
            java_agent_source: "../java-agent".to_string(),
            java_agent_command: "java".to_string(),
            forge_api_url: None,
            forge_api_token: None,
            forge_project: None,
            pulse_health_url: None,
            pulse_ui_url: None,
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

#[cfg(test)]
mod control_outcome_tests {
    use super::*;

    #[test]
    fn control_outcome_is_opt_in_for_defaults_and_legacy_config() {
        assert!(!FleetConfig::default().hermes_control_outcome_enabled);
        let mut legacy = serde_json::to_value(FleetConfig::default()).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .remove("hermes_control_outcome_enabled");
        assert!(
            !serde_json::from_value::<FleetConfig>(legacy)
                .unwrap()
                .hermes_control_outcome_enabled
        );
    }
}
