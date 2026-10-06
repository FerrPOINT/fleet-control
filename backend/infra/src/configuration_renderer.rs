use domain::Agent;
use serde_json::{Map, Value, json};
use shared::{AppConfig, AppError};

pub(crate) fn native_listener(
    agent: &Agent,
    config: &AppConfig,
    content: &mut Value,
) -> Result<(), AppError> {
    let port = agent
        .api_port
        .filter(|port| (1024..=65535).contains(port))
        .ok_or_else(|| AppError::validation("managed Hermes API port is required"))?;
    let root = content
        .as_object_mut()
        .ok_or_else(|| AppError::validation("Hermes configuration must be an object"))?;
    let platforms = object(root, "platforms")?;
    let api = object(platforms, "api_server")?;
    api.insert("enabled".into(), Value::Bool(true));
    api.remove("key");
    api.remove("cors_origins");
    let extra = object(api, "extra")?;
    extra.insert("host".into(), json!(listener_host(config)));
    extra.insert("port".into(), json!(port));
    // Empty dotenv CORS does not override YAML in the native bridge.
    extra.insert(
        "cors_origins".into(),
        json!(config.server.cors_allowed_origins),
    );
    // Native dotenv/config-env bridge supplies the derived credential, not user JSON.
    extra.remove("key");
    Ok(())
}

fn object<'a>(
    parent: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>, AppError> {
    parent
        .entry(key)
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| {
            AppError::validation("Hermes API platform configuration must contain objects")
        })
}

pub(crate) fn managed_env_key(key: &str) -> bool {
    matches!(
        key,
        "HERMES_HOME"
            | "HERMES_SERVE_HEADLESS"
            | "API_SERVER_ENABLED"
            | "API_SERVER_KEY"
            | "API_SERVER_HOST"
            | "API_SERVER_PORT"
            | "API_SERVER_CORS_ORIGINS"
    )
}

pub(crate) fn env(agent: &Agent, config: &AppConfig) -> Result<String, AppError> {
    let port = agent
        .api_port
        .filter(|port| (1024..=65535).contains(port))
        .ok_or_else(|| AppError::validation("managed Hermes API port is required"))?;
    let mut env = String::from("# Managed by Fleet Control renderer v2.\n");
    for (key, value) in [
        (
            "HERMES_HOME",
            if config.fleet.container_control.is_some() {
                "/config".into()
            } else {
                agent.paths.config.clone()
            },
        ),
        ("HERMES_SERVE_HEADLESS", "1".into()),
        ("API_SERVER_ENABLED", "true".into()),
        (
            "API_SERVER_KEY",
            crate::agent_runtime_token(config, agent.id)?,
        ),
        ("API_SERVER_HOST", listener_host(config).into()),
        ("API_SERVER_PORT", port.to_string()),
        (
            "API_SERVER_CORS_ORIGINS",
            config.server.cors_allowed_origins.join(","),
        ),
    ] {
        env.push_str(key);
        env.push('=');
        env.push_str(&serde_json::to_string(&value).map_err(AppError::internal)?);
        env.push('\n');
    }
    Ok(env)
}

fn listener_host(config: &AppConfig) -> &'static str {
    if config.fleet.container_control.is_some() {
        "0.0.0.0"
    } else {
        "127.0.0.1"
    }
}

pub(crate) fn workspace(agent: &Agent, config: &AppConfig) -> String {
    if agent.kind == domain::AgentKind::Hermes && config.fleet.container_control.is_some() {
        "/workspace".into()
    } else {
        agent.paths.workspace.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::AgentStatus;
    use std::path::Path;
    use uuid::Uuid;

    #[test]
    fn native_listener_seals_managed_address_but_preserves_unrelated_settings() {
        let agent =
            crate::tests::test_agent(Path::new("unused"), Uuid::new_v4(), AgentStatus::Ready);
        let mut content = json!({"model":"fixture","platforms":{"api_server":{
            "enabled":false,"key":"fixture-only","cors_origins":["*"],
            "extra":{"host":"0.0.0.0","port":80,"key":"fixture-only","cors_origins":["*"],
                "direct_model_requests":true}},"telegram":{"enabled":false}}});
        let mut config = AppConfig::default();
        config.server.cors_allowed_origins.clear();
        native_listener(&agent, &config, &mut content).unwrap();
        assert_eq!(content["platforms"]["api_server"]["enabled"], true);
        assert_eq!(
            content["platforms"]["api_server"]["extra"]["host"],
            "127.0.0.1"
        );
        assert_eq!(content["platforms"]["api_server"]["extra"]["port"], 29002);
        assert_eq!(
            content["platforms"]["api_server"]["extra"]["cors_origins"],
            json!([])
        );
        assert!(content["platforms"]["api_server"].get("key").is_none());
        assert!(
            content["platforms"]["api_server"]
                .get("cors_origins")
                .is_none()
        );
        assert!(
            content["platforms"]["api_server"]["extra"]
                .get("key")
                .is_none()
        );
        assert_eq!(
            content["platforms"]["api_server"]["extra"]["direct_model_requests"],
            true
        );
        assert_eq!(content["platforms"]["telegram"]["enabled"], false);
        assert_eq!(content["model"], "fixture");
    }

    #[test]
    fn invalid_api_structure_or_port_never_panics_or_renders() {
        let mut agent =
            crate::tests::test_agent(Path::new("unused"), Uuid::new_v4(), AgentStatus::Ready);
        for mut content in [
            json!([]),
            json!({"platforms":false}),
            json!({"platforms":{"api_server":null}}),
            json!({"platforms":{"api_server":{"extra":[]}}}),
        ] {
            assert!(native_listener(&agent, &AppConfig::default(), &mut content).is_err());
        }
        for port in [None, Some(0), Some(80), Some(65536)] {
            agent.api_port = port;
            assert!(native_listener(&agent, &AppConfig::default(), &mut json!({})).is_err());
            assert!(env(&agent, &AppConfig::default()).is_err());
        }
    }

    #[test]
    fn container_listener_uses_only_container_paths_and_bridge_address() {
        let agent = crate::tests::test_agent(
            Path::new("/controller-visible/agents"),
            Uuid::new_v4(),
            AgentStatus::Ready,
        );
        let mut config = AppConfig::default();
        config.fleet.runtime_token_secret = "fixture-only-not-production".into();
        config.fleet.container_control = Some(shared::config::ContainerControlConfig {
            python: "python3".into(),
            base_root: "/trusted/base".into(),
            source_sha256: ["a".repeat(64), "b".repeat(64), "c".repeat(64)],
            context: "default".into(),
            provisioning: None,
        });
        let mut content = json!({});
        native_listener(&agent, &config, &mut content).unwrap();
        assert_eq!(
            content["platforms"]["api_server"]["extra"]["host"],
            "0.0.0.0"
        );
        assert_eq!(workspace(&agent, &config), "/workspace");
        let env = env(&agent, &config).unwrap();
        assert!(env.contains("HERMES_HOME=\"/config\"\n"));
        assert!(env.contains("API_SERVER_HOST=\"0.0.0.0\"\n"));
        assert!(!env.contains("controller-visible"));
    }
}
