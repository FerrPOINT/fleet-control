use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
use serde_json::Value;

#[tokio::test]
#[ignore = "requires isolated FLEET_HERMES_TIME_MIGRATION_TEST_DATABASE_URL"]
async fn journal_time_order_additive_upgrade_preserves_history_and_original_guard() {
    let url = std::env::var("FLEET_HERMES_TIME_MIGRATION_TEST_DATABASE_URL")
        .expect("isolated PostgreSQL is required for journal time migration");
    let db = Database::connect(url).await.unwrap();
    let empty = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT NOT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE') AS empty"))
        .await.unwrap().unwrap().try_get::<bool>("","empty").unwrap();
    assert!(
        empty,
        "time-order migration requires its own empty disposable database"
    );
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261005_000014_hermes_journal_time_order")
        .unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    db.execute_unprepared("INSERT INTO users(id,email,username,display_name,password_hash)
        VALUES('aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','clock@example.test','clock-migration','Migration','disabled')").await.unwrap();
    let snapshot = || {
        Statement::from_string(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
            'guard',pg_get_functiondef('fleet_guard_hermes_dispatch()'::regprocedure)) AS value")
    };
    let original: Value = db
        .query_one(snapshot())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap();
    Migrator::up(&db, Some(1)).await.unwrap();
    assert_eq!(
        db.query_one(snapshot())
            .await
            .unwrap()
            .unwrap()
            .try_get::<Value>("", "value")
            .unwrap(),
        original
    );
    let order: Value = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT jsonb_agg(tgname ORDER BY tgname) AS value FROM pg_trigger
         WHERE tgrelid='hermes_dispatch_journal'::regclass AND NOT tgisinternal",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap();
    assert_eq!(
        order,
        serde_json::json!([
            "fleet_hermes_dispatch_guard",
            "fleet_hermes_dispatch_time_order"
        ])
    );
    Migrator::down(&db, Some(1)).await.unwrap();
    assert_eq!(
        db.query_one(snapshot())
            .await
            .unwrap()
            .unwrap()
            .try_get::<Value>("", "value")
            .unwrap(),
        original
    );
    let absent: bool = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT to_regprocedure('fleet_order_hermes_dispatch_time()') IS NULL AS absent",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "absent")
        .unwrap();
    assert!(absent);
    Migrator::up(&db, Some(1)).await.unwrap();
    assert_eq!(
        db.query_one(snapshot())
            .await
            .unwrap()
            .unwrap()
            .try_get::<Value>("", "value")
            .unwrap(),
        original
    );
}
