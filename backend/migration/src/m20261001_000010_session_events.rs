use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE session_event_cursors (
                session_id uuid PRIMARY KEY REFERENCES agent_sessions(id) ON DELETE CASCADE,
                sequence bigint NOT NULL CHECK (sequence > 0)
             );
             CREATE TABLE session_events (
                session_id uuid NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
                sequence bigint NOT NULL,
                event_type text NOT NULL,
                payload jsonb NOT NULL,
                created_at timestamptz NOT NULL DEFAULT now(),
                PRIMARY KEY(session_id, sequence)
             );
             CREATE FUNCTION fleet_record_session_event() RETURNS trigger AS $$
             DECLARE target uuid; seq bigint; kind text; entity uuid;
             BEGIN
                IF TG_TABLE_NAME = 'agent_sessions' THEN
                    target := NEW.id; entity := NEW.id; kind := 'session_changed';
                ELSIF TG_TABLE_NAME = 'session_messages' THEN
                    target := NEW.session_id; entity := NEW.id; kind := 'session_message_changed';
                ELSIF TG_TABLE_NAME = 'session_agent_runs' THEN
                    target := NEW.session_id; entity := NEW.id; kind := 'session_run_changed';
                ELSE
                    target := NEW.session_id; entity := NEW.id; kind := 'runtime_approval_requested';
                END IF;
                -- Serialize sequence allocation per session until commit. A global sequence
                -- alone can skip late-committing events during cursor-based replay.
                INSERT INTO session_event_cursors(session_id, sequence) VALUES(target, 1)
                  ON CONFLICT(session_id) DO UPDATE SET sequence = session_event_cursors.sequence + 1
                  RETURNING sequence INTO seq;
                INSERT INTO session_events(session_id, sequence, event_type, payload)
                  VALUES(target, seq, kind, jsonb_build_object('type', kind, 'session_id', target, 'entity_id', entity));
                RETURN NEW;
             END;
             $$ LANGUAGE plpgsql;
             CREATE TRIGGER fleet_session_event AFTER INSERT OR UPDATE ON agent_sessions
                FOR EACH ROW EXECUTE FUNCTION fleet_record_session_event();
             CREATE TRIGGER fleet_message_event AFTER INSERT OR UPDATE ON session_messages
                FOR EACH ROW EXECUTE FUNCTION fleet_record_session_event();
             CREATE TRIGGER fleet_run_event AFTER INSERT OR UPDATE ON session_agent_runs
                FOR EACH ROW EXECUTE FUNCTION fleet_record_session_event();
             CREATE TRIGGER fleet_approval_event AFTER INSERT OR UPDATE ON runtime_approval_requests
                FOR EACH ROW EXECUTE FUNCTION fleet_record_session_event();"
        ).await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP TRIGGER fleet_session_event ON agent_sessions;
             DROP TRIGGER fleet_message_event ON session_messages;
             DROP TRIGGER fleet_run_event ON session_agent_runs;
             DROP TRIGGER fleet_approval_event ON runtime_approval_requests;
             DROP FUNCTION fleet_record_session_event();
             DROP TABLE session_events;
             DROP TABLE session_event_cursors;",
            )
            .await?;
        Ok(())
    }
}
