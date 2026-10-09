use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
use serde_json::Value;

#[tokio::test]
#[ignore = "requires empty disposable FLEET_RUNTIME_CONTROL_MIGRATION_TEST_DATABASE_URL"]
async fn control_ledger_additive_upgrade_empty_down_and_nonempty_guard() {
    let url = std::env::var("FLEET_RUNTIME_CONTROL_MIGRATION_TEST_DATABASE_URL")
        .expect("isolated control migration PostgreSQL is required");
    let db = Database::connect(url).await.unwrap();
    let empty = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT NOT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE') AS empty"))
        .await.unwrap().unwrap().try_get::<bool>("","empty").unwrap();
    assert!(
        empty,
        "control migration requires its own empty disposable database"
    );
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261005_000013_runtime_controls")
        .unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    db.execute_unprepared(
        "INSERT INTO users(id,email,username,display_name,password_hash)
          VALUES('aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','control@example.test','control-migration','Migration','disabled');
         INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,api_port,runtime_path,config_path,workspace_path,logs_path)
          VALUES('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',1,'agent1','hermes','developer','running','Migration',24003,'unused','unused','unused','unused');
         INSERT INTO agent_sessions(id,agent_id,user_id,title,state)
          VALUES('cccccccc-cccc-4ccc-8ccc-cccccccccccc','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','Legacy','active');
         INSERT INTO session_agent_runs(id,session_id,agent_id,run_role,state,runtime_session_id)
          VALUES('dddddddd-dddd-4ddd-8ddd-dddddddddddd','cccccccc-cccc-4ccc-8ccc-cccccccccccc',
            'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','primary','pending',
            'fleet:cccccccc-cccc-4ccc-8ccc-cccccccccccc:bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb');
         INSERT INTO session_messages(id,session_id,author_type,author_user_id,body,message_kind,delivery_state)
          VALUES('eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee','cccccccc-cccc-4ccc-8ccc-cccccccccccc',
            'user','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','Legacy prompt','user_prompt','pending');
         UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id='eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee';"
    ).await.unwrap();
    let snapshot = || {
        Statement::from_string(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('session',(SELECT to_jsonb(s) FROM agent_sessions s LIMIT 1),
          'run',(SELECT to_jsonb(r) FROM session_agent_runs r LIMIT 1),
          'message',(SELECT to_jsonb(m) FROM session_messages m WHERE message_kind='user_prompt' LIMIT 1)) AS value")
    };
    let original: Value = db
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
        original
    );
    Migrator::down(&db, Some(1)).await.unwrap();
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
    db.execute_unprepared(
        "WITH ts AS (SELECT clock_timestamp() AS value), body AS (SELECT jsonb_build_object('input','Legacy prompt',
          'session_id','fleet:cccccccc-cccc-4ccc-8ccc-cccccccccccc:bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb')::text AS value)
         INSERT INTO hermes_dispatch_journal(message_id,run_id,session_id,agent_id,run_role,requested_session_id,
          request_body,request_hash,idempotency_key,origin,credential_fingerprint,capabilities,created_at,recovery_deadline)
         SELECT 'eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee','dddddddd-dddd-4ddd-8ddd-dddddddddddd','cccccccc-cccc-4ccc-8ccc-cccccccccccc',
          'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','primary',
          'fleet:cccccccc-cccc-4ccc-8ccc-cccccccccccc:bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',body.value,
          encode(sha256(convert_to(body.value,'UTF8')),'hex'),'eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee','http://127.0.0.1:24003',repeat('a',64),
          '{\"object\":\"hermes.api_server.capabilities\",\"platform\":\"hermes-agent\",\"auth\":{\"type\":\"bearer\",\"required\":true},
            \"runtime\":{\"mode\":\"server_agent\",\"tool_execution\":\"server\",\"split_runtime\":false},
            \"features\":{\"run_submission\":true,\"run_status\":true,\"run_events_sse\":true,\"run_stop\":true,
              \"runs_idempotency\":{\"supported\":true,\"durable\":true,\"retention_seconds\":86400}},
            \"endpoints\":{\"runs\":{\"method\":\"POST\",\"path\":\"/v1/runs\"},\"run_status\":{\"method\":\"GET\",\"path\":\"/v1/runs/{run_id}\"},
              \"run_events\":{\"method\":\"GET\",\"path\":\"/v1/runs/{run_id}/events\"},\"run_stop\":{\"method\":\"POST\",\"path\":\"/v1/runs/{run_id}/stop\"}}}'::jsonb,
          ts.value,ts.value+interval '86340 seconds' FROM ts,body;
         INSERT INTO runtime_control_commands(id,session_id,session_run_id,agent_id,actor_user_id,operation,idempotency_key,
          payload_sha256,runtime_run_id,runtime_session_id,original_request_sha256,api_origin,credential_fingerprint)
         SELECT '11111111-1111-4111-8111-111111111111',session_id,run_id,agent_id,'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
          'stop','migration-only',repeat('b',64),'run_migration','migration-session',request_hash,origin,credential_fingerprint
          FROM hermes_dispatch_journal;"
    ).await.unwrap();
    assert!(Migrator::down(&db, Some(1)).await.is_err());
    let retained = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM seaql_migrations WHERE version='m20261005_000013_runtime_controls') AS retained"))
        .await.unwrap().unwrap().try_get::<bool>("","retained").unwrap();
    assert!(retained);
    assert!(
        db.execute_unprepared("DELETE FROM runtime_control_commands")
            .await
            .is_err()
    );
    assert!(
        db.execute_unprepared("UPDATE runtime_control_commands SET idempotency_key='changed'")
            .await
            .is_err()
    );
    assert!(
        db.execute_unprepared(
            "UPDATE runtime_control_commands SET state='acknowledged',acknowledgement='stopping'"
        )
        .await
        .is_err()
    );
}
