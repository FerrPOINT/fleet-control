use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

#[tokio::test]
#[ignore = "requires empty FLEET_CONTAINER_ACTIVATION_MIGRATION_TEST_DATABASE_URL"]
async fn activation_is_one_additive_migration_and_restores_exact_v17_guards() {
    let url = std::env::var("FLEET_CONTAINER_ACTIVATION_MIGRATION_TEST_DATABASE_URL")
        .expect("own empty activation migration database required");
    assert!(url.ends_with("/fleet_container_activation_migration_test"));
    let db = Database::connect(url).await.unwrap();
    assert!(
        Migrator::get_migration_models(&db)
            .await
            .unwrap()
            .is_empty()
    );
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261009_000018_container_activation")
        .unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    let ledger = Migrator::get_migration_models(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|m| (m.version, m.applied_at))
        .collect::<Vec<_>>();
    let read = || {
        Statement::from_string(DbBackend::Postgres,
        "SELECT pg_get_functiondef('fleet_guard_preparation_launch()'::regprocedure) AS definition
         UNION ALL SELECT pg_get_functiondef('fleet_guard_preparation_config()'::regprocedure)
         UNION ALL SELECT pg_get_functiondef('fleet_guard_hermes_dispatch()'::regprocedure) ORDER BY definition")
    };
    let before = db
        .query_all(read())
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.try_get::<String>("", "definition").unwrap())
        .collect::<Vec<_>>();
    Migrator::up(&db, Some(1)).await.unwrap();
    assert_eq!(Migrator::get_migration_models(&db).await.unwrap().len(), 19);
    Migrator::down(&db, Some(1)).await.unwrap();
    assert_eq!(
        Migrator::get_migration_models(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|m| (m.version, m.applied_at))
            .collect::<Vec<_>>(),
        ledger
    );
    assert_eq!(
        db.query_all(read())
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.try_get::<String>("", "definition").unwrap())
            .collect::<Vec<_>>(),
        before
    );
    Migrator::up(&db, Some(1)).await.unwrap();
    // Even an unknown, pre-native planned command is protected history.
    db.execute_unprepared(r#"
        INSERT INTO users(id,email,username,display_name,password_hash,system_role,is_active)
        VALUES('00000000-0000-0000-0000-000000000001','activation@test.invalid','activation','activation','!','operator',false);
        INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,runtime_path,config_path,workspace_path,logs_path)
        VALUES('00000000-0000-0000-0000-000000000002',1,'agent1','hermes','developer','ready','activation','/a/runtime','/a/config','/a/workspace','/a/logs');
        INSERT INTO agent_config_revisions(agent_id,revision,state,snapshot,created_by_user_id)
        VALUES('00000000-0000-0000-0000-000000000002',1,'activating','{}','00000000-0000-0000-0000-000000000001');
        INSERT INTO runtime_container_activations(id,agent_id,revision,record)
        VALUES('00000000-0000-0000-0000-000000000003','00000000-0000-0000-0000-000000000002',1,
            '{"claim":{"id":"00000000-0000-0000-0000-000000000003","agent_id":"00000000-0000-0000-0000-000000000002","revision":1,
            "intent_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},"phase":"planned","previous_stop":null,"candidate":null,"candidate_stop":null,"rollback":null,"readiness":null}');
    "#).await.unwrap();
    assert!(
        db.execute_unprepared("DELETE FROM runtime_container_activations")
            .await
            .is_err()
    );
    let error = Migrator::down(&db, Some(1)).await.unwrap_err().to_string();
    assert!(error.contains("Activation custody history prevents downgrade"));
    assert_eq!(Migrator::get_migration_models(&db).await.unwrap().len(), 19);
    db.close().await.unwrap();
}
