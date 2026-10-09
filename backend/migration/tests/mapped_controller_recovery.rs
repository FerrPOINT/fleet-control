use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};

async fn original(db: &DatabaseConnection) -> Vec<String> {
    db.query_all(Statement::from_string(
        DbBackend::Postgres,
        "SELECT to_jsonb(c)::text AS snapshot FROM runtime_container_launches c
         UNION ALL SELECT to_jsonb(a)::text FROM agents a
         UNION ALL SELECT pg_get_functiondef('fleet_guard_hermes_dispatch()'::regprocedure)
         UNION ALL SELECT pg_get_constraintdef(oid) FROM pg_constraint
            WHERE conrelid='runtime_container_launches'::regclass AND contype='c'
         UNION ALL SELECT to_jsonb(m)::text FROM seaql_migrations m
            WHERE version<>'m20261009_000016_mapped_controller_recovery' ORDER BY snapshot",
    ))
    .await
    .unwrap()
    .into_iter()
    .map(|r| r.try_get("", "snapshot").unwrap())
    .collect()
}

#[tokio::test]
#[ignore = "requires empty FLEET_MAPPED_CONTROLLER_MIGRATION_TEST_DATABASE_URL"]
async fn mapped_recovery_is_one_additive_migration_and_cannot_erase_custody() {
    let url = std::env::var("FLEET_MAPPED_CONTROLLER_MIGRATION_TEST_DATABASE_URL")
        .expect("own empty mapped recovery database is mandatory");
    assert!(url.ends_with("/fleet_mapped_controller_migration_test"));
    let db = Database::connect(url).await.unwrap();
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261009_000016_mapped_controller_recovery")
        .unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    db.execute_unprepared(r#"
        INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,runtime_path,config_path,workspace_path,logs_path)
        VALUES('00000000-0000-0000-0000-000000000001',1,'agent1','hermes','developer','stopped','historical','/agents/agent1/runtime','/agents/agent1/config','/agents/agent1/workspace','/agents/agent1/logs');
        INSERT INTO runtime_container_launches(generation,agent_id,controller_id,prepared,state,stop_id)
        VALUES('00000000-0000-0000-0000-000000000002','00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000003',
            '{"agent_id":"00000000-0000-0000-0000-000000000001","container":{"registration":{"contract_version":2,"generation":"00000000-0000-0000-0000-000000000002","resource_id":"00000000-0000-0000-0000-000000000001"}}}',
            'claimed','00000000-0000-0000-0000-000000000004');
    "#).await.unwrap();
    let before = original(&db).await;
    Migrator::up(&db, Some(1)).await.unwrap();
    assert_eq!(
        Migrator::get_migration_models(&db)
            .await
            .unwrap()
            .last()
            .unwrap()
            .version,
        "m20261009_000016_mapped_controller_recovery"
    );
    let count = db
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS n FROM runtime_container_recoveries",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(count.try_get::<i64>("", "n").unwrap(), 0);
    Migrator::down(&db, Some(1)).await.unwrap();
    assert_eq!(original(&db).await, before);
    Migrator::up(&db, Some(1)).await.unwrap();
    db.execute_unprepared(r#"
        INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,runtime_path,config_path,workspace_path,logs_path)
        VALUES('00000000-0000-0000-0000-000000000005',2,'agent2','hermes','developer','stopped','mapped','/agents/agent2/runtime','/agents/agent2/config','/agents/agent2/workspace','/agents/agent2/logs');
        INSERT INTO runtime_container_launches(generation,agent_id,controller_id,prepared,state,stop_id)
        VALUES('00000000-0000-0000-0000-000000000006','00000000-0000-0000-0000-000000000005','00000000-0000-0000-0000-000000000003',
            '{"agent_id":"00000000-0000-0000-0000-000000000005","container":{"registration":{"contract_version":3,"generation":"00000000-0000-0000-0000-000000000006","resource_id":"00000000-0000-0000-0000-000000000005"}}}',
            'claimed','00000000-0000-0000-0000-000000000007');
    "#).await.unwrap();
    let error = Migrator::down(&db, Some(1)).await.unwrap_err().to_string();
    assert!(error.contains("Mapped custody history prevents downgrade"));
    assert_eq!(Migrator::get_migration_models(&db).await.unwrap().len(), 17);
    db.close().await.unwrap();
}
