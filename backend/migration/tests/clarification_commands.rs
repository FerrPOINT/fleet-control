use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};

async fn ledger(db: &sea_orm::DatabaseConnection) -> Vec<(String, i64)> {
    Migrator::get_migration_models(db)
        .await
        .unwrap()
        .into_iter()
        .map(|m| (m.version, m.applied_at))
        .collect()
}

#[tokio::test]
#[ignore = "requires empty disposable FLEET_CLARIFICATION_MIGRATION_TEST_DATABASE_URL"]
async fn clarification_upgrade_preserves_history_and_populated_down_is_held() {
    let url = std::env::var("FLEET_CLARIFICATION_MIGRATION_TEST_DATABASE_URL")
        .expect("isolated clarification migration PostgreSQL is required");
    let db = Database::connect(url).await.unwrap();
    let empty: bool = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT NOT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE') AS empty"))
        .await.unwrap().unwrap().try_get("","empty").unwrap();
    assert!(empty, "requires own empty disposable database");
    let versions = Migrator::migrations();
    let target = versions
        .iter()
        .position(|m| m.name() == "m20261010_000020_clarification_commands")
        .unwrap();
    Migrator::up(&db, Some(u32::try_from(target).unwrap()))
        .await
        .unwrap();
    db.execute_unprepared(
        "INSERT INTO users(id,email,username,display_name,password_hash,central_sub)
         VALUES('aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','custody@example.test','custody','Synthetic','disabled','owner');
         INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,runtime_path,config_path,workspace_path,logs_path)
         VALUES('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',1,'agent1','hermes','developer','stopped','Synthetic','unused','unused','unused','unused');
         INSERT INTO agent_sessions(id,agent_id,user_id,title,state)
         VALUES('cccccccc-cccc-4ccc-8ccc-cccccccccccc','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','Synthetic','draft');
         INSERT INTO task_chat_bindings(session_id,tracker_instance_id,project_id,task_id,root_task_id,agent_id,owner_subject,idempotency_key)
         VALUES('cccccccc-cccc-4ccc-8ccc-cccccccccccc','fixture','dddddddd-dddd-4ddd-8ddd-dddddddddddd',
           'eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee','eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','owner','binding');"
    ).await.unwrap();
    let before = ledger(&db).await;
    Migrator::up(&db, Some(1)).await.unwrap();
    Migrator::down(&db, Some(1)).await.unwrap();
    let after = ledger(&db).await;
    assert_eq!(before, after);
    Migrator::up(&db, Some(1)).await.unwrap();
    let body = serde_json::json!({"idempotency_key":"original","expected_question_version":1,"requirement_revision":2,
        "selected_option_ids":[],"text":"Synthetic","comment":null}).to_string();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO clarification_answer_commands(id,session_id,question_id,actor_user_id,owner_subject,binding,idempotency_key,request_body,payload_sha256)
         SELECT '11111111-1111-4111-8111-111111111111',b.session_id,'22222222-2222-4222-8222-222222222222',
         'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',b.owner_subject,to_jsonb(b)-'session_id'-'idempotency_key'-'created_at',
         'original',$1,encode(sha256(convert_to($1,'UTF8')),'hex') FROM task_chat_bindings b",[body.into()])).await.unwrap();
    let saved_ledger = ledger(&db).await;
    assert!(db.execute_unprepared(
        "INSERT INTO clarification_answer_commands(id,session_id,question_id,actor_user_id,owner_subject,
         binding,idempotency_key,request_body,payload_sha256,state,attempt_id,answer)
         SELECT '33333333-3333-4333-8333-333333333333',session_id,question_id,actor_user_id,owner_subject,
         binding,'forged',replace(request_body,'original','forged'),
         encode(sha256(convert_to(replace(request_body,'original','forged'),'UTF8')),'hex'),
         'delivered','44444444-4444-4444-8444-444444444444','{}'::jsonb FROM clarification_answer_commands"
    ).await.is_err());
    assert!(Migrator::down(&db, Some(1)).await.is_err());
    assert_eq!(ledger(&db).await, saved_ledger);
    for sql in [
        "DELETE FROM clarification_answer_commands",
        "UPDATE clarification_answer_commands SET idempotency_key='changed'",
        "UPDATE clarification_answer_commands SET state='delivered',answer='{}'::jsonb",
    ] {
        assert!(db.execute_unprepared(sql).await.is_err());
    }
    db.close().await.unwrap();
}
