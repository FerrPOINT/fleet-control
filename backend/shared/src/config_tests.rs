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
