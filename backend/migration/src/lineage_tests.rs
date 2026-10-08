use super::{LegacyMigrator, Migrator, MigratorTrait};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement,
};
use sea_orm_migration::MigrationStatus;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_SCHEMA: AtomicU64 = AtomicU64::new(0);
const COMBINED: &str = super::COMBINED_VERSION;

#[test]
fn registered_versions_match_lineage_discriminators() {
    let canonical = Migrator::migrations();
    let legacy = LegacyMigrator::migrations();
    assert_eq!(canonical.len(), 12);
    assert_eq!(legacy.len(), 15);
    assert_eq!(canonical[9].name(), COMBINED);
    assert_eq!(
        canonical.last().unwrap().name(),
        "m20261008_000091_context_drafts"
    );
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
    assert_eq!(before.len(), 12);
    assert!(before.iter().any(|(version, _)| version == COMBINED));
    Migrator::up(&fixture.db, None).await.unwrap();
    assert_eq!(ledger(&fixture.db).await, before);
    assert!(
        Migrator::get_pending_migrations(&fixture.db)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        Migrator::down(&fixture.db, Some(1))
            .await
            .unwrap_err()
            .to_string()
            .contains("compatible cohort")
    );
    assert_eq!(ledger(&fixture.db).await, before);
    Migrator::up(&fixture.db, None).await.unwrap();
    assert_eq!(ledger(&fixture.db).await.len(), 12);
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
    assert_eq!(after.len(), 12);
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
    assert!(
        Migrator::down(&fixture.db, Some(1))
            .await
            .unwrap_err()
            .to_string()
            .contains("compatible cohort")
    );
    let remaining = ledger(&fixture.db).await;
    assert_eq!(remaining, before);
    assert!(remaining.iter().all(|entry| before.contains(entry)));
    Migrator::up(&fixture.db, None).await.unwrap();
    let after = ledger(&fixture.db).await;
    assert_eq!(after.len(), 15);
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
    assert_eq!(before.len(), 15);
    assert_eq!(data.len(), 2);
    for _ in 0..2 {
        Migrator::up(&fixture.db, None).await.unwrap();
        let status = Migrator::get_migration_with_status(&fixture.db)
            .await
            .unwrap();
        assert_eq!(status.len(), 15);
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
    assert_eq!(after.len(), 15);
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
