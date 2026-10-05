use super::*;

#[test]
fn defaults_are_fleet_control_specific() {
    let cfg = AppConfig::default();

    assert_eq!(cfg.server.port, 23801);
    assert_eq!(cfg.auth.mode, "hmac");
    assert_eq!(cfg.auth.jwt_issuer, "fleet-control");
    assert_eq!(cfg.auth.jwt_audience, "sdlc");
    assert_eq!(cfg.fleet.agent_port_base, 29000);
    assert_eq!(cfg.fleet.agent_port_stride, 10);
    assert!(cfg.fleet.base_package_checkout.is_empty());
    assert!(cfg.fleet.controller_root.is_empty());
    assert!(cfg.pm.readback_token.is_empty());
    assert!(cfg.tracker.url.is_empty());
    assert!(cfg.tracker.instance_id.is_empty());
    assert!(
        cfg.server
            .cors_allowed_origins
            .iter()
            .any(|origin| origin.contains("23802"))
    );
}

#[test]
fn controller_storage_is_operator_configured_and_preserves_legacy_config() {
    let mut legacy = serde_json::to_value(FleetConfig::default()).unwrap();
    legacy.as_object_mut().unwrap().remove("controller_root");
    let restored: FleetConfig = serde_json::from_value(legacy.clone()).unwrap();
    assert!(restored.controller_root.is_empty());
    legacy["controller_root"] = serde_json::json!("/operator/controller");
    let configured: FleetConfig = serde_json::from_value(legacy).unwrap();
    assert_eq!(configured.controller_root, "/operator/controller");
}

#[test]
fn sdlc_configuration_reader_is_opt_in_and_preserves_legacy_config() {
    let mut legacy = serde_json::to_value(AppConfig::default()).unwrap();
    legacy.as_object_mut().unwrap().remove("sdlc");
    let restored: AppConfig = serde_json::from_value(legacy).unwrap();
    assert!(!restored.sdlc.configuration_readback_enabled);
    assert!(restored.sdlc.auth_url.is_empty());
    assert!(restored.sdlc.configuration_reader_subject.is_empty());
    assert!(restored.sdlc.configuration_reader_agent_ids.is_empty());
    assert!(restored.sdlc.workflow_binding.url.is_empty());
    assert!(restored.sdlc.workflow_binding.read_pat.is_empty());
    let config: SdlcConfig = serde_json::from_value(serde_json::json!({
        "configuration_readback_enabled":true,
        "auth_url":"http://auth.example.test",
        "configuration_reader_subject":"eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee",
        "configuration_reader_agent_ids":"dddddddd-dddd-4ddd-8ddd-dddddddddddd"
    }))
    .unwrap();
    assert!(config.configuration_readback_enabled);
    assert_eq!(
        config.configuration_reader_agent_ids,
        "dddddddd-dddd-4ddd-8ddd-dddddddddddd"
    );
}

#[test]
fn sdlc_workflow_binding_credential_is_server_only() {
    let config = SdlcWorkflowConfig {
        url: "http://workflow.example.test".into(),
        read_pat: "fixture-workflow-read-pat".into(),
    };
    assert!(!format!("{config:?}").contains(&config.read_pat));
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.contains(&config.read_pat));
    assert!(!json.contains("read_pat"));
}

#[test]
fn base_package_checkout_is_opt_in_and_preserves_legacy_config() {
    let mut legacy = serde_json::to_value(FleetConfig::default()).unwrap();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("base_package_checkout");
    let restored: FleetConfig = serde_json::from_value(legacy.clone()).unwrap();
    assert!(restored.base_package_checkout.is_empty());
    legacy["base_package_checkout"] = serde_json::json!("/operator/cache");
    let configured: FleetConfig = serde_json::from_value(legacy).unwrap();
    assert_eq!(configured.base_package_checkout, "/operator/cache");
}

#[test]
fn tracker_configuration_defaults_and_round_trips_without_process_environment() {
    let mut legacy = serde_json::to_value(AppConfig::default()).unwrap();
    legacy.as_object_mut().unwrap().remove("tracker");
    let restored: AppConfig = serde_json::from_value(legacy).unwrap();
    assert!(restored.tracker.url.is_empty());
    assert!(!restored.tracker.pm_draft_creation_enabled);
    assert!(restored.tracker.pm_draft_project_ids.is_empty());
    assert!(!restored.tracker.events.enabled);
    let cfg: TrackerConfig = serde_json::from_value(serde_json::json!({
        "url":"http://tracker.example.test:8080", "instance_id":"tracker-one"
    }))
    .unwrap();
    assert_eq!(cfg.instance_id, "tracker-one");
    assert_eq!(cfg.url, "http://tracker.example.test:8080");
}

#[test]
fn tracker_event_credentials_are_server_only_and_redacted() {
    let cfg = TrackerConfig {
        events: TrackerEventsConfig {
            read_pat: "test-only-read-pat-secret".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(!format!("{cfg:?}").contains(&cfg.events.read_pat));
    let json = serde_json::to_value(&cfg).unwrap();
    assert!(json["events"].get("read_pat").is_none());
    assert_eq!(cfg.events.poll_interval_seconds, 5);
}

#[test]
fn pm_config_debug_does_not_disclose_readback_credential() {
    let cfg = PmConfig {
        readback_token: "test-only-pm-readback-secret".into(),
        namespace_read_pat: "test-only-namespace-read-secret".into(),
        ..Default::default()
    };
    let debug = format!("{cfg:?}");
    assert!(!debug.contains(&cfg.readback_token));
    assert!(!debug.contains(&cfg.namespace_read_pat));
    assert!(debug.contains("[REDACTED]"));
    let serialized = serde_json::to_value(&cfg).unwrap();
    assert!(serialized.get("readback_token").is_none());
    assert!(serialized.get("namespace_read_pat").is_none());
}

#[test]
fn pm_credentials_default_disabled_and_never_serialize_parent_secret() {
    let legacy: PmConfig = serde_json::from_value(serde_json::json!({})).unwrap();
    assert!(!legacy.credentials.enabled);
    assert_eq!(legacy.credentials.ttl_seconds, 300);
    let mut cfg = legacy;
    cfg.credentials.parent_pat = "test-only-parent-secret".into();
    let debug = format!("{cfg:?}");
    assert!(!debug.contains(&cfg.credentials.parent_pat));
    let value = serde_json::to_value(&cfg).unwrap();
    assert!(value["credentials"].get("parent_pat").is_none());
}

#[test]
fn default_config_requires_jwt_secret_override() {
    let err = AppConfig::from_path("missing-test-config.toml").unwrap_err();
    assert!(err.to_string().contains("jwt_secret"));
}

#[test]
fn oidc_auth_mode_is_reserved_until_shared_validator_is_enabled() {
    let path = std::env::temp_dir().join(format!(
        "fleet-control-auth-mode-{}.toml",
        uuid::Uuid::new_v4()
    ));
    std::fs::write(
        &path,
        r#"
[auth]
mode = "oidc"
jwt_secret = "test-jwt-secret"
jwt_issuer = "fleet-control"
jwt_audience = "sdlc"

[fleet]
runtime_token_secret = "test-runtime-secret"
"#,
    )
    .expect("test config");

    let err = AppConfig::from_path(&path).unwrap_err();
    assert!(err.to_string().contains("auth.mode"));
    let _ = std::fs::remove_file(path);
}
