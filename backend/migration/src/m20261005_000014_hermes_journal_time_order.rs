use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Trigger names order this after the immutable identity/ACK guard.
        manager.get_connection().execute_unprepared(
            "CREATE FUNCTION fleet_order_hermes_dispatch_time() RETURNS trigger AS $$
             BEGIN
                IF OLD.state='prepared' AND NEW.state='submitted' THEN
                    NEW.submitted_at := GREATEST(NEW.submitted_at,OLD.created_at);
                ELSIF OLD.state='submitted' AND NEW.state='accepted' THEN
                    NEW.accepted_at := GREATEST(NEW.accepted_at,OLD.submitted_at);
                END IF;
                RETURN NEW;
             END;
             $$ LANGUAGE plpgsql;
             CREATE TRIGGER fleet_hermes_dispatch_time_order BEFORE UPDATE ON hermes_dispatch_journal
                FOR EACH ROW EXECUTE FUNCTION fleet_order_hermes_dispatch_time();"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "LOCK TABLE hermes_dispatch_journal IN ACCESS EXCLUSIVE MODE;
             DO $$ BEGIN
                IF EXISTS(SELECT 1 FROM hermes_dispatch_journal) THEN
                    RAISE EXCEPTION 'Hermes journal time ordering requires reconciliation before downgrade' USING ERRCODE='23514';
                END IF;
             END $$;
             DROP TRIGGER fleet_hermes_dispatch_time_order ON hermes_dispatch_journal;
             DROP FUNCTION fleet_order_hermes_dispatch_time();"
        ).await?;
        Ok(())
    }
}
