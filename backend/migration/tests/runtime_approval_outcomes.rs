use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
use serde_json::Value;

#[tokio::test]
async fn approval_outcome_upgrade_preserves_legacy_and_refuses_nonempty_down() {
    let Ok(url) = std::env::var("FLEET_APPROVAL_OUTCOME_MIGRATION_TEST_DATABASE_URL") else {
        return;
    };
    let db = Database::connect(url).await.unwrap();
    let empty=db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT NOT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE') AS empty"))
        .await.unwrap().unwrap().try_get::<bool>("","empty").unwrap();
    assert!(
        empty,
        "approval outcome migration requires its own disposable database"
    );
    let target = Migrator::migrations()
        .iter()
        .position(|m| m.name() == "m20261005_000016_runtime_approval_outcomes")
        .unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    db.execute_unprepared("INSERT INTO users(id,email,username,display_name,password_hash)
        VALUES('aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','approval-migration@example.test','approval-migration','Migration','disabled');
        INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,api_port,runtime_path,config_path,workspace_path,logs_path)
        VALUES('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',1,'agent1','hermes','developer','running','Migration',24003,'unused','unused','unused','unused');
        INSERT INTO agent_sessions(id,agent_id,user_id,title,state)
        VALUES('cccccccc-cccc-4ccc-8ccc-cccccccccccc','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','Legacy','active');
        INSERT INTO session_agent_runs(id,session_id,agent_id,run_role,state,runtime_session_id)
        VALUES('dddddddd-dddd-4ddd-8ddd-dddddddddddd','cccccccc-cccc-4ccc-8ccc-cccccccccccc',
            'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','primary','waiting','migration-session');
        INSERT INTO runtime_approval_requests(id,session_id,session_run_id,agent_id,runtime_run_id,runtime_approval_id,prompt,detail,state)
        VALUES('eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee','cccccccc-cccc-4ccc-8ccc-cccccccccccc','dddddddd-dddd-4ddd-8ddd-dddddddddddd',
            'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','run_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','migration-request','Migration','{}','pending');
        INSERT INTO runtime_approval_decisions(id,session_id,approval_id,session_run_id,actor_user_id,choice,idempotency_key,state)
        VALUES('11111111-1111-4111-8111-111111111111','cccccccc-cccc-4ccc-8ccc-cccccccccccc','eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee',
            'dddddddd-dddd-4ddd-8ddd-dddddddddddd','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','once','legacy-key','uncertain');")
        .await.unwrap();
    let snapshot = || {
        Statement::from_string(DatabaseBackend::Postgres,
        "SELECT jsonb_build_object('users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
            'decisions',(SELECT jsonb_agg(to_jsonb(d)-'outcome_required'-'submission_claimed' ORDER BY id)
                FROM runtime_approval_decisions d)) AS value")
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
    let mode = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT outcome_required,submission_claimed FROM runtime_approval_decisions",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(!mode.try_get::<bool>("", "outcome_required").unwrap());
    assert!(!mode.try_get::<bool>("", "submission_claimed").unwrap());
    assert!(
        db.execute_unprepared("UPDATE runtime_approval_decisions SET outcome_required=true")
            .await
            .is_err()
    );
    Migrator::down(&db, Some(1)).await.unwrap();
    assert_eq!(
        db.query_one(snapshot())
            .await
            .unwrap()
            .unwrap()
            .try_get::<Value>("", "value")
            .unwrap(),
        original
    );
    let guard = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT pg_get_functiondef('fleet_guard_approval_decision()'::regprocedure) AS value",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "value")
        .unwrap();
    assert!(guard.contains("OLD.state='uncertain' AND NEW.state IN ('delivered','failed')"));
    assert!(!guard.contains("outcome_required"));
    assert!(
        db.execute_unprepared("UPDATE runtime_approval_decisions SET choice='deny'")
            .await
            .is_err()
    );
    Migrator::up(&db, Some(1)).await.unwrap();
    db.execute_unprepared("INSERT INTO runtime_approval_requests(id,session_id,session_run_id,agent_id,runtime_run_id,runtime_approval_id,prompt,detail,state)
        VALUES('22222222-2222-4222-8222-222222222222','cccccccc-cccc-4ccc-8ccc-cccccccccccc','dddddddd-dddd-4ddd-8ddd-dddddddddddd',
            'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','run_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','second-request','Migration','{}','pending');
        INSERT INTO runtime_approval_decisions(id,session_id,approval_id,session_run_id,actor_user_id,choice,idempotency_key,state,outcome_required)
        VALUES('33333333-3333-4333-8333-333333333333','cccccccc-cccc-4ccc-8ccc-cccccccccccc','22222222-2222-4222-8222-222222222222',
            'dddddddd-dddd-4ddd-8ddd-dddddddddddd','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','deny','original-key','uncertain',true);
        UPDATE runtime_approval_decisions SET state='failed' WHERE id='33333333-3333-4333-8333-333333333333';")
        .await.unwrap();
    assert!(Migrator::down(&db, Some(1)).await.is_err());
    let retained=db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM runtime_approval_decisions WHERE outcome_required AND state='failed')
            AND EXISTS(SELECT 1 FROM seaql_migrations WHERE version='m20261005_000016_runtime_approval_outcomes') AS retained"))
        .await.unwrap().unwrap().try_get::<bool>("","retained").unwrap();
    assert!(retained);
}
