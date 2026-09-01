use super::*;

#[test]
fn defaults_are_fleet_control_specific() {
    let cfg = AppConfig::default();

    assert_eq!(cfg.server.port, 23801);
    assert_eq!(cfg.fleet.agent_port_base, 29000);
    assert_eq!(cfg.fleet.agent_port_stride, 10);
    assert!(
        cfg.server
            .cors_allowed_origins
            .iter()
            .any(|origin| origin.contains("23802"))
    );
}

#[test]
fn default_config_requires_jwt_secret_override() {
    let err = AppConfig::from_path("missing-test-config.toml").unwrap_err();
    assert!(err.to_string().contains("jwt_secret"));
}
