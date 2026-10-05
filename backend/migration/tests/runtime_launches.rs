use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
use serde_json::Value;

#[tokio::test]
async fn launch_upgrade_preserves_legacy_and_nonempty_history_blocks_downgrade() {
    let Ok(url) = std::env::var("FLEET_RUNTIME_LAUNCH_MIGRATION_TEST_DATABASE_URL") else {
        return;
    };
    let db = Database::connect(url).await.unwrap();
    let empty = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT NOT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE') AS empty"))
        .await.unwrap().unwrap().try_get::<bool>("", "empty").unwrap();
    assert!(
        empty,
        "runtime launch upgrade requires a disposable database"
    );
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261006_000017_runtime_launches")
        .unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    db.execute_unprepared("INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,api_port,runtime_path,config_path,workspace_path,logs_path)
        VALUES('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',1,'agent1','hermes','developer','running','Existing',24003,'unused','unused','unused','unused');
        INSERT INTO agent_runtime(agent_id,desired_state,pid,command_preview) VALUES('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','running',123,'Existing');")
        .await.unwrap();
    let snapshot = || {
        Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT jsonb_build_object('agents',(SELECT jsonb_agg(to_jsonb(a)) FROM agents a),
            'runtime',(SELECT jsonb_agg(to_jsonb(r)) FROM agent_runtime r)) AS value",
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
    assert_eq!(
        db.query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT count(*) AS count FROM runtime_launches"
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "count")
        .unwrap(),
        0
    );
    Migrator::down(&db, Some(1)).await.unwrap();
    Migrator::up(&db, Some(1)).await.unwrap();
    let insert = "INSERT INTO runtime_launches(id,agent_id,controller_id,binding)
        VALUES('cccccccc-cccc-4ccc-8ccc-cccccccccccc','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','dddddddd-dddd-4ddd-8ddd-dddddddddddd',
            '{\"id\":\"cccccccc-cccc-4ccc-8ccc-cccccccccccc\",\"agent_id\":\"bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb\",\"controller_id\":\"dddddddd-dddd-4ddd-8ddd-dddddddddddd\"}')";
    assert!(
        db.execute_unprepared(&insert.replace(
            "dddddddd-dddd-4ddd-8ddd-dddddddddddd\"",
            "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee\""
        ))
        .await
        .is_err()
    );
    assert!(
        db.execute_unprepared(&insert.replace(
            "{\"id\":\"cccccccc-cccc-4ccc-8ccc-cccccccccccc\"",
            "{\"id\":null"
        ))
        .await
        .is_err()
    );
    db.execute_unprepared(insert).await.unwrap();
    assert!(Migrator::down(&db, Some(1)).await.is_err());
    for sql in [
        "DELETE FROM runtime_launches",
        "TRUNCATE runtime_launches",
        "UPDATE runtime_launches SET id='aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa'",
        "UPDATE runtime_launches SET state='gateway_started',pid=0,observed_at=now()",
        "UPDATE runtime_launches SET state='gateway_started',pid=NULL,observed_at=now()",
    ] {
        assert!(db.execute_unprepared(sql).await.is_err());
    }
    db.execute_unprepared("UPDATE runtime_launches SET state='spawn_failed',observed_at=now()")
        .await
        .unwrap();
    assert!(Migrator::down(&db, Some(1)).await.is_err());
    let retained = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM runtime_launches WHERE state='spawn_failed')
            AND EXISTS(SELECT 1 FROM seaql_migrations WHERE version='m20261006_000017_runtime_launches') AS retained"))
        .await.unwrap().unwrap().try_get::<bool>("","retained").unwrap();
    assert!(retained);
}
