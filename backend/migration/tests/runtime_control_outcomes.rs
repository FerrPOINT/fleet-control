use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
use serde_json::Value;

#[tokio::test]
async fn control_outcome_upgrade_empty_down_preserves_legacy_guard_and_history() {
    let Ok(url) = std::env::var("FLEET_RUNTIME_OUTCOME_MIGRATION_TEST_DATABASE_URL") else {
        return;
    };
    let db = Database::connect(url).await.unwrap();
    let empty = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT NOT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE') AS empty"))
        .await.unwrap().unwrap().try_get::<bool>("","empty").unwrap();
    assert!(
        empty,
        "outcome migration requires its own empty disposable database"
    );
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261005_000015_runtime_control_outcomes")
        .unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    db.execute_unprepared("INSERT INTO users(id,email,username,display_name,password_hash)
        VALUES('aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','outcome@example.test','outcome-migration','Migration','disabled')").await.unwrap();
    let snapshot = || {
        Statement::from_string(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
            'guard',pg_get_functiondef('guard_runtime_control_command()'::regprocedure)) AS value")
    };
    let original: Value = db
        .query_one(snapshot())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap();
    Migrator::up(&db, Some(1)).await.unwrap();
    let users = db
        .query_one(snapshot())
        .await
        .unwrap()
        .unwrap()
        .try_get::<Value>("", "value")
        .unwrap();
    assert_eq!(users["users"], original["users"]);
    Migrator::down(&db, Some(1)).await.unwrap();
    let reverted = db
        .query_one(snapshot())
        .await
        .unwrap()
        .unwrap()
        .try_get::<Value>("", "value")
        .unwrap();
    assert_eq!(reverted["users"], original["users"]);
    // Function whitespace differs; compare the original legacy transition logic explicitly.
    let guard = reverted["guard"].as_str().unwrap();
    assert!(guard.contains("OLD.state='uncertain' AND NEW.state='terminal_observed'"));
    assert!(!guard.contains("outcome_required"));
    let absent = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT to_regclass('runtime_control_outcomes') IS NULL AS absent",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<bool>("", "absent")
        .unwrap();
    assert!(absent);
    Migrator::up(&db, Some(1)).await.unwrap();
    assert_eq!(
        db.query_one(snapshot())
            .await
            .unwrap()
            .unwrap()
            .try_get::<Value>("", "value")
            .unwrap()["users"],
        original["users"]
    );
    db.execute_unprepared(
        "INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,api_port,runtime_path,config_path,workspace_path,logs_path)
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
         UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id='eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee';
         WITH ts AS (SELECT clock_timestamp() AS value), body AS (SELECT jsonb_build_object('input','Legacy prompt',
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
          'stop','migration-only',repeat('b',64),'run_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','migration-session',request_hash,origin,credential_fingerprint
          FROM hermes_dispatch_journal;"
    ).await.unwrap();
    let command = || {
        Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT to_jsonb(c)-'outcome_required' AS value FROM runtime_control_commands c LIMIT 1",
        )
    };
    let legacy: Value = db
        .query_one(command())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap();
    Migrator::down(&db, Some(1)).await.unwrap();
    Migrator::up(&db, Some(1)).await.unwrap();
    assert_eq!(
        db.query_one(command())
            .await
            .unwrap()
            .unwrap()
            .try_get::<Value>("", "value")
            .unwrap(),
        legacy
    );
    db.execute_unprepared(
        "BEGIN;
         UPDATE runtime_control_commands SET state='submitted',outcome_required=true;
         INSERT INTO runtime_control_outcomes(command_id,context)
         SELECT id,jsonb_build_object('command_id',id,'run_id',runtime_run_id,'operation',operation,'origin',api_origin,
            'credential_fingerprint',credential_fingerprint,'request_body','',
            'request_sha256',encode(sha256(''::bytea),'hex'),
            'capabilities',jsonb_build_object('contract_version',1,'profile','default',
                'native_source_revision','bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3',
                'store_id','22222222-2222-4222-8222-222222222222','scope_fingerprint',repeat('c',64)))
            FROM runtime_control_commands;
         COMMIT;"
    ).await.unwrap();
    assert!(Migrator::down(&db, Some(1)).await.is_err());
    let retained = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM runtime_control_outcomes) AND EXISTS(
            SELECT 1 FROM seaql_migrations WHERE version='m20261005_000015_runtime_control_outcomes') AS retained"))
        .await.unwrap().unwrap().try_get::<bool>("","retained").unwrap();
    assert!(retained);
}
