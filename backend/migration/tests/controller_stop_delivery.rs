use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, Statement, entity::prelude::Uuid,
};
use serde_json::{Value, json};

#[tokio::test]
async fn stop_delivery_upgrade_preserves_history_and_refuses_populated_downgrade() {
    let url = std::env::var("FLEET_TEST_DATABASE_URL")
        .expect("controller stop migration requires isolated PostgreSQL");
    let admin = Database::connect(&url).await.unwrap();
    let schema = format!("controller_stop_{}", Uuid::new_v4().simple());
    admin
        .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
        .await
        .unwrap();
    let mut options = ConnectOptions::new(url);
    options.max_connections(2).set_schema_search_path(&schema);
    let db = Database::connect(options).await.unwrap();
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261007_000022_controller_stop_delivery")
        .unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    let agent = Uuid::new_v4();
    let launch = Uuid::new_v4();
    let controller = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,api_port,runtime_path,config_path,workspace_path,logs_path)
         VALUES($1,1,'agent1','hermes','developer','running','Retained',24003,'unused','unused','unused','unused')", [agent.into()])).await.unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO runtime_launches(id,agent_id,controller_id,binding) VALUES($1,$2,$3,$4)",
        [
            launch.into(),
            agent.into(),
            controller.into(),
            json!({"id":launch,"agent_id":agent,"controller_id":controller}).into(),
        ],
    ))
    .await
    .unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_launches SET state='gateway_started',pid=1234,observed_at=clock_timestamp() WHERE id=$1",[launch.into()])).await.unwrap();
    let snapshot = || {
        Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT jsonb_build_object('agents',(SELECT jsonb_agg(to_jsonb(a)) FROM agents a),
         'launches',(SELECT jsonb_agg(to_jsonb(l)) FROM runtime_launches l)) AS value",
        )
    };
    let before: Value = db
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
        before
    );
    Migrator::down(&db, Some(1)).await.unwrap();
    Migrator::up(&db, Some(1)).await.unwrap();
    let intent = json!({"launch_id":launch,"agent_id":agent,"operation_id":launch,
        "launch_sha256":"a".repeat(64),"snapshot_sha256":"b".repeat(64),"pid":1234});
    for malformed in [
        json!({}),
        json!({"launch_id":launch,"operation_id":launch,"agent_id":agent,"pid":999}),
    ] {
        assert!(db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO runtime_controller_stop_deliveries(launch_id,intent,intent_sha256) VALUES($1,$2,repeat('c',64))",
            [launch.into(),malformed.into()])).await.is_err());
    }
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_controller_stop_deliveries(launch_id,intent,intent_sha256) VALUES($1,$2,repeat('c',64))",
        [launch.into(),intent.clone().into()])).await.unwrap();
    for sql in [
        "DELETE FROM runtime_controller_stop_deliveries",
        "TRUNCATE runtime_controller_stop_deliveries",
        "UPDATE runtime_controller_stop_deliveries SET intent_sha256=repeat('d',64)",
        "UPDATE runtime_controller_stop_deliveries SET intent=jsonb_set(intent,'{pid}','999')",
        "UPDATE runtime_controller_stop_deliveries SET dispatch_command='{}',dispatch_command_sha256=repeat('d',64)",
        "UPDATE runtime_controller_stop_deliveries SET native_outcome='{}',native_outcome_sha256=repeat('d',64)",
    ] {
        assert!(db.execute_unprepared(sql).await.is_err(), "{sql}");
    }
    assert!(Migrator::down(&db, Some(1)).await.is_err());
    assert_eq!(
        db.query_one(snapshot())
            .await
            .unwrap()
            .unwrap()
            .try_get::<Value>("", "value")
            .unwrap(),
        before
    );
    let retained = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT intent,dispatch_command,native_outcome FROM runtime_controller_stop_deliveries WHERE launch_id=$1",[launch.into()])).await.unwrap().unwrap();
    assert_eq!(retained.try_get::<Value>("", "intent").unwrap(), intent);
    assert!(
        retained
            .try_get::<Option<Value>>("", "dispatch_command")
            .unwrap()
            .is_none()
    );
    assert!(
        retained
            .try_get::<Option<Value>>("", "native_outcome")
            .unwrap()
            .is_none()
    );
    db.close().await.unwrap();
    admin
        .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
        .await
        .unwrap();
    admin.close().await.unwrap();
}
