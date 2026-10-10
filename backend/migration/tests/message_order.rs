use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseBackend, Statement};

#[tokio::test]
#[ignore = "requires an empty disposable FLEET_MESSAGE_ORDER_TEST_DATABASE_URL database"]
async fn historical_backfill_and_clock_rollback_keep_order_without_changing_wire() {
    let url = std::env::var("FLEET_MESSAGE_ORDER_TEST_DATABASE_URL").unwrap();
    // Reuse the prepared-statement cache across drop/recreate boundaries.
    let mut options = ConnectOptions::new(url);
    options.max_connections(1).min_connections(1);
    let db = Database::connect(options).await.unwrap();
    Migrator::up(&db, Some(10)).await.unwrap();
    db.execute_unprepared(
        "INSERT INTO users(id,email,username,display_name,password_hash) VALUES
          ('aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','order@example.test','order','Order','!');
         INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,runtime_path,config_path,workspace_path,logs_path)
          VALUES ('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',1,'agent1','hermes','developer','stopped','Order','unused','unused','unused','unused');
         INSERT INTO agent_sessions(id,agent_id,user_id,title,state)
          VALUES ('cccccccc-cccc-4ccc-8ccc-cccccccccccc','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','Order','draft');
         INSERT INTO session_messages(id,session_id,author_type,body,message_kind,created_at) VALUES
          ('dddddddd-dddd-4ddd-8ddd-ddddddddddd1','cccccccc-cccc-4ccc-8ccc-cccccccccccc','system','Historical later','system_event','2030-01-01'),
          ('dddddddd-dddd-4ddd-8ddd-ddddddddddd2','cccccccc-cccc-4ccc-8ccc-cccccccccccc','system','Historical earlier','system_event','2020-01-01');",
    )
    .await
    .unwrap();
    let before = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT count(*) AS n FROM session_events".to_string(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "n")
        .unwrap();
    Migrator::up(&db, None).await.unwrap();
    let after = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT count(*) AS n FROM session_events".to_string(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "n")
        .unwrap();
    assert_eq!(
        before, after,
        "backfill must not emit message-change events"
    );
    db.execute_unprepared(
        "INSERT INTO session_messages(id,session_id,author_type,body,message_kind,created_at) VALUES
          ('dddddddd-dddd-4ddd-8ddd-ddddddddddd3','cccccccc-cccc-4ccc-8ccc-cccccccccccc','system','Appended after rollback','system_event','2010-01-01');",
    )
    .await
    .unwrap();
    let rows = db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT body,append_sequence FROM session_messages ORDER BY append_sequence"
                .to_string(),
        ))
        .await
        .unwrap();
    let bodies: Vec<String> = rows
        .iter()
        .map(|r| r.try_get("", "body").unwrap())
        .collect();
    assert_eq!(
        bodies,
        [
            "Historical earlier",
            "Historical later",
            "Appended after rollback"
        ]
    );
    let sequences: Vec<i64> = rows
        .iter()
        .map(|r| r.try_get("", "append_sequence").unwrap())
        .collect();
    assert!(sequences.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(
        db.execute_unprepared("UPDATE session_messages SET append_sequence=append_sequence+100")
            .await
            .is_err()
    );
    assert!(db.execute_unprepared(
        "INSERT INTO session_messages(id,session_id,author_type,body,message_kind,append_sequence) VALUES
          (gen_random_uuid(),'cccccccc-cccc-4ccc-8ccc-cccccccccccc','system','Forbidden','system_event',100);"
    ).await.is_err());
    let object_identity = || {
        Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_backend_pid()::bigint AS connection_pid,
                to_regprocedure($1)::oid::bigint AS hermes_oid,
                to_regprocedure($2)::oid::bigint AS recovery_oid,
                to_regclass($3)::oid::bigint AS launches_oid",
            [
                "fleet_guard_hermes_dispatch()".into(),
                "fleet_guard_container_recovery()".into(),
                "runtime_container_launches".into(),
            ],
        )
    };
    let original_identity = db.query_one(object_identity()).await.unwrap().unwrap();
    let migrations = Migrator::migrations();
    let task_chat_index = migrations
        .iter()
        .position(|migration| migration.name() == "m20261001_000010_task_chats")
        .unwrap();
    let successors = u32::try_from(migrations.len() - task_chat_index - 1).unwrap();
    Migrator::down(&db, Some(successors)).await.unwrap();
    let ledger = Migrator::get_migration_models(&db).await.unwrap();
    assert_eq!(ledger.len(), task_chat_index + 1);
    assert!(ledger.iter().all(|row| {
        migrations[..=task_chat_index]
            .iter()
            .any(|migration| migration.name() == row.version.as_str())
    }));
    let error = Migrator::down(&db, Some(1)).await.unwrap_err().to_string();
    assert!(error.contains("task-chat history prevents downgrade"));
    assert_eq!(
        Migrator::get_migration_models(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|row| (row.version, row.applied_at))
            .collect::<Vec<_>>(),
        ledger
            .into_iter()
            .map(|row| (row.version, row.applied_at))
            .collect::<Vec<_>>()
    );
    Migrator::up(&db, None).await.unwrap();
    let restored_identity = db.query_one(object_identity()).await.unwrap().unwrap();
    assert_eq!(
        original_identity
            .try_get::<i64>("", "connection_pid")
            .unwrap(),
        restored_identity
            .try_get::<i64>("", "connection_pid")
            .unwrap()
    );
    for object in ["hermes_oid", "recovery_oid", "launches_oid"] {
        assert_ne!(
            original_identity.try_get::<i64>("", object).unwrap(),
            restored_identity.try_get::<i64>("", object).unwrap(),
            "roundtrip must exercise recreated objects on the same connection"
        );
    }
    let after = db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT body,append_sequence FROM session_messages ORDER BY append_sequence"
                .to_string(),
        ))
        .await
        .unwrap();
    assert_eq!(
        after
            .iter()
            .map(|row| row.try_get::<String>("", "body").unwrap())
            .collect::<Vec<_>>(),
        bodies
    );
    assert_eq!(
        after
            .iter()
            .map(|row| row.try_get::<i64>("", "append_sequence").unwrap())
            .collect::<Vec<_>>(),
        sequences
    );
}
