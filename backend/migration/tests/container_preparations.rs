use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
use serde_json::Value;

#[tokio::test]
async fn preparation_upgrade_preserves_launches_and_retained_fences_block_downgrade() {
    let Ok(url) = std::env::var("FLEET_CONTAINER_PREPARATION_MIGRATION_TEST_DATABASE_URL") else {
        return;
    };
    let db = Database::connect(url).await.unwrap();
    let empty = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT NOT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE') AS empty"))
        .await.unwrap().unwrap().try_get::<bool>("", "empty").unwrap();
    assert!(empty, "preparation upgrade requires a disposable database");
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261006_000018_container_preparations")
        .unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    db.execute_unprepared("INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,api_port,runtime_path,config_path,workspace_path,logs_path)
        VALUES('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',1,'agent1','hermes','developer','stopped','Existing',24003,'unused','unused','unused','unused');
        INSERT INTO agent_runtime(agent_id,desired_state,command_preview) VALUES('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','stopped','Existing');
        INSERT INTO runtime_launches(id,agent_id,controller_id,binding)
        VALUES('cccccccc-cccc-4ccc-8ccc-cccccccccccc','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','dddddddd-dddd-4ddd-8ddd-dddddddddddd',
            '{\"id\":\"cccccccc-cccc-4ccc-8ccc-cccccccccccc\",\"agent_id\":\"bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb\",\"controller_id\":\"dddddddd-dddd-4ddd-8ddd-dddddddddddd\"}');
        UPDATE runtime_launches SET state='spawn_failed',observed_at=now();").await.unwrap();
    let snapshot = || {
        Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT jsonb_build_object('agents',(SELECT jsonb_agg(to_jsonb(a)) FROM agents a),
            'runtime',(SELECT jsonb_agg(to_jsonb(r)) FROM agent_runtime r),
            'launches',(SELECT jsonb_agg(to_jsonb(l)) FROM runtime_launches l)) AS value",
        )
    };
    let original = db
        .query_one(snapshot())
        .await
        .unwrap()
        .unwrap()
        .try_get::<Value>("", "value")
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
    Migrator::down(&db, Some(1)).await.unwrap();
    Migrator::up(&db, Some(1)).await.unwrap();
    let insert = "INSERT INTO runtime_container_preparations(agent_id,ordinal,controller_id,generation,operation_id,intent_sha256)
        VALUES('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',1,'dddddddd-dddd-4ddd-8ddd-dddddddddddd',
            'eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee','ffffffff-ffff-4fff-8fff-ffffffffffff',repeat('a',64))";
    for invalid in [
        insert.replace(",1,", ",-1,"),
        insert.replace("repeat('a',64)", "'short'"),
        insert.replace(
            "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee",
            "00000000-0000-0000-0000-000000000000",
        ),
    ] {
        assert!(db.execute_unprepared(&invalid).await.is_err());
    }
    db.execute_unprepared(insert).await.unwrap();
    assert!(db.execute_unprepared(insert).await.is_err());
    for sql in [
        "DELETE FROM runtime_container_preparations",
        "TRUNCATE runtime_container_preparations",
        "UPDATE runtime_container_preparations SET intent_sha256=repeat('b',64)",
        "UPDATE runtime_container_preparations SET ordinal=ordinal+1",
    ] {
        assert!(db.execute_unprepared(sql).await.is_err());
    }
    assert!(Migrator::down(&db, Some(1)).await.is_err());
    let retained = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM runtime_container_preparations WHERE ordinal=1)
            AND EXISTS(SELECT 1 FROM seaql_migrations WHERE version='m20261006_000018_container_preparations') AS retained"))
        .await.unwrap().unwrap().try_get::<bool>("", "retained").unwrap();
    assert!(retained);
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
