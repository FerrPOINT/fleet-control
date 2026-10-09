use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
use serde_json::{Value, json};

#[tokio::test]
#[ignore = "requires empty FLEET_CONTAINER_CONTROLLER_MIGRATION_TEST_DATABASE_URL"]
async fn additive_container_upgrade_restores_original_guard_and_preserves_history() {
    let url = std::env::var("FLEET_CONTAINER_CONTROLLER_MIGRATION_TEST_DATABASE_URL")
        .expect("own empty PostgreSQL database required");
    let db = Database::connect(url).await.unwrap();
    let empty:bool=db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT NOT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE') AS empty"))
        .await.unwrap().unwrap().try_get("","empty").unwrap();
    assert!(empty);
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261009_000015_container_controller")
        .unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    db.execute_unprepared("INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,api_port,runtime_path,config_path,workspace_path,logs_path)
        VALUES('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',1,'agent1','hermes','developer','running','Legacy',24003,'original','original','original','original');
        INSERT INTO agent_runtime(agent_id,desired_state,pid,command_preview) VALUES('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','running',123,'original');").await.unwrap();
    let read = || {
        Statement::from_string(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('agents',(SELECT jsonb_agg(to_jsonb(a)) FROM agents a),'runtime',(SELECT jsonb_agg(to_jsonb(r)) FROM agent_runtime r),
        'guard',pg_get_functiondef('fleet_guard_hermes_dispatch()'::regprocedure),
        'ledger',(SELECT jsonb_agg(to_jsonb(m) ORDER BY version) FROM seaql_migrations m)) AS value")
    };
    let before: Value = db
        .query_one(read())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap();
    Migrator::up(&db, Some(1)).await.unwrap();
    let after: Value = db
        .query_one(read())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap();
    assert_eq!(before["agents"], after["agents"]);
    assert_eq!(before["runtime"], after["runtime"]);
    assert!(
        after["guard"]
            .as_str()
            .unwrap()
            .contains("fleet_container_origin(a.id, NEW.origin, a.api_port, NEW.capabilities)")
    );
    Migrator::down(&db, Some(1)).await.unwrap();
    assert_eq!(
        before,
        db.query_one(read())
            .await
            .unwrap()
            .unwrap()
            .try_get::<Value>("", "value")
            .unwrap()
    );
    Migrator::up(&db, Some(1)).await.unwrap();
    let prepared = json!({"agent_id":"bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb","container":{"registration":{
        "generation":"cccccccc-cccc-4ccc-8ccc-cccccccccccc","resource_id":"bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"}}});
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_container_launches(generation,agent_id,controller_id,prepared,state,stop_id) VALUES(
        'cccccccc-cccc-4ccc-8ccc-cccccccccccc','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','dddddddd-dddd-4ddd-8ddd-dddddddddddd',$1,'claimed','eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee')",
        [prepared.into()])).await.unwrap();
    let error = Migrator::down(&db, Some(1)).await.unwrap_err().to_string();
    assert!(error.contains("Original container history prevents downgrade"));
    let current: Value = db
        .query_one(read())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap();
    assert_eq!(current["agents"], before["agents"]);
    assert_eq!(current["runtime"], before["runtime"]);
}
