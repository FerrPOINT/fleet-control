use sea_orm::{DatabaseBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const OLD_BOUND_GUARD: &str =
    "OR EXISTS(SELECT 1 FROM task_chat_bindings WHERE session_id=NEW.session_id)
   OR EXISTS(SELECT 1 FROM pm_run_bindings WHERE session_run_id=NEW.run_id)";
const PM_BOUND_GUARD: &str = "OR ((NEW.capabilities ? 'fleet_pm'
     OR EXISTS(SELECT 1 FROM task_chat_bindings WHERE session_id=NEW.session_id)
     OR EXISTS(SELECT 1 FROM pm_run_bindings WHERE session_run_id=NEW.run_id))
     AND NOT fleet_pm_dispatch_prepared(NEW.session_id,NEW.agent_id,NEW.run_id,NEW.message_id,
       NEW.requested_session_id,NEW.capabilities))";

async fn replace_bound_guard(
    manager: &SchemaManager<'_>,
    before: &str,
    after: &str,
) -> Result<(), DbErr> {
    let db = manager.get_connection();
    let row = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "SELECT pg_get_functiondef('fleet_guard_hermes_dispatch()'::regprocedure) AS definition"))
        .await?.ok_or_else(|| DbErr::Custom("Hermes dispatch guard missing".into()))?;
    let definition: String = row.try_get("", "definition")?;
    if definition.matches(before).count() != 1 {
        return Err(DbErr::Custom(
            "Unsupported Hermes dispatch guard; reconcile source before upgrade".into(),
        ));
    }
    db.execute_unprepared(&definition.replacen(before, after, 1))
        .await?;
    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(PM_PREPARED)
            .await?;
        replace_bound_guard(manager, OLD_BOUND_GUARD, PM_BOUND_GUARD).await?;
        manager.get_connection().execute_unprepared("ALTER TABLE hermes_dispatch_journal
          DROP CONSTRAINT hermes_dispatch_journal_check6,
          ADD CONSTRAINT hermes_dispatch_journal_check6 CHECK (
            (NOT (capabilities ? 'fleet_pm') AND requested_session_id='fleet:'||session_id::text||':'||agent_id::text)
            OR (capabilities ? 'fleet_pm' AND requested_session_id=
              'fleet-pm:'||session_id::text||':'||agent_id::text||':'||message_id::text));").await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("DO $$ BEGIN
          IF EXISTS(SELECT 1 FROM hermes_dispatch_journal WHERE capabilities ? 'fleet_pm') THEN
            RAISE EXCEPTION 'PM native journal history requires reconciliation before downgrade' USING ERRCODE='23514';
          END IF;
        END $$;").await?;
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE hermes_dispatch_journal
          DROP CONSTRAINT hermes_dispatch_journal_check6,
          ADD CONSTRAINT hermes_dispatch_journal_check6 CHECK (
            requested_session_id='fleet:'||session_id::text||':'||agent_id::text);",
            )
            .await?;
        replace_bound_guard(manager, PM_BOUND_GUARD, OLD_BOUND_GUARD).await?;
        manager
            .get_connection()
            .execute_unprepared(
                "DROP FUNCTION fleet_pm_dispatch_prepared(uuid,uuid,uuid,uuid,text,jsonb);",
            )
            .await?;
        Ok(())
    }
}

