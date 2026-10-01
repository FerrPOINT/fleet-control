use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "ALTER TABLE agents ADD COLUMN sdlc_role text NULL
                CHECK (sdlc_role IN ('project_manager','analyst','architect','developer','reviewer','tester','dev_ops'));
             UPDATE agents SET sdlc_role = role
                WHERE product_role = 'executor' AND role IN ('developer','tester');
             CREATE INDEX agents_sdlc_role_idx ON agents(sdlc_role) WHERE archived_at IS NULL;
             CREATE TABLE session_event_cursors (
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
                FOR EACH ROW EXECUTE FUNCTION fleet_record_session_event();
             CREATE TABLE message_dispatch_outbox (
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
                FOR EACH ROW EXECUTE FUNCTION fleet_queue_message();
             CREATE TABLE agent_config_revisions (
                agent_id uuid NOT NULL REFERENCES agents(id),
                revision bigint NOT NULL CHECK (revision > 0),
                state text NOT NULL CHECK (state IN ('draft','validated','activating','active','failed')),
                snapshot jsonb NOT NULL,
                validation_errors jsonb NOT NULL DEFAULT '[]',
                last_error text NULL,
                claimed_at timestamptz NULL,
                created_by_user_id uuid NOT NULL REFERENCES users(id),
                created_at timestamptz NOT NULL DEFAULT now(),
                PRIMARY KEY(agent_id, revision)
             );
             CREATE TABLE agent_config_heads (
                agent_id uuid PRIMARY KEY REFERENCES agents(id),
                desired_revision bigint NOT NULL,
                effective_revision bigint NULL,
                draining boolean NOT NULL DEFAULT false,
                FOREIGN KEY(agent_id, desired_revision) REFERENCES agent_config_revisions(agent_id, revision),
                FOREIGN KEY(agent_id, effective_revision) REFERENCES agent_config_revisions(agent_id, revision)
             );
             CREATE UNIQUE INDEX agent_config_one_activation ON agent_config_revisions(agent_id) WHERE state = 'activating';"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP TABLE agent_config_heads; DROP TABLE agent_config_revisions;
             DROP TRIGGER fleet_message_dispatch ON session_messages;
             DROP FUNCTION fleet_queue_message();
             DROP TABLE message_dispatch_outbox;
             DROP TRIGGER fleet_session_event ON agent_sessions;
             DROP TRIGGER fleet_message_event ON session_messages;
             DROP TRIGGER fleet_run_event ON session_agent_runs;
             DROP TRIGGER fleet_approval_event ON runtime_approval_requests;
             DROP FUNCTION fleet_record_session_event();
             DROP TABLE session_events;
             DROP TABLE session_event_cursors;
             ALTER TABLE agents DROP COLUMN sdlc_role;",
            )
            .await?;
        Ok(())
    }
}
