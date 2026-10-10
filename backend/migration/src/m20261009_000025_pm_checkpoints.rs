use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("CREATE TABLE pm_run_checkpoints (
            session_run_id uuid PRIMARY KEY REFERENCES pm_run_bindings(session_run_id),
            command jsonb NOT NULL CHECK(jsonb_typeof(command)='object'),
            request_sha256 text NOT NULL CHECK(request_sha256 ~ '^[0-9a-f]{64}$'),
            receipt jsonb CHECK(receipt IS NULL OR jsonb_typeof(receipt)='object'),
            created_at timestamptz NOT NULL DEFAULT clock_timestamp());
          CREATE FUNCTION fleet_guard_pm_checkpoint() RETURNS trigger AS $$ BEGIN
            IF TG_OP='DELETE' OR NEW.session_run_id IS DISTINCT FROM OLD.session_run_id
              OR NEW.command IS DISTINCT FROM OLD.command OR NEW.request_sha256 IS DISTINCT FROM OLD.request_sha256
              OR NEW.created_at IS DISTINCT FROM OLD.created_at
              OR (OLD.receipt IS NOT NULL AND NEW.receipt IS DISTINCT FROM OLD.receipt) THEN
                RAISE EXCEPTION 'PM checkpoint intent and acknowledgement are immutable' USING ERRCODE='23514';
            END IF;
            RETURN NEW;
          END $$ LANGUAGE plpgsql;
          CREATE TRIGGER pm_run_checkpoint_guard BEFORE UPDATE OR DELETE ON pm_run_checkpoints
            FOR EACH ROW EXECUTE FUNCTION fleet_guard_pm_checkpoint();").await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DO $$ BEGIN
            IF EXISTS(SELECT 1 FROM pm_run_checkpoints) THEN
              RAISE EXCEPTION 'PM checkpoint downgrade would discard execution evidence';
            END IF;
          END $$;
          DROP TABLE pm_run_checkpoints;
          DROP FUNCTION fleet_guard_pm_checkpoint();",
            )
            .await?;
        Ok(())
    }
}
