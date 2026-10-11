use migration::{Migrator, MigratorTrait, SchemaManager};
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, TransactionTrait,
};
use std::collections::BTreeMap;

const REPAIR: &str = "m20261010_000024_pm_ack_bounds";
const RUN: &str = "00000000-0000-4000-8000-000000000004";

async fn ledger(db: &DatabaseConnection) -> BTreeMap<String, i64> {
    Migrator::get_migration_models(db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.version, row.applied_at))
        .collect()
}

async fn constraints(db: &DatabaseConnection) -> BTreeMap<String, String> {
    db.query_all(Statement::from_string(
        DbBackend::Postgres,
        "SELECT conname::text AS name,pg_get_constraintdef(oid) AS definition FROM pg_constraint
         WHERE conrelid='pm_dispatch_journal'::regclass AND contype='c' ORDER BY conname",
    ))
    .await
    .unwrap()
    .into_iter()
    .map(|row| {
        (
            row.try_get("", "name").unwrap(),
            row.try_get("", "definition").unwrap(),
        )
    })
    .collect()
}

async fn journal(db: &DatabaseConnection) -> String {
    db.query_one(Statement::from_string(
        DbBackend::Postgres,
        "SELECT row_to_json(j)::text AS snapshot FROM pm_dispatch_journal j",
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "snapshot")
    .unwrap()
}

async fn guard(db: &DatabaseConnection) -> (i64, String) {
    let row = db
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT 'guard_pm_dispatch_journal()'::regprocedure::oid::bigint AS id,
         pg_get_functiondef('guard_pm_dispatch_journal()'::regprocedure) AS body",
        ))
        .await
        .unwrap()
        .unwrap();
    (
        row.try_get("", "id").unwrap(),
        row.try_get("", "body").unwrap(),
    )
}

