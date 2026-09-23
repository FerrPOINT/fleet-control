use app::{AuditLogFilter, FleetRepository, managed_settings_from_config};
use infra::{PostgresFleetRepository, connect_database, run_migrations};
use sea_orm::ConnectionTrait;
use shared::{AppConfig, DatabaseConfig};
use uuid::Uuid;

fn test_database(url: String) -> DatabaseConfig {
    DatabaseConfig {
        url,
        max_connections: 5,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    }
}

#[tokio::test]
async fn managed_settings_are_versioned_atomic_and_rollbackable() {
    let Ok(url) = std::env::var("FLEET_TEST_DATABASE_URL") else {
        eprintln!("FLEET_TEST_DATABASE_URL is not configured; skipping PostgreSQL test");
        return;
    };
    let database = test_database(url);
    run_migrations(database.clone())
        .await
        .expect("run fleet migrations");
    let connection = connect_database(database)
        .await
        .expect("connect test database");
    connection
        .execute_unprepared(
            "TRUNCATE managed_settings_versions, audit_log, users RESTART IDENTITY CASCADE",
        )
        .await
        .expect("reset managed settings fixtures");

    let actor_id = Uuid::new_v4();
    connection
        .execute_unprepared(&format!(
            "INSERT INTO users (id, email, username, display_name, password_hash, is_system_admin) \
             VALUES ('{actor_id}', 'settings@example.test', 'settings-test', 'Settings Test', 'disabled', true)"
        ))
        .await
        .expect("insert settings actor");
    let repo = PostgresFleetRepository::new(connection);

    let base = AppConfig::default();
    let first_snapshot = managed_settings_from_config(&base);
    let first = repo
        .activate_managed_settings(
            first_snapshot.clone(),
            actor_id,
            None,
            None,
            "settings.apply",
        )
        .await
        .expect("activate initial settings");
    assert_eq!(first.version, 1);
    assert!(first.is_active);

    let mut second_snapshot = first_snapshot.clone();
    second_snapshot.runtime.hermes_command = "hermes-next".to_string();
    let stale = repo
        .activate_managed_settings(
            second_snapshot.clone(),
            actor_id,
            None,
            None,
            "settings.apply",
        )
        .await
        .expect_err("stale writer must be rejected");
    assert!(stale.to_string().contains("changed concurrently"));
    assert_eq!(
        repo.get_active_managed_settings()
            .await
            .expect("load active settings")
            .expect("active settings")
            .version,
        first.version
    );

    let second = repo
        .activate_managed_settings(
            second_snapshot,
            actor_id,
            Some(first.version),
            None,
            "settings.apply",
        )
        .await
        .expect("activate second settings");
    let rollback = repo
        .activate_managed_settings(
            first_snapshot,
            actor_id,
            Some(second.version),
            Some(first.version),
            "settings.rollback",
        )
        .await
        .expect("rollback settings");
    assert_eq!(rollback.rollback_of_version, Some(first.version));
    assert_eq!(rollback.version, 3);

    let failed = repo
        .activate_managed_settings(
            rollback.snapshot.clone(),
            Uuid::new_v4(),
            Some(rollback.version),
            None,
            "settings.apply",
        )
        .await
        .expect_err("missing audit actor must roll back the transaction");
    assert!(failed.to_string().contains("database"));
    let active = repo
        .get_active_managed_settings()
        .await
        .expect("load active after rollback failure")
        .expect("active settings after rollback failure");
    assert_eq!(active.version, rollback.version);
    assert!(active.is_active);

    let versions = repo
        .list_managed_settings_versions(20)
        .await
        .expect("list settings history");
    assert_eq!(
        versions.iter().map(|item| item.version).collect::<Vec<_>>(),
        vec![3, 2, 1]
    );
    assert_eq!(versions.iter().filter(|item| item.is_active).count(), 1);

    let audits = repo
        .list_audit_log(AuditLogFilter {
            entity_type: Some("managed_settings_version".to_string()),
            limit: 20,
            ..AuditLogFilter::default()
        })
        .await
        .expect("list settings audit");
    assert_eq!(audits.len(), 3);
    assert_eq!(audits[0].action, "settings.rollback");
}
