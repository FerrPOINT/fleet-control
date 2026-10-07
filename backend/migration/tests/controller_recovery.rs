use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, Statement, entity::prelude::Uuid,
};
use serde_json::{Value, json};

#[tokio::test]
async fn recovery_upgrade_preserves_original_rows_and_retained_history_blocks_downgrade() {
    let url = std::env::var("FLEET_TEST_DATABASE_URL")
        .expect("controller recovery migration requires isolated PostgreSQL");
    let admin = Database::connect(&url).await.unwrap();
    let schema = format!("controller_recovery_{}", Uuid::new_v4().simple());
    admin
        .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
        .await
        .unwrap();
    let mut options = ConnectOptions::new(url);
    options.max_connections(2).set_schema_search_path(&schema);
    let db = Database::connect(options).await.unwrap();
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261007_000020_controller_recovery")
        .unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    let agent = Uuid::new_v4();
    let launch = Uuid::new_v4();
    let old_controller = Uuid::new_v4();
    let new_controller = Uuid::new_v4();
    let command = Uuid::new_v4();
    let binding = json!({"id":launch,"agent_id":agent,"controller_id":old_controller});
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,api_port,runtime_path,config_path,workspace_path,logs_path)
         VALUES($1,1,'agent1','hermes','developer','running','Existing',24003,'unused','unused','unused','unused')", [agent.into()]))
        .await.unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO runtime_launches(id,agent_id,controller_id,binding) VALUES($1,$2,$3,$4)",
        [
            launch.into(),
            agent.into(),
            old_controller.into(),
            binding.into(),
        ],
    ))
    .await
    .unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_launches SET state='gateway_started',pid=1234,observed_at=clock_timestamp() WHERE id=$1", [launch.into()]))
        .await.unwrap();
    let snapshot = || {
        Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT jsonb_build_object('agents',(SELECT jsonb_agg(to_jsonb(a)) FROM agents a),
            'launches',(SELECT jsonb_agg(to_jsonb(l)) FROM runtime_launches l)) AS value",
        )
    };
    let before = db
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
        before
    );
    assert_eq!(
        db.query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT count(*) AS count FROM runtime_controller_recoveries"
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
    let request = json!({"id":command,"launch_id":launch,"agent_id":agent,"controller_id":new_controller,"predecessor_id":null});
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_controller_recoveries(id,launch_id,agent_id,controller_id,epoch,request,request_sha256,lease_expires_at)
        VALUES($1,$2,$3,$4,1,$5,repeat('a',64),clock_timestamp()+interval '30 seconds')",
        [command.into(),launch.into(),agent.into(),new_controller.into(),request.into()])).await.unwrap();
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
    assert!(
        db.query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM runtime_controller_recoveries WHERE id=$1",
            [command.into()]
        ))
        .await
        .unwrap()
        .is_some()
    );
    db.close().await.unwrap();
    admin
        .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
        .await
        .unwrap();
    admin.close().await.unwrap();
}