const PM_PREPARED: &str = r#"
CREATE FUNCTION fleet_pm_dispatch_prepared(session uuid,agent uuid,run uuid,message uuid,
  native_session text,facts jsonb) RETURNS boolean LANGUAGE sql STABLE AS $$
 SELECT EXISTS(
   SELECT 1 FROM pm_draft_creation_operations p
   JOIN task_chat_bindings b ON b.session_id=session
   JOIN agent_sessions s ON s.id=b.session_id
   JOIN agents a ON a.id=b.agent_id
   JOIN session_messages m ON m.id=message AND m.session_id=s.id
   WHERE p.id::text=facts #>> '{fleet_pm,operation_id}'
     AND p.operation->>'session_id'=s.id::text AND p.owner_user_id=s.user_id
     AND p.operation->>'project_id'=b.project_id::text
     AND p.operation->>'tracker_instance_id'=b.tracker_instance_id
     AND p.operation #>> '{request,agent_id}'=agent::text AND a.id=agent
     AND a.kind='hermes' AND a.sdlc_role='project_manager'
     AND p.operation #>> '{draft,task_id}'=b.task_id::text
     AND p.operation #>> '{draft,root_task_id}'=b.root_task_id::text
     AND p.operation->>'owner_subject'=b.owner_subject
     AND jsonb_typeof(p.operation #> '{credentials,receipt}')='object'
     AND jsonb_typeof(p.operation #> '{execution_lease,receipt}')='object'
     AND jsonb_typeof(p.operation #> '{workflow_assignment,receipt}')='object'
     AND facts #> '{fleet_pm,identity}'=jsonb_build_object(
       'task',p.operation #> '{reservation,execution,key}',
       'execution_ref',p.operation #> '{reservation,assignment,execution_id}',
       'tracker_instance_ref',p.operation->'tracker_instance_id',
       'tracker_project_ref',p.operation->'project_id',
       'task_ref',to_jsonb(b.task_id::text),'root_ref',to_jsonb(b.root_task_id::text),
       'agent_ref',to_jsonb(agent::text),
       'assignment_operation_key',p.operation #> '{reservation,assignment_operation_key}',
       'assignment_ref',p.operation #> '{reservation,assignment,assignment_id}',
       'assignment_revision',p.operation #> '{reservation,assignment,version}')
     AND native_session='fleet-pm:' || session::text || ':' || agent::text || ':' || message::text
     AND (
       (NOT (facts->'fleet_pm' ? 'resume_old_run_id')
         AND (facts->'fleet_pm')-'operation_id'-'identity'='{}'::jsonb
         AND m.author_type='user' AND m.created_by_user_id=p.owner_user_id
         AND m.message_kind='user_prompt' AND m.idempotency_key='fleet-pm-intake:' || p.id::text
         AND m.body=p.operation #>> '{input,input,description}')
       OR EXISTS(SELECT 1 FROM pm_run_resumes resume
         JOIN pm_run_bindings old ON old.session_run_id=resume.old_run_id
         JOIN pm_run_checkpoints checkpoint ON checkpoint.session_run_id=old.session_run_id
         WHERE resume.new_run_id=run AND resume.message_id=message
           AND old.session_id=session AND old.agent_id=agent
           AND old.reservation->'identity'=facts #> '{fleet_pm,identity}'
           AND old.terminal_status IS NOT NULL
           AND resume.old_run_id::text=facts #>> '{fleet_pm,resume_old_run_id}'
           AND (facts->'fleet_pm')-'operation_id'-'identity'-'resume_old_run_id'='{}'::jsonb
           AND jsonb_typeof(resume.journal->'receipt')='object'
           AND jsonb_typeof(checkpoint.receipt)='object'
           AND m.author_type='system' AND m.message_kind='control'
           AND m.body=resume.journal #>> '{intent,prompt}')
     )
 );
$$;
"#;

#[cfg(test)]
mod live_schema_test {
    use super::*;

    #[tokio::test]
    #[ignore = "requires the explicitly owned native QA database"]
    async fn upgrade_owned_native_qa_through_canonical_migrator() {
        let db =
            sea_orm::Database::connect(std::env::var("FLEET_PM_MIGRATION27_TEST_URL").unwrap())
                .await
                .unwrap();
        crate::Migrator::up(&db, None).await.unwrap();
        let row = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT pg_get_functiondef('fleet_guard_hermes_dispatch()'::regprocedure) AS definition"))
            .await.unwrap().unwrap();
        let definition: String = row.try_get("", "definition").unwrap();
        assert!(definition.contains("fleet_pm_dispatch_prepared"));
        assert!(!definition.contains(OLD_BOUND_GUARD));
        let row = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT pg_get_constraintdef(oid) AS definition FROM pg_constraint
              WHERE conrelid='hermes_dispatch_journal'::regclass AND conname='hermes_dispatch_journal_check6'"))
            .await.unwrap().unwrap();
        let definition: String = row.try_get("", "definition").unwrap();
        assert!(definition.contains("fleet-pm:"));
    }

    #[tokio::test]
    #[ignore = "requires explicitly owned native QA with real PM journal history"]
    async fn native_pm_history_prevents_downgrade() {
        let db =
            sea_orm::Database::connect(std::env::var("FLEET_PM_MIGRATION27_TEST_URL").unwrap())
                .await
                .unwrap();
        let row = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT count(*) AS count FROM hermes_dispatch_journal WHERE capabilities ? 'fleet_pm'"))
            .await.unwrap().unwrap();
        let before: i64 = row.try_get("", "count").unwrap();
        assert!(before > 0, "This case requires actual native PM history");
        assert!(crate::Migrator::down(&db, Some(1)).await.is_err());
        assert_eq!(
            crate::Migrator::get_applied_migrations(&db)
                .await
                .unwrap()
                .last()
                .unwrap()
                .name(),
            Migration.name()
        );
        let row = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT count(*) AS count FROM hermes_dispatch_journal WHERE capabilities ? 'fleet_pm'"))
            .await.unwrap().unwrap();
        assert_eq!(row.try_get::<i64>("", "count").unwrap(), before);
    }
}
