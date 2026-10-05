use app::FleetRepository;
use domain::{SystemRole, UpdateUserRoleRequest};
use infra::{PostgresFleetRepository, connect_database, run_migrations};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement, TransactionTrait};
use shared::DatabaseConfig;
use std::sync::Arc;
use uuid::Uuid;

async fn fixture() -> (Arc<PostgresFleetRepository>, DatabaseConnection) {
    let config = DatabaseConfig {
        url: std::env::var("FLEET_TEST_DATABASE_URL").expect("isolated PostgreSQL is required"),
        max_connections: 10,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    };
    run_migrations(config.clone()).await.unwrap();
    let db = connect_database(config.clone()).await.unwrap();
    let repository_db = connect_database(config).await.unwrap();
    (Arc::new(PostgresFleetRepository::new(repository_db)), db)
}

async fn timestamps(db: &DatabaseConnection, id: Uuid) -> (String, String) {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT created_at::text, updated_at::text FROM users WHERE id = $1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    (
        row.try_get("", "created_at").unwrap(),
        row.try_get("", "updated_at").unwrap(),
    )
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn central_name_refresh_preserves_identity_role_and_historical_same_email() {
    let (repo, db) = fixture().await;
    let legacy_id = Uuid::new_v4();
    let email = format!("profile-{legacy_id}@example.test");
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,system_role,is_system_admin) \
         VALUES ($1,$2,$3,'Historical Human','!', 'admin',true)",
        [legacy_id.into(), email.clone().into(), legacy_id.to_string().into()],
    ))
    .await
    .unwrap();
    let subject = Uuid::new_v4().to_string();
    let initial = repo
        .find_or_create_central_user(&subject, &email, "Central Human")
        .await
        .unwrap();
    assert_ne!(initial.id, legacy_id);
    repo.update_user_role(
        initial.id,
        UpdateUserRoleRequest {
            role: SystemRole::Operator,
        },
    )
    .await
    .unwrap();
    let created = timestamps(&db, initial.id).await.0;
    let renamed = repo
        .find_or_create_central_user(&subject, &email, " Renamed Central Human ")
        .await
        .unwrap();
    assert_eq!(renamed.display_name, "Renamed Central Human");
    assert_eq!(renamed.id, initial.id);
    assert_eq!(renamed.email, initial.email);
    assert_eq!(renamed.username, initial.username);
    assert_eq!(renamed.system_role, SystemRole::Operator);
    assert!(!renamed.is_system_admin);
    assert_eq!(timestamps(&db, initial.id).await.0, created);
    let refreshed = timestamps(&db, initial.id).await;
    let again = repo
        .find_or_create_central_user(&subject, &email, "Renamed Central Human")
        .await
        .unwrap();
    assert_eq!(again.id, initial.id);
    assert_eq!(timestamps(&db, initial.id).await, refreshed);
    assert!(
        repo.find_or_create_central_user(&subject, &email, " ")
            .await
            .is_err()
    );
    assert_eq!(timestamps(&db, initial.id).await, refreshed);
    let historical = repo.find_user_by_id(legacy_id).await.unwrap().unwrap();
    assert_eq!(historical.display_name, "Historical Human");
    assert_eq!(historical.system_role, SystemRole::Admin);
    let distinct = repo
        .find_or_create_central_user(&Uuid::new_v4().to_string(), &email, "Distinct Human")
        .await
        .unwrap();
    assert_ne!(distinct.id, initial.id);
    assert_ne!(distinct.id, legacy_id);
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn unchanged_central_profile_does_not_wait_for_user_write_lock() {
    let (repo, db) = fixture().await;
    let subject = Uuid::new_v4().to_string();
    let initial = repo
        .find_or_create_central_user(&subject, "profile-lock@example.test", "Central Human")
        .await
        .unwrap();
    let before = timestamps(&db, initial.id).await;
    let blocker = db.begin().await.unwrap();
    blocker
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM users WHERE id = $1 FOR UPDATE",
            [initial.id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let mut lookup = tokio::spawn(async move {
        repo.find_or_create_central_user(&subject, "profile-lock@example.test", "Central Human")
            .await
    });
    let resolved = tokio::time::timeout(std::time::Duration::from_secs(3), &mut lookup).await;
    let completed_without_lock = resolved.is_ok();
    blocker.rollback().await.unwrap();
    let user = match resolved {
        Ok(result) => result.unwrap().unwrap(),
        Err(_) => tokio::time::timeout(std::time::Duration::from_secs(10), lookup)
            .await
            .expect("lookup completes after releasing the lock")
            .unwrap()
            .unwrap(),
    };
    assert!(
        completed_without_lock,
        "unchanged central profile must not acquire a user write lock"
    );
    assert_eq!(user.id, initial.id);
    assert_eq!(timestamps(&db, initial.id).await, before);
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn inactive_central_profile_is_not_reactivated_or_overwritten() {
    let (repo, db) = fixture().await;
    let subject = Uuid::new_v4().to_string();
    let initial = repo
        .find_or_create_central_user(&subject, "inactive-profile@example.test", "Central Human")
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active = false WHERE id = $1",
        [initial.id.into()],
    ))
    .await
    .unwrap();
    let before = timestamps(&db, initial.id).await;
    assert!(
        repo.find_or_create_central_user(
            &subject,
            "inactive-profile@example.test",
            "Renamed Human"
        )
        .await
        .is_err()
    );
    let inactive = repo.find_user_by_id(initial.id).await.unwrap().unwrap();
    assert!(!inactive.is_active);
    assert_eq!(inactive.display_name, initial.display_name);
    assert_eq!(timestamps(&db, initial.id).await, before);
}
