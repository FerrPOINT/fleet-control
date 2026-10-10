use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("CREATE TABLE pm_run_resumes (
          old_run_id uuid PRIMARY KEY REFERENCES pm_run_bindings(session_run_id),
          new_run_id uuid NOT NULL UNIQUE, message_id uuid NOT NULL UNIQUE,
          journal jsonb NOT NULL CHECK(jsonb_typeof(journal)='object'),
          created_at timestamptz NOT NULL DEFAULT clock_timestamp());
          CREATE FUNCTION fleet_guard_pm_resume() RETURNS trigger AS $$ BEGIN
            IF TG_OP='DELETE' OR NEW.old_run_id IS DISTINCT FROM OLD.old_run_id
              OR NEW.new_run_id IS DISTINCT FROM OLD.new_run_id OR NEW.message_id IS DISTINCT FROM OLD.message_id
              OR NEW.created_at IS DISTINCT FROM OLD.created_at
              OR (NEW.journal-'receipt') IS DISTINCT FROM (OLD.journal-'receipt')
              OR (OLD.journal->'receipt'<>'null'::jsonb AND NEW.journal->'receipt' IS DISTINCT FROM OLD.journal->'receipt') THEN
              RAISE EXCEPTION 'PM original resume intent and acknowledgement are immutable' USING ERRCODE='23514';
            END IF;
            RETURN NEW;
          END $$ LANGUAGE plpgsql;
          CREATE TRIGGER pm_run_resume_guard BEFORE UPDATE OR DELETE ON pm_run_resumes
            FOR EACH ROW EXECUTE FUNCTION fleet_guard_pm_resume();").await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("DO $$ BEGIN
            IF EXISTS(SELECT 1 FROM pm_run_resumes) THEN RAISE EXCEPTION 'PM resume downgrade would discard execution evidence'; END IF;
          END $$; DROP TABLE pm_run_resumes; DROP FUNCTION fleet_guard_pm_resume();").await?;
        Ok(())
    }
}