#[tokio::test]
#[ignore = "requires empty disposable FLEET_PM_ACK_MIGRATION_TEST_DATABASE_URL"]
async fn pm_ack_bounds_repairs_installed_022_preserving_custody_and_empty_roundtrip() {
    let url = std::env::var("FLEET_PM_ACK_MIGRATION_TEST_DATABASE_URL")
        .expect("own empty PM ACK migration database required");
    assert!(url.ends_with("/fleet_pm_ack_migration_test"));
    let db = Database::connect(url).await.unwrap();
    assert!(ledger(&db).await.is_empty());
    let migrations = Migrator::migrations();
    let target = migrations.iter().position(|m| m.name() == REPAIR).unwrap();
    Migrator::up(&db, Some(target as u32)).await.unwrap();
    let original_ledger = ledger(&db).await;
    let original_constraints = constraints(&db).await;
    let original_guard = guard(&db).await;
    let names = original_constraints
        .iter()
        .filter(|(_, definition)| definition.contains("^[A-Za-z0-9_-]{1,512}$"))
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 1);
    let name = &names[0];
    assert!(name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'));
    for tamper in [
        format!("ALTER TABLE pm_dispatch_journal DROP CONSTRAINT {name};
                 ALTER TABLE pm_dispatch_journal ADD CONSTRAINT {name} CHECK(hermes_run_ref IS NULL)"),
        "ALTER TABLE pm_dispatch_journal ADD CONSTRAINT duplicate_pm_ack
         CHECK(hermes_run_ref IS NULL OR (submitted AND hermes_run_ref ~ '^[A-Za-z0-9_-]{1,512}$'))".into(),
    ] {
        let txn = db.begin().await.unwrap();
        txn.execute_unprepared(&tamper).await.unwrap();
        let error = migrations[target]
            .up(&SchemaManager::new(&txn))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("PM ACK constraint shape changed"));
        txn.rollback().await.unwrap();
        assert_eq!(constraints(&db).await, original_constraints);
        assert_eq!(ledger(&db).await, original_ledger);
    }
    Migrator::up(&db, Some(1)).await.unwrap();
    let mut repaired_constraints = constraints(&db).await;
    let repaired = repaired_constraints.remove(name).unwrap();
    assert!(repaired.contains("^[A-Za-z0-9_-]+$"));
    let mut other_constraints = original_constraints.clone();
    other_constraints.remove(name);
    assert_eq!(repaired_constraints, other_constraints);
    assert_eq!(guard(&db).await, original_guard);
    Migrator::down(&db, Some(1)).await.unwrap();
    assert_eq!(ledger(&db).await, original_ledger);
    assert_eq!(constraints(&db).await, original_constraints);
    assert_eq!(guard(&db).await, original_guard);

    // Start the populated upgrade from an actual installed 022, before 023 and the repair.
    Migrator::down(&db, Some(1)).await.unwrap();
    let old_ledger = ledger(&db).await;
    assert_eq!(
        old_ledger.last_key_value().unwrap().0,
        "m20261010_000022_pm_dispatch"
    );
    db.execute_unprepared(r#"
        INSERT INTO users(id,email,username,display_name,password_hash)
            VALUES('00000000-0000-4000-8000-000000000001','ack@example.test','ack-migration','ACK test','disabled');
        INSERT INTO agents(id,ordinal,name,kind,role,status,display_name,runtime_path,config_path,workspace_path,logs_path)
            VALUES('00000000-0000-4000-8000-000000000002',1,'ack-migration','hermes','developer','stopped','ACK test','runtime','config','workspace','logs');
        INSERT INTO agent_sessions(id,agent_id,user_id,title,state)
            VALUES('00000000-0000-4000-8000-000000000003','00000000-0000-4000-8000-000000000002',
                   '00000000-0000-4000-8000-000000000001','ACK test','draft');
        INSERT INTO task_chat_bindings(session_id,tracker_instance_id,project_id,task_id,root_task_id,agent_id,owner_subject,idempotency_key)
            VALUES('00000000-0000-4000-8000-000000000003','tracker-test','00000000-0000-4000-8000-000000000005',
                   '00000000-0000-4000-8000-000000000006','00000000-0000-4000-8000-000000000006',
                   '00000000-0000-4000-8000-000000000002','ack-owner','ack-test');
        INSERT INTO session_agent_runs(id,session_id,agent_id,run_role,state)
            VALUES('00000000-0000-4000-8000-000000000004','00000000-0000-4000-8000-000000000003',
                   '00000000-0000-4000-8000-000000000002','primary','pending');
        INSERT INTO pm_run_bindings(session_run_id,session_id,agent_id,reservation,dispatch_operation_key,runtime_session_id)
            VALUES('00000000-0000-4000-8000-000000000004','00000000-0000-4000-8000-000000000003',
                   '00000000-0000-4000-8000-000000000002','{}','ack-test','native-session');
        INSERT INTO pm_dispatch_journal(session_run_id,intent)
            VALUES('00000000-0000-4000-8000-000000000004',jsonb_build_object(
                'session_run_id','00000000-0000-4000-8000-000000000004','submitted',false,'hermes_run_ref',NULL,
                'request_body','{"input":"owned test","session_id":"native-session"}',
                'origin','http://native.test','workflow_origin','http://workflow.test',
                'credential_fingerprint',repeat('a',64),'workflow_credential_fingerprint',repeat('b',64)));
        UPDATE pm_dispatch_journal SET submitted=true;
    "#).await.unwrap();
    let pending = journal(&db).await;
    let update = |value: String| {
        Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE pm_dispatch_journal SET hermes_run_ref=$2 WHERE session_run_id=$1::text::uuid AND submitted",
            [RUN.into(), value.into()],
        )
    };
    let error = db.execute(update("run_old".into())).await.unwrap_err();
    assert!(error.to_string().contains("invalid regular expression"));
    assert_eq!(journal(&db).await, pending);
    // Keep downgrade assertions on the repair when later migrations are registered.
    let repair_steps = u32::try_from(target + 1 - old_ledger.len()).unwrap();
    Migrator::up(&db, Some(repair_steps)).await.unwrap();
    let upgraded_ledger = ledger(&db).await;
    assert_eq!(upgraded_ledger.len(), target + 1);
    assert_eq!(upgraded_ledger.last_key_value().unwrap().0, REPAIR);
    for (version, applied_at) in old_ledger {
        assert_eq!(upgraded_ledger.get(&version), Some(&applied_at));
    }
    assert_eq!(journal(&db).await, pending);
    assert_eq!(guard(&db).await, original_guard);
    let error = Migrator::down(&db, Some(1)).await.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("PM dispatch custody prevents ACK bounds downgrade")
    );
    assert_eq!(ledger(&db).await, upgraded_ledger);
    assert_eq!(journal(&db).await, pending);
    for (value, valid) in [
        (String::new(), false),
        ("r".repeat(513), false),
        ("run/slash".to_string(), false),
        ("r".repeat(512), true),
    ] {
        let txn = db.begin().await.unwrap();
        let result = txn.execute(update(value)).await;
        assert_eq!(result.is_ok(), valid, "PM ACK bounds upgrade predicate");
        if let Ok(result) = result {
            assert_eq!(result.rows_affected(), 1);
        }
        txn.rollback().await.unwrap();
        assert_eq!(journal(&db).await, pending);
    }
    assert_eq!(
        db.execute(update("run_old".into()))
            .await
            .unwrap()
            .rows_affected(),
        1
    );
    let acknowledged = journal(&db).await;
    let snapshot: serde_json::Value = serde_json::from_str(&acknowledged).unwrap();
    assert_eq!(snapshot["hermes_run_ref"], "run_old");
    assert_eq!(snapshot["submitted"], true);
    let error = Migrator::down(&db, Some(1)).await.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("PM dispatch custody prevents ACK bounds downgrade")
    );
    assert_eq!(ledger(&db).await, upgraded_ledger);
    assert_eq!(journal(&db).await, acknowledged);
    assert_eq!(guard(&db).await, original_guard);
    db.close().await.unwrap();
}
