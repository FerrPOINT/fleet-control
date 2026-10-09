use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

#[tokio::test]
#[ignore = "requires empty FLEET_CONTAINER_PREPARATION_MIGRATION_TEST_DATABASE_URL"]
async fn preparation_is_one_additive_migration_and_retains_unknown_custody() {
    let url = std::env::var("FLEET_CONTAINER_PREPARATION_MIGRATION_TEST_DATABASE_URL")
        .expect("own empty preparation migration database is mandatory");
    assert!(url.ends_with("/fleet_container_preparation_migration_test"));
    let db = Database::connect(url).await.unwrap();
    assert!(
        Migrator::get_migration_models(&db)
            .await
            .unwrap()
            .is_empty()
    );
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261009_000017_container_preparation")
        .unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    let before = Migrator::get_migration_models(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|m| (m.version, m.applied_at))
        .collect::<Vec<_>>();
    db.execute_unprepared("INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,runtime_path,config_path,workspace_path,logs_path)
        VALUES('00000000-0000-0000-0000-000000000001',1,'agent1','hermes','developer','ready','historic','/a/runtime','/a/config','/a/workspace','/a/logs')").await.unwrap();
    let read = || {
        Statement::from_string(DbBackend::Postgres,"SELECT to_jsonb(a)::text AS snapshot FROM agents a
        UNION ALL SELECT pg_get_functiondef('fleet_guard_hermes_dispatch()'::regprocedure) ORDER BY snapshot")
    };
    let original = db
        .query_all(read())
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.try_get::<String>("", "snapshot").unwrap())
        .collect::<Vec<_>>();
    Migrator::up(&db, Some(1)).await.unwrap();
    Migrator::down(&db, Some(1)).await.unwrap();
    assert_eq!(
        Migrator::get_migration_models(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|m| (m.version, m.applied_at))
            .collect::<Vec<_>>(),
        before
    );
    assert_eq!(
        db.query_all(read())
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.try_get::<String>("", "snapshot").unwrap())
            .collect::<Vec<_>>(),
        original
    );
    Migrator::up(&db, Some(1)).await.unwrap();
    db.execute_unprepared(r#"INSERT INTO runtime_container_preparations(agent_id,generation,operation_id,claim)
        VALUES('00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000002','00000000-0000-0000-0000-000000000003',
        '{"agent_id":"00000000-0000-0000-0000-000000000001","generation":"00000000-0000-0000-0000-000000000002","operation_id":"00000000-0000-0000-0000-000000000003","intent_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","paths":{"runtime":"/a/runtime","config":"/a/config","workspace":"/a/workspace","logs":"/a/logs"},"api_port":null,"configuration_revision":null,"configuration_sha256":null}')"#).await.unwrap();
    let error = Migrator::down(&db, Some(1)).await.unwrap_err().to_string();
    assert!(error.contains("Preparation custody history prevents downgrade"));
    assert_eq!(Migrator::get_migration_models(&db).await.unwrap().len(), 18);
    db.close().await.unwrap();
}
