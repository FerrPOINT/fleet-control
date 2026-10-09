use super::{LegacyMigrator, Migrator, MigratorTrait};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement,
};
use sea_orm_migration::MigrationStatus;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_SCHEMA: AtomicU64 = AtomicU64::new(0);
const COMBINED: &str = super::COMBINED_VERSION;
const TASK_CHATS: &str = "m20261001_000010_task_chats";
const PM_CREDENTIALS: &str = "m20261004_000011_pm_credentials";
const CONTAINER_CONTROLLER: &str = "m20261009_000015_container_controller";
const MAPPED_RECOVERY: &str = "m20261009_000016_mapped_controller_recovery";
const PREPARATION: &str = "m20261009_000017_container_preparation";
const ACTIVATION: &str = "m20261009_000018_container_activation";
const DISPATCH_JOURNAL: &str = "m20261004_000012_hermes_dispatch_journal";

#[test]
fn registered_versions_match_lineage_discriminators() {
    let canonical = Migrator::migrations();
    let legacy = LegacyMigrator::migrations();
    assert_eq!(canonical.len(), 17);
    assert_eq!(legacy.len(), 20);
    assert_eq!(canonical[9].name(), COMBINED);
    assert_eq!(canonical[10].name(), TASK_CHATS);
    assert_eq!(legacy[13].name(), TASK_CHATS);
    assert_eq!(canonical[11].name(), PM_CREDENTIALS);
    assert_eq!(canonical[12].name(), DISPATCH_JOURNAL);
    assert_eq!(legacy[14].name(), PM_CREDENTIALS);
    assert_eq!(legacy[15].name(), DISPATCH_JOURNAL);
    assert_eq!(canonical[13].name(), CONTAINER_CONTROLLER);
    assert_eq!(legacy[16].name(), CONTAINER_CONTROLLER);
    assert_eq!(canonical[14].name(), MAPPED_RECOVERY);
    assert_eq!(legacy[17].name(), MAPPED_RECOVERY);
    assert_eq!(canonical[15].name(), PREPARATION);
    assert_eq!(legacy[18].name(), PREPARATION);
    assert_eq!(canonical.last().unwrap().name(), ACTIVATION);
    assert_eq!(legacy.last().unwrap().name(), ACTIVATION);
    assert_eq!(
        legacy
            .iter()
            .skip(9)
            .take(4)
            .map(|item| item.name())
            .collect::<Vec<_>>(),
        super::SPLIT_VERSIONS
    );
    assert_eq!(
        canonical
            .iter()
            .take(9)
            .map(|item| item.name())
            .collect::<Vec<_>>(),
        legacy
            .iter()
            .take(9)
            .map(|item| item.name())
            .collect::<Vec<_>>()
    );
}

struct Fixture {
    admin: DatabaseConnection,
    db: DatabaseConnection,
    schema: String,
}

impl Fixture {
    async fn new() -> Self {
        let url = std::env::var("FLEET_MIGRATION_TEST_DATABASE_URL")
            .expect("isolated PostgreSQL is required for lineage tests");
        let admin = Database::connect(&url).await.unwrap();
        let schema = format!(
            "fleet_migration_qa_{}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_SCHEMA.fetch_add(1, Ordering::Relaxed)
        );
        admin
            .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
            .await
            .unwrap();
        let mut options = ConnectOptions::new(url);
        options.max_connections(2).set_schema_search_path(&schema);
        let db = Database::connect(options).await.unwrap();
        Self { admin, db, schema }
    }

    async fn close(self) {
        self.db.close().await.unwrap();
        self.admin
            .execute_unprepared(&format!("DROP SCHEMA {} CASCADE", self.schema))
            .await
            .unwrap();
        self.admin.close().await.unwrap();
    }
}

async fn ledger(db: &DatabaseConnection) -> Vec<(String, i64)> {
    Migrator::get_migration_models(db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.version, row.applied_at))
        .collect()
}

