use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

#[tokio::test]
#[ignore = "requires empty FLEET_RECOVERED_ACTIVATION_MIGRATION_TEST_DATABASE_URL"]
async fn recovered_activation_additive_roundtrip_preserves_original_recovery_guard_and_ledger() {
    let url = std::env::var("FLEET_RECOVERED_ACTIVATION_MIGRATION_TEST_DATABASE_URL")
        .expect("own empty migration database required");
    assert!(url.ends_with("/fleet_recovered_activation_migration_test"));
    let db = Database::connect(url).await.unwrap();
    assert!(
        Migrator::get_migration_models(&db)
            .await
            .unwrap()
            .is_empty()
    );
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261009_000019_recovered_activation")
        .unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    let ledger = Migrator::get_migration_models(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|m| (m.version, m.applied_at))
        .collect::<Vec<_>>();
    let read =
        || {
            Statement::from_string(DbBackend::Postgres,
        "SELECT pg_get_functiondef('fleet_guard_container_recovery()'::regprocedure) AS body,
        'fleet_guard_container_recovery()'::regprocedure::oid::bigint AS identity")
        };
    let before = db.query_one(read()).await.unwrap().unwrap();
    let custody_read =
        || {
            Statement::from_string(DbBackend::Postgres,
        "SELECT pg_get_functiondef('fleet_container_custody_live(uuid)'::regprocedure) AS body,
        'fleet_container_custody_live(uuid)'::regprocedure::oid::bigint AS identity")
        };
    let custody = db.query_one(custody_read()).await.unwrap().unwrap();
    let custody_body: String = custody.try_get("", "body").unwrap();
    let custody_oid: i64 = custody.try_get("", "identity").unwrap();
    let body: String = before.try_get("", "body").unwrap();
    let identity: i64 = before.try_get("", "identity").unwrap();
    Migrator::up(&db, Some(1)).await.unwrap();
    let after = db.query_one(read()).await.unwrap().unwrap();
    let inherited = db.query_one(custody_read()).await.unwrap().unwrap();
    assert_eq!(
        inherited.try_get::<i64>("", "identity").unwrap(),
        custody_oid
    );
    assert!(
        inherited
            .try_get::<String>("", "body")
            .unwrap()
            .contains("fleet_activation_parent(g)")
    );
    assert_eq!(after.try_get::<i64>("", "identity").unwrap(), identity);
    let widened = after.try_get::<String>("", "body").unwrap();
    assert_eq!(widened.replace("(c.state NOT IN ('running','stopping') AND NOT (c.state='exited' AND fleet_activation_anchor(c.generation)))",
        "c.state NOT IN ('running','stopping')"),body);
    assert_eq!(Migrator::get_migration_models(&db).await.unwrap().len(), 20);
    Migrator::down(&db, Some(1)).await.unwrap();
    let restored = db.query_one(custody_read()).await.unwrap().unwrap();
    assert_eq!(
        restored.try_get::<String>("", "body").unwrap(),
        custody_body
    );
    assert_eq!(
        restored.try_get::<i64>("", "identity").unwrap(),
        custody_oid
    );
    assert_eq!(
        db.query_one(read())
            .await
            .unwrap()
            .unwrap()
            .try_get::<String>("", "body")
            .unwrap(),
        body
    );
    assert_eq!(
        Migrator::get_migration_models(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|m| (m.version, m.applied_at))
            .collect::<Vec<_>>(),
        ledger
    );
    Migrator::up(&db, Some(1)).await.unwrap();
    assert!(
        !db.query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT fleet_activation_anchor('00000000-0000-0000-0000-000000000001') AS held"
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<bool>("", "held")
        .unwrap()
    );
    db.close().await.unwrap();
}
