use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, Statement, entity::prelude::Uuid,
};
use serde_json::{Value, json};

#[tokio::test]
async fn delivery_upgrade_empty_restore_and_populated_history_guard() {
    let url = std::env::var("FLEET_TEST_DATABASE_URL")
        .expect("controller delivery migration needs isolated PostgreSQL");
    let admin = Database::connect(&url).await.unwrap();
    let schema = format!("controller_delivery_{}", Uuid::new_v4().simple());
    admin
        .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
        .await
        .unwrap();
    let mut options = ConnectOptions::new(url);
    options.max_connections(2).set_schema_search_path(&schema);
    let db = Database::connect(options).await.unwrap();
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261007_000021_controller_recovery_delivery")
        .unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    let agent = Uuid::new_v4();
    let launch = Uuid::new_v4();
    let old = Uuid::new_v4();
    let owner = Uuid::new_v4();
    let id = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,api_port,runtime_path,config_path,workspace_path,logs_path)
         VALUES($1,1,'agent1','hermes','developer','running','Retained',24003,'unused','unused','unused','unused')", [agent.into()])).await.unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO runtime_launches(id,agent_id,controller_id,binding) VALUES($1,$2,$3,$4)",
        [
            launch.into(),
            agent.into(),
            old.into(),
            json!({"id":launch,"agent_id":agent,"controller_id":old}).into(),
        ],
    ))
    .await
    .unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_launches SET state='gateway_started',pid=1234,observed_at=clock_timestamp() WHERE id=$1", [launch.into()])).await.unwrap();
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
    let request = json!({"id":id,"launch_id":launch,"agent_id":agent,"controller_id":owner,"predecessor_id":null});
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_controller_recoveries(id,launch_id,agent_id,controller_id,epoch,request,request_sha256,lease_expires_at)
         VALUES($1,$2,$3,$4,1,$5,repeat('a',64),clock_timestamp()+interval '30 seconds') RETURNING lease_expires_at",
        [id.into(),launch.into(),agent.into(),owner.into(),request.clone().into()])).await.unwrap().unwrap();
    let deadline: sea_orm::prelude::DateTimeWithTimeZone =
        row.try_get("", "lease_expires_at").unwrap();
    let command = json!({"request":request,"epoch":1,"lease_version":1,"lease_expires_at":deadline.to_rfc3339()});
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_controller_recovery_deliveries(recovery_id,command,command_sha256) VALUES($1,$2,repeat('b',64))",
        [id.into(),command.clone().into()])).await.unwrap();
    for sql in [
        "DELETE FROM runtime_controller_recovery_deliveries",
        "TRUNCATE runtime_controller_recovery_deliveries",
        "UPDATE runtime_controller_recovery_deliveries SET command=jsonb_set(command,'{epoch}','null')",
        "UPDATE runtime_controller_recovery_deliveries SET command_sha256=repeat('c',64)",
        "UPDATE runtime_controller_recovery_deliveries SET native_receipt='{}',native_receipt_sha256=repeat('c',64)",
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
        "SELECT command,dispatch_claimed,native_receipt FROM runtime_controller_recovery_deliveries WHERE recovery_id=$1", [id.into()]))
        .await.unwrap().unwrap();
    assert_eq!(retained.try_get::<Value>("", "command").unwrap(), command);
    assert!(!retained.try_get::<bool>("", "dispatch_claimed").unwrap());
    assert!(
        retained
            .try_get::<Option<Value>>("", "native_receipt")
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