async fn seed_history(db: &DatabaseConnection) {
    db.execute_unprepared(
        "INSERT INTO users(id,email,username,display_name,password_hash,system_role,is_system_admin,is_active) \
         VALUES ('00000000-0000-0000-0000-000000000001','qa-history@example.test','qa-history', \
                 'QA historical operator','!','operator',false,false); \
         INSERT INTO deployment_jobs(id,job_kind,state,requested_by_user_id,title,detail,idempotency_key) \
         VALUES ('00000000-0000-0000-0000-000000000002','product_rollback','completed', \
                 '00000000-0000-0000-0000-000000000001','QA historical deploy', \
                 '{\"commit_sha\":\"qa-owned\",\"environment\":\"qa\",\"marker\":\"preserve\"}', \
                 '00000000-0000-0000-0000-000000000003')",
    ).await.unwrap();
}

async fn history(db: &DatabaseConnection) -> Vec<String> {
    db.query_all(Statement::from_string(
        DbBackend::Postgres,
        "SELECT row_to_json(u)::text AS snapshot FROM users u WHERE username='qa-history' \
         UNION ALL SELECT row_to_json(j)::text AS snapshot FROM deployment_jobs j \
         WHERE title='QA historical deploy' ORDER BY snapshot"
            .to_owned(),
    ))
    .await
    .unwrap()
    .into_iter()
    .map(|row| row.try_get("", "snapshot").unwrap())
    .collect()
}

