use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE message_dispatch_outbox (
                message_id uuid PRIMARY KEY REFERENCES session_messages(id) ON DELETE CASCADE,
                agent_id uuid NOT NULL REFERENCES agents(id),
                state text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','dispatching','dispatched','uncertain','failed')),
                last_error text NULL,
                created_at timestamptz NOT NULL DEFAULT now(),
                updated_at timestamptz NOT NULL DEFAULT now()
             );
             CREATE INDEX message_dispatch_pending_idx ON message_dispatch_outbox(created_at) WHERE state = 'pending';
             CREATE UNIQUE INDEX message_dispatch_agent_guard ON message_dispatch_outbox(agent_id) WHERE state IN ('dispatching','uncertain');
             CREATE FUNCTION fleet_queue_message() RETURNS trigger AS $$
             BEGIN
                IF NEW.message_kind = 'user_prompt' AND NEW.delivery_state = 'pending' THEN
                    INSERT INTO message_dispatch_outbox(message_id, agent_id)
                        SELECT NEW.id, agent_id FROM agent_sessions WHERE id = NEW.session_id;
                END IF;
                RETURN NEW;
             END;
             $$ LANGUAGE plpgsql;
             CREATE TRIGGER fleet_message_dispatch AFTER INSERT ON session_messages
                FOR EACH ROW EXECUTE FUNCTION fleet_queue_message();"
        ).await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP TRIGGER fleet_message_dispatch ON session_messages;
             DROP FUNCTION fleet_queue_message();
             DROP TABLE message_dispatch_outbox;",
            )
            .await?;
        Ok(())
    }
}
