use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
use serde_json::Value;

#[tokio::test]
async fn credentials_additive_upgrade_preserves_legacy_operation_and_empty_downgrade() {
    let Ok(url) = std::env::var("FLEET_CREDENTIAL_MIGRATION_TEST_DATABASE_URL") else {
        return;
    };
    let db = Database::connect(&url).await.unwrap();
    let target = Migrator::migrations()
        .iter()
        .position(|migration| migration.name() == "m20261004_000011_pm_credentials")
        .expect("credential migration must remain registered");
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    db.execute_unprepared(
        "INSERT INTO users(id,email,username,display_name,password_hash)
         VALUES('11111111-1111-4111-8111-111111111111','migration@example.test','credentials-migration','Migration','disabled');
         INSERT INTO pm_draft_creation_operations(id,owner_user_id,idempotency_key,operation)
         VALUES('22222222-2222-4222-8222-222222222222','11111111-1111-4111-8111-111111111111','legacy',
            '{\"id\":\"22222222-2222-4222-8222-222222222222\",\"owner_user_id\":\"11111111-1111-4111-8111-111111111111\",
              \"owner_subject\":\"55555555-5555-4555-8555-555555555555\",\"tracker_instance_id\":\"tracker-test\",
              \"project_id\":\"33333333-3333-4333-8333-333333333333\",
              \"request\":{\"agent_id\":\"44444444-4444-4444-8444-444444444444\",\"title\":\"Legacy task\",\"description\":\"Original\",\"idempotency_key\":\"legacy\"},
              \"draft\":null,\"input\":null,\"reservation\":null,\"session_id\":null}'::jsonb);",
    ).await.unwrap();
    let read = || {
        Statement::from_string(DatabaseBackend::Postgres,
        "SELECT operation FROM pm_draft_creation_operations WHERE id='22222222-2222-4222-8222-222222222222'".to_string())
    };
    let original: Value = db
        .query_one(read())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "operation")
        .unwrap();
    Migrator::up(&db, None).await.unwrap();
    let current: Value = db
        .query_one(read())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "operation")
        .unwrap();
    assert_eq!(original, current);
    assert!(current.get("credentials").is_none());
    db.execute_unprepared("UPDATE pm_draft_creation_operations SET updated_at=now() WHERE id='22222222-2222-4222-8222-222222222222'").await.unwrap();
    assert!(db.execute_unprepared("UPDATE pm_draft_creation_operations SET operation=jsonb_set(operation,'{request,title}','\"Changed\"')").await.is_err());
    Migrator::down(&db, Some((Migrator::migrations().len() - target) as u32))
        .await
        .unwrap();
    let restored: Value = db
        .query_one(read())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "operation")
        .unwrap();
    assert_eq!(original, restored);
    Migrator::up(&db, None).await.unwrap();
    let final_value: Value = db
        .query_one(read())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "operation")
        .unwrap();
    assert_eq!(original, final_value);
}