async fn inject_version(db: &DatabaseConnection, version: &str) {
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO seaql_migrations(version,applied_at) VALUES ($1,1)",
        [version.into()],
    ))
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_MIGRATION_TEST_DATABASE_URL"]
async fn fresh_canonical_install_is_repeatable() {
    let fixture = Fixture::new().await;
    Migrator::up(&fixture.db, None).await.unwrap();
    let before = ledger(&fixture.db).await;
    assert_eq!(before.len(), 17);
    assert!(before.iter().any(|(version, _)| version == COMBINED));
    Migrator::up(&fixture.db, None).await.unwrap();
    assert_eq!(ledger(&fixture.db).await, before);
    assert!(
        Migrator::get_pending_migrations(&fixture.db)
            .await
            .unwrap()
            .is_empty()
    );
    Migrator::down(&fixture.db, Some(1)).await.unwrap();
    assert_eq!(ledger(&fixture.db).await.len(), 16);
    Migrator::up(&fixture.db, None).await.unwrap();
    assert_eq!(ledger(&fixture.db).await.len(), 17);
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires isolated FLEET_MIGRATION_TEST_DATABASE_URL"]
async fn common_prefix_completes_with_canonical_foundation() {
    let fixture = Fixture::new().await;
    Migrator::up(&fixture.db, Some(9)).await.unwrap();
    seed_history(&fixture.db).await;
    let before = ledger(&fixture.db).await;
    let data = history(&fixture.db).await;
    Migrator::up(&fixture.db, None).await.unwrap();
    let after = ledger(&fixture.db).await;
    assert_eq!(after.len(), 17);
    assert!(after.iter().any(|(version, _)| version == COMBINED));
    assert!(before.iter().all(|entry| after.contains(entry)));
    assert_eq!(history(&fixture.db).await, data);
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires isolated FLEET_MIGRATION_TEST_DATABASE_URL"]
async fn split_down_one_and_reapply_preserves_other_history() {
    let fixture = Fixture::new().await;
    LegacyMigrator::up(&fixture.db, None).await.unwrap();
    seed_history(&fixture.db).await;
    let before = ledger(&fixture.db).await;
    let data = history(&fixture.db).await;
    Migrator::down(&fixture.db, Some(1)).await.unwrap();
    let remaining = ledger(&fixture.db).await;
    assert_eq!(remaining.len(), 19);
    assert!(remaining.iter().all(|entry| before.contains(entry)));
    Migrator::up(&fixture.db, None).await.unwrap();
    let after = ledger(&fixture.db).await;
    assert_eq!(after.len(), 20);
    assert!(remaining.iter().all(|entry| after.contains(entry)));
    assert!(!after.iter().any(|(version, _)| version == COMBINED));
    assert_eq!(history(&fixture.db).await, data);
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires isolated FLEET_MIGRATION_TEST_DATABASE_URL"]
async fn complete_split_history_preserves_data_and_ledger() {
    let fixture = Fixture::new().await;
    LegacyMigrator::up(&fixture.db, None).await.unwrap();
    seed_history(&fixture.db).await;
    let before = ledger(&fixture.db).await;
    let data = history(&fixture.db).await;
    assert_eq!(before.len(), 20);
    assert_eq!(data.len(), 2);
    for _ in 0..2 {
        Migrator::up(&fixture.db, None).await.unwrap();
        let status = Migrator::get_migration_with_status(&fixture.db)
            .await
            .unwrap();
        assert_eq!(status.len(), 20);
        assert!(
            status
                .iter()
                .all(|migration| migration.status() == MigrationStatus::Applied)
        );
        assert!(
            Migrator::get_pending_migrations(&fixture.db)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(ledger(&fixture.db).await, before);
        assert_eq!(history(&fixture.db).await, data);
    }
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires isolated FLEET_MIGRATION_TEST_DATABASE_URL"]
async fn partial_split_history_completes_only_missing_versions() {
    let fixture = Fixture::new().await;
    LegacyMigrator::up(&fixture.db, Some(10)).await.unwrap();
    seed_history(&fixture.db).await;
    let before = ledger(&fixture.db).await;
    let data = history(&fixture.db).await;
    assert_eq!(before.len(), 10);
    Migrator::up(&fixture.db, None).await.unwrap();
    let after = ledger(&fixture.db).await;
    assert_eq!(after.len(), 20);
    assert!(!after.iter().any(|(version, _)| version == COMBINED));
    assert!(before.iter().all(|entry| after.contains(entry)));
    assert_eq!(history(&fixture.db).await, data);
    assert!(
        Migrator::get_pending_migrations(&fixture.db)
            .await
            .unwrap()
            .is_empty()
    );
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires isolated FLEET_MIGRATION_TEST_DATABASE_URL"]
async fn unknown_history_fails_without_rewriting_data_or_ledger() {
    let fixture = Fixture::new().await;
    Migrator::up(&fixture.db, None).await.unwrap();
    seed_history(&fixture.db).await;
    inject_version(&fixture.db, "m20990101_000001_unknown").await;
    let before = ledger(&fixture.db).await;
    let data = history(&fixture.db).await;
    let error = Migrator::up(&fixture.db, None)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("m20990101_000001_unknown"));
    assert_eq!(ledger(&fixture.db).await, before);
    assert_eq!(history(&fixture.db).await, data);
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires isolated FLEET_MIGRATION_TEST_DATABASE_URL"]
async fn mixed_history_fails_without_rewriting_data_or_ledger() {
    let fixture = Fixture::new().await;
    LegacyMigrator::up(&fixture.db, None).await.unwrap();
    seed_history(&fixture.db).await;
    inject_version(&fixture.db, COMBINED).await;
    let before = ledger(&fixture.db).await;
    let data = history(&fixture.db).await;
    let error = Migrator::up(&fixture.db, None)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("mixes split SDLC and combined foundation"));
    assert_eq!(ledger(&fixture.db).await, before);
    assert_eq!(history(&fixture.db).await, data);
    fixture.close().await;
}

async fn seed_runtime_history(db: &DatabaseConnection) {
    seed_history(db).await;
    db.execute_unprepared(
        "INSERT INTO agents(id,ordinal,name,kind,role,product_role,sdlc_role,status,display_name,
            runtime_path,config_path,workspace_path,logs_path)
         VALUES ('00000000-0000-0000-0000-000000000004',1,'agent1','hermes','developer',
            'executor','developer','stopped','Historical developer',
            '/qa/agent1/runtime','/qa/agent1/config','/qa/agent1/workspace','/qa/agent1/logs');
         INSERT INTO agent_runtime(agent_id,command_preview)
         VALUES ('00000000-0000-0000-0000-000000000004','historical-command');
         INSERT INTO agent_sessions(id,agent_id,user_id,title,state,task_key)
         VALUES ('00000000-0000-0000-0000-000000000005','00000000-0000-0000-0000-000000000004',
            '00000000-0000-0000-0000-000000000001','Historical private chat','active','HISTORY-1');
         INSERT INTO session_messages(id,session_id,author_type,author_user_id,body,message_kind,delivery_state)
         VALUES ('00000000-0000-0000-0000-000000000006','00000000-0000-0000-0000-000000000005',
            'user','00000000-0000-0000-0000-000000000001','Historical prompt','user_prompt','pending');
         INSERT INTO agent_config_revisions(agent_id,revision,state,snapshot,created_by_user_id)
         VALUES ('00000000-0000-0000-0000-000000000004',1,'active',jsonb_build_object('historical',true),
            '00000000-0000-0000-0000-000000000001');
         INSERT INTO agent_config_heads(agent_id,desired_revision,effective_revision)
         VALUES ('00000000-0000-0000-0000-000000000004',1,1);"
    ).await.unwrap();
}

async fn runtime_history(db: &DatabaseConnection) -> Vec<String> {
    db.query_all(Statement::from_string(
        DbBackend::Postgres,
        "SELECT to_jsonb(a)::text AS snapshot FROM agents a
         UNION ALL SELECT to_jsonb(r)::text FROM agent_runtime r
         UNION ALL SELECT to_jsonb(s)::text FROM agent_sessions s
         UNION ALL SELECT (to_jsonb(m)-'append_sequence')::text FROM session_messages m
         UNION ALL SELECT to_jsonb(o)::text FROM message_dispatch_outbox o
         UNION ALL SELECT to_jsonb(c)::text FROM agent_config_revisions c
         UNION ALL SELECT to_jsonb(h)::text FROM agent_config_heads h
         UNION ALL SELECT to_jsonb(e)::text FROM session_events e
         UNION ALL SELECT to_jsonb(c)::text FROM session_event_cursors c ORDER BY snapshot"
            .to_owned(),
    ))
    .await
    .unwrap()
    .into_iter()
    .map(|row| row.try_get("", "snapshot").unwrap())
    .collect()
}

#[tokio::test]
#[ignore = "requires isolated FLEET_MIGRATION_TEST_DATABASE_URL"]
async fn both_accepted_foundations_upgrade_task_chats_without_legacy_rebinding() {
    for split in [false, true] {
        let fixture = Fixture::new().await;
        let expected = if split {
            LegacyMigrator::up(&fixture.db, Some(13)).await.unwrap();
            20
        } else {
            Migrator::up(&fixture.db, Some(10)).await.unwrap();
            17
        };
        seed_runtime_history(&fixture.db).await;
        let before = ledger(&fixture.db).await;
        let users = history(&fixture.db).await;
        let data = runtime_history(&fixture.db).await;
        assert_eq!(data.len(), 10);
        Migrator::up(&fixture.db, None).await.unwrap();
        let upgraded = ledger(&fixture.db).await;
        assert_eq!(upgraded.len(), expected);
        assert!(before.iter().all(|entry| upgraded.contains(entry)));
        assert!(upgraded.iter().any(|(version, _)| version == TASK_CHATS));
        assert_eq!(runtime_history(&fixture.db).await, data);
        assert_eq!(history(&fixture.db).await, users);
        let row = fixture
            .db
            .query_one(Statement::from_string(
                DbBackend::Postgres,
                "SELECT (SELECT count(*) FROM task_chat_bindings) AS bindings,
                 (SELECT count(*) FROM pm_run_bindings) AS runs,
                 (SELECT count(*) FROM tracker_event_inbox) AS events,
                 (SELECT append_sequence FROM session_messages LIMIT 1) AS sequence"
                    .to_owned(),
            ))
            .await
            .unwrap()
            .unwrap();
        for column in ["bindings", "runs", "events"] {
            assert_eq!(row.try_get::<i64>("", column).unwrap(), 0);
        }
        assert_eq!(row.try_get::<i64>("", "sequence").unwrap(), 1);
        Migrator::down(&fixture.db, Some(6)).await.unwrap();
        let task_chat_ledger = ledger(&fixture.db).await;
        assert_eq!(task_chat_ledger.len(), expected - 6);
        assert_eq!(runtime_history(&fixture.db).await, data);
        let error = Migrator::down(&fixture.db, Some(1))
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("task-chat history prevents downgrade"));
        assert_eq!(ledger(&fixture.db).await, task_chat_ledger);
        Migrator::up(&fixture.db, None).await.unwrap();
        assert_eq!(runtime_history(&fixture.db).await, data);
        assert_eq!(history(&fixture.db).await, users);
        fixture.close().await;
    }
}

#[tokio::test]
#[ignore = "requires isolated FLEET_MIGRATION_TEST_DATABASE_URL"]
async fn task_chat_downgrade_preserves_unmessaged_binding_and_draft_operation() {
    for binding in [false, true] {
        let fixture = Fixture::new().await;
        Migrator::up(&fixture.db, Some(11)).await.unwrap();
        seed_history(&fixture.db).await;
        if binding {
            fixture.db.execute_unprepared(
                "INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,runtime_path,config_path,workspace_path,logs_path)
                 VALUES ('00000000-0000-0000-0000-000000000004',1,'agent1','hermes','developer','stopped','QA binding','unused','unused','unused','unused');
                 INSERT INTO agent_sessions(id,agent_id,user_id,title,state)
                 VALUES ('00000000-0000-0000-0000-000000000005','00000000-0000-0000-0000-000000000004',
                         '00000000-0000-0000-0000-000000000001','QA empty task chat','draft');
                 INSERT INTO task_chat_bindings(session_id,tracker_instance_id,project_id,task_id,root_task_id,agent_id,owner_subject,idempotency_key)
                 VALUES ('00000000-0000-0000-0000-000000000005','qa-owned',
                         '00000000-0000-0000-0000-000000000007','00000000-0000-0000-0000-000000000008',
                         '00000000-0000-0000-0000-000000000008','00000000-0000-0000-0000-000000000004',
                         'qa-owner','qa-binding');"
            ).await.unwrap();
        } else {
            fixture.db.execute_unprepared(
                "INSERT INTO pm_draft_creation_operations(id,owner_user_id,idempotency_key,operation)
                 VALUES ('00000000-0000-0000-0000-000000000006','00000000-0000-0000-0000-000000000001','qa-draft',
                         '{\"id\":\"00000000-0000-0000-0000-000000000006\",\"owner_user_id\":\"00000000-0000-0000-0000-000000000001\",
                            \"request\":{\"idempotency_key\":\"qa-draft\"},\"draft\":null,\"input\":null,\"reservation\":null,\"session_id\":null}');"
            ).await.unwrap();
        }
        let before = ledger(&fixture.db).await;
        let users = history(&fixture.db).await;
        let data = task_history(&fixture.db).await;
        assert_eq!(data.len(), 1);
        let row = fixture
            .db
            .query_one(Statement::from_string(
                DbBackend::Postgres,
                "SELECT count(*) AS n FROM session_messages".to_owned(),
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<i64>("", "n").unwrap(), 0);
        let error = Migrator::down(&fixture.db, Some(1))
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("task-chat history prevents downgrade"));
        assert_eq!(ledger(&fixture.db).await, before);
        assert_eq!(task_history(&fixture.db).await, data);
        assert_eq!(history(&fixture.db).await, users);
        fixture.close().await;
    }
}

async fn task_history(db: &DatabaseConnection) -> Vec<String> {
    db.query_all(Statement::from_string(
        DbBackend::Postgres,
        "SELECT to_jsonb(b)::text AS snapshot FROM task_chat_bindings b
         UNION ALL SELECT to_jsonb(o)::text FROM pm_draft_creation_operations o ORDER BY snapshot"
            .to_owned(),
    ))
    .await
    .unwrap()
    .into_iter()
    .map(|row| row.try_get::<String>("", "snapshot").unwrap())
    .collect()
}
