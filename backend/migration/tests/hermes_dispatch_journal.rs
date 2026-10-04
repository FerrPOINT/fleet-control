use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
use serde_json::Value;

const LEGACY: &str = "SELECT jsonb_build_object(
    'runs',(SELECT jsonb_agg(to_jsonb(r) ORDER BY r.id) FROM session_agent_runs r
        WHERE r.session_id='cccccccc-cccc-4ccc-8ccc-cccccccccccc'),
    'messages',(SELECT jsonb_agg(to_jsonb(m) ORDER BY m.id) FROM session_messages m
        WHERE m.session_id='cccccccc-cccc-4ccc-8ccc-cccccccccccc'),
    'outbox',(SELECT jsonb_agg(to_jsonb(o) ORDER BY o.message_id) FROM message_dispatch_outbox o
        JOIN session_messages m ON m.id=o.message_id
        WHERE m.session_id='cccccccc-cccc-4ccc-8ccc-cccccccccccc')) AS value";

#[tokio::test]
async fn dispatch_journal_additive_upgrade_empty_down_reup_preserves_legacy_and_nonempty_refuses_down()
 {
    let Ok(url) = std::env::var("FLEET_DISPATCH_JOURNAL_MIGRATION_TEST_DATABASE_URL") else {
        return;
    };
    let db = Database::connect(url).await.unwrap();
    let empty=db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT NOT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE') AS empty"))
        .await.unwrap().unwrap().try_get::<bool>("","empty").unwrap();
    assert!(
        empty,
        "dispatch migration regression requires its own empty disposable database"
    );
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261004_000012_hermes_dispatch_journal")
        .expect("dispatch journal migration must remain registered");
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    db.execute_unprepared(
        "INSERT INTO users(id,email,username,display_name,password_hash)
          VALUES('aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','dispatch@example.test','dispatch-migration','Migration','disabled');
         INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,api_port,runtime_path,config_path,workspace_path,logs_path)
          VALUES('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',1,'agent1','hermes','developer','running','Migration',24003,'unused','unused','unused','unused');
         INSERT INTO agent_sessions(id,agent_id,user_id,title,state)
          VALUES('cccccccc-cccc-4ccc-8ccc-cccccccccccc','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','Legacy','active');
         INSERT INTO session_agent_runs(id,session_id,agent_id,run_role,state,model,provider,model_options)
          VALUES('dddddddd-dddd-4ddd-8ddd-dddddddddddd','cccccccc-cccc-4ccc-8ccc-cccccccccccc','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','primary','pending',
            'legacy-model','legacy-provider','{\"temperature\":0.1}');
         INSERT INTO session_messages(id,session_id,author_type,author_user_id,body,message_kind,delivery_state)
          VALUES('eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee','cccccccc-cccc-4ccc-8ccc-cccccccccccc','user','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','Legacy prompt','user_prompt','pending');
         INSERT INTO session_agent_runs(id,session_id,agent_id,run_role,state,runtime_session_id,runtime_run_id)
          VALUES('11111111-1111-4111-8111-111111111111','cccccccc-cccc-4ccc-8ccc-cccccccccccc','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','primary','pending','legacy-accepted-session','run_legacy_accepted'),
                ('22222222-2222-4222-8222-222222222222','cccccccc-cccc-4ccc-8ccc-cccccccccccc','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','primary','completed','legacy-completed-session','run_legacy_completed');
         INSERT INTO session_messages(id,session_id,author_type,author_user_id,body,message_kind,delivery_state,runtime_message_id)
          VALUES('33333333-3333-4333-8333-333333333333','cccccccc-cccc-4ccc-8ccc-cccccccccccc','user','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','Legacy accepted','user_prompt','dispatched','run_legacy_accepted'),
                ('44444444-4444-4444-8444-444444444444','cccccccc-cccc-4ccc-8ccc-cccccccccccc','user','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','Legacy completed','user_prompt','completed','run_legacy_completed');
         UPDATE message_dispatch_outbox SET state='dispatched'
          WHERE message_id IN ('33333333-3333-4333-8333-333333333333','44444444-4444-4444-8444-444444444444');"
    ).await.unwrap();
    let read = || Statement::from_string(DatabaseBackend::Postgres, LEGACY);
    let original: Value = db
        .query_one(read())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap();
    Migrator::up(&db, Some(1)).await.unwrap();
    let upgraded: Value = db
        .query_one(read())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap();
    assert_eq!(original, upgraded);
    assert_eq!(
        db.query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT count(*) AS n FROM hermes_dispatch_journal"
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "n")
        .unwrap(),
        0
    );
    Migrator::down(&db, Some(1)).await.unwrap();
    let downgraded: Value = db
        .query_one(read())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap();
    assert_eq!(original, downgraded);
    Migrator::up(&db, Some(1)).await.unwrap();
    let restored: Value = db
        .query_one(read())
        .await
        .unwrap()
        .unwrap()
        .try_get("", "value")
        .unwrap();
    assert_eq!(original, restored);
    db.execute_unprepared(
        "UPDATE session_agent_runs SET runtime_session_id='fleet:cccccccc-cccc-4ccc-8ccc-cccccccccccc:bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb'
          WHERE id='dddddddd-dddd-4ddd-8ddd-dddddddddddd';
         UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id='eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee';
         WITH ts AS (SELECT clock_timestamp() AS value), body AS (SELECT jsonb_build_object(
            'input','Legacy prompt','session_id','fleet:cccccccc-cccc-4ccc-8ccc-cccccccccccc:bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',
            'model','legacy-model','provider','legacy-provider','model_options',jsonb_build_object('temperature',0.1))::text AS value)
         INSERT INTO hermes_dispatch_journal(message_id,run_id,session_id,agent_id,run_role,requested_session_id,request_body,request_hash,
            idempotency_key,origin,credential_fingerprint,capabilities,created_at,recovery_deadline)
         SELECT 'eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee','dddddddd-dddd-4ddd-8ddd-dddddddddddd','cccccccc-cccc-4ccc-8ccc-cccccccccccc',
            'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','primary',
            'fleet:cccccccc-cccc-4ccc-8ccc-cccccccccccc:bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',body.value,
            encode(sha256(convert_to(body.value,'UTF8')),'hex'),'eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee',
            'http://127.0.0.1:24003',repeat('a',64),
            '{\"object\":\"hermes.api_server.capabilities\",\"platform\":\"hermes-agent\",\"auth\":{\"type\":\"bearer\",\"required\":true},
              \"runtime\":{\"mode\":\"server_agent\",\"tool_execution\":\"server\",\"split_runtime\":false},
              \"features\":{\"run_submission\":true,\"run_status\":true,\"run_events_sse\":true,\"run_stop\":true,
                \"runs_idempotency\":{\"supported\":true,\"durable\":true,\"retention_seconds\":86400}},
              \"endpoints\":{\"runs\":{\"method\":\"POST\",\"path\":\"/v1/runs\"},\"run_status\":{\"method\":\"GET\",\"path\":\"/v1/runs/{run_id}\"},
                \"run_events\":{\"method\":\"GET\",\"path\":\"/v1/runs/{run_id}/events\"},\"run_stop\":{\"method\":\"POST\",\"path\":\"/v1/runs/{run_id}/stop\"}}}'::jsonb,
            ts.value,ts.value+interval '86340 seconds' FROM ts,body;"
    ).await.unwrap();
    assert!(Migrator::down(&db, Some(1)).await.is_err());
    let state: String=db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT state FROM hermes_dispatch_journal WHERE message_id='eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee'"))
        .await.unwrap().unwrap().try_get("","state").unwrap();
    assert_eq!(state, "prepared");
    let history = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM seaql_migrations WHERE version=$1) AS retained",
            ["m20261004_000012_hermes_dispatch_journal".into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(history.try_get::<bool>("", "retained").unwrap());
}
