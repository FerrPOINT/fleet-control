use domain::{Agent, AgentKind, SdlcRole};
use serde_json::{Value, json};
use shared::{AppConfig, AppError};

pub(crate) fn origin(config: &AppConfig) -> Result<String, AppError> {
    let raw = config
        .pm
        .dispatch
        .tool_origin
        .as_deref()
        .ok_or_else(|| AppError::Unavailable("PM MCP tool origin is not configured".into()))?;
    let url =
        reqwest::Url::parse(raw).map_err(|_| AppError::validation("invalid PM MCP origin"))?;
    if raw.trim() != raw
        || raw.chars().any(|c| c.is_whitespace() || c.is_control())
        || raw.contains('\\')
        || !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(AppError::validation("invalid PM MCP origin"));
    }
    Ok(url.as_str().trim_end_matches('/').into())
}

/// Only references the already isolated native runtime credential; no upstream bearer is materialized.
pub(crate) fn configure(
    content: &mut Value,
    agent: &Agent,
    config: &AppConfig,
) -> Result<(), AppError> {
    if !config.pm.dispatch.enabled
        || agent.kind != AgentKind::Hermes
        || agent.sdlc_role != Some(SdlcRole::ProjectManager)
    {
        return Ok(());
    }
    let entry = json!({"url":format!("{}/internal/runtime/v1/pm/agents/{}/mcp",origin(config)?,agent.id),
        "transport":"http","headers":{"Authorization":"Bearer ${API_SERVER_KEY}","MCP-Protocol-Version":"2025-03-26"},
        "strict_redirect_headers":true,"trust":"full"});
    if !content.is_object()
        || content.get("mcp_servers").is_some_and(|v| !v.is_object())
        || content
            .get("platform_toolsets")
            .is_some_and(|v| !v.is_object())
        || content
            .pointer("/mcp_servers/fleet_pm")
            .is_some_and(|v| v != &entry)
        || content
            .pointer("/platform_toolsets/api_server")
            .is_some_and(|v| v != &json!(["fleet_pm"]))
    {
        return Err(AppError::conflict(
            "PM MCP configuration conflicts with the isolated tool profile",
        ));
    }
    content["mcp_servers"]["fleet_pm"] = entry;
    content["platform_toolsets"]["api_server"] = json!(["fleet_pm"]);
    Ok(())
}

pub(crate) fn reject_server_secrets(value: &str, config: &AppConfig) -> Result<(), AppError> {
    for secret in [
        &config.pm.credentials.parent_pat,
        &config.pm.dispatch.assignment_token,
        &config.pm.dispatch.runtime_token,
        &config.pm.readback_token,
        &config.pm.namespace_read_pat,
        &config.auth.jwt_secret,
        &config.fleet.runtime_token_secret,
    ] {
        if secret.len() >= 16 && value.contains(secret) {
            return Err(AppError::validation(
                "server credential cannot enter the PM runtime profile",
            ));
        }
    }
    Ok(())
}

pub(crate) async fn verify(
    repo: &dyn app::FleetRepository,
    agent: &Agent,
    config: &AppConfig,
) -> Result<(), AppError> {
    use app::AgentProvisioner;
    origin(config)?;
    if repo.agent_is_draining(agent.id).await? {
        return Err(AppError::conflict("PM configuration is draining"));
    }
    let revision = repo
        .get_effective_config_revision(agent.id)
        .await?
        .ok_or_else(|| {
            AppError::Unavailable(
                "activate the isolated PM tool configuration before dispatch".into(),
            )
        })?;
    crate::FilesystemProvisioner
        .verify_effective_configuration(agent, config, &revision)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn origin_cannot_embed_a_credential_path_or_redirect_query() {
        let mut config = AppConfig::default();
        for raw in [
            "https://user:secret@fleet/",
            "file:///tmp",
            "https://fleet/path",
            "https://fleet/?secret=x",
            "https://fleet/#x",
            " https://fleet/",
            "https://fleet\\evil/",
        ] {
            config.pm.dispatch.tool_origin = Some(raw.into());
            assert!(origin(&config).is_err());
        }
        config.pm.dispatch.tool_origin = Some("https://fleet/".into());
        assert_eq!(origin(&config).unwrap(), "https://fleet");
    }

    #[test]
    fn server_credentials_are_rejected_without_returning_the_secret() {
        let mut config = AppConfig::default();
        config.pm.credentials.parent_pat = "sdlc_pat_server-only-test-secret".into();
        let error = reject_server_secrets(
            &format!("prefix {} suffix", config.pm.credentials.parent_pat),
            &config,
        )
        .unwrap_err()
        .to_string();
        assert!(!error.contains(&config.pm.credentials.parent_pat));
        assert!(reject_server_secrets("Bearer ${API_SERVER_KEY}", &config).is_ok());
    }
}
