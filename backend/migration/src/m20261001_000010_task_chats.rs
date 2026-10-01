use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE task_chat_bindings (
                session_id uuid PRIMARY KEY REFERENCES agent_sessions(id),
                tracker_instance_id text NOT NULL CHECK (length(tracker_instance_id) BETWEEN 1 AND 128),
                project_id uuid NOT NULL, task_id uuid NOT NULL, root_task_id uuid NOT NULL,
                agent_id uuid NOT NULL REFERENCES agents(id),
                owner_subject text NOT NULL CHECK (length(owner_subject) BETWEEN 1 AND 256),
                idempotency_key text NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
                created_at timestamptz NOT NULL DEFAULT now(),
                UNIQUE(tracker_instance_id, task_id, agent_id)
             );
             CREATE TABLE tracker_event_cursors (
                session_id uuid PRIMARY KEY REFERENCES task_chat_bindings(session_id),
                sequence bigint NOT NULL DEFAULT 0 CHECK (sequence >= 0)
             );
             CREATE TABLE tracker_event_inbox (
                session_id uuid NOT NULL REFERENCES task_chat_bindings(session_id),
                event_id uuid NOT NULL,
                source_sequence bigint NOT NULL CHECK (source_sequence > 0),
                event_type text NOT NULL,
                payload_hash text NOT NULL CHECK (length(payload_hash) = 64),
                message_id uuid NOT NULL UNIQUE REFERENCES session_messages(id),
                source_created_at timestamptz NOT NULL,
                received_at timestamptz NOT NULL DEFAULT now(),
                PRIMARY KEY(session_id,event_id),
                UNIQUE(session_id,source_sequence)
             );
             CREATE FUNCTION fleet_guard_tracker_receipt() RETURNS trigger AS $$
             BEGIN
                RAISE EXCEPTION 'Tracker event receipt is immutable' USING ERRCODE = '23514';
             END;
             $$ LANGUAGE plpgsql;
             CREATE TRIGGER fleet_tracker_receipt_guard BEFORE UPDATE OR DELETE ON tracker_event_inbox
                FOR EACH ROW EXECUTE FUNCTION fleet_guard_tracker_receipt();
             CREATE FUNCTION fleet_guard_task_chat() RETURNS trigger AS $$
             BEGIN
                IF EXISTS (SELECT 1 FROM task_chat_bindings WHERE session_id = OLD.id)
                   AND (NEW.agent_id IS DISTINCT FROM OLD.agent_id OR NEW.user_id IS DISTINCT FROM OLD.user_id
                     OR NEW.leader_agent_id IS DISTINCT FROM OLD.leader_agent_id
                     OR NEW.visibility IS DISTINCT FROM OLD.visibility) THEN
                    RAISE EXCEPTION 'task chat identity is immutable' USING ERRCODE = '23514';
                END IF;
                RETURN NEW;
             END;
             $$ LANGUAGE plpgsql;
             CREATE TRIGGER fleet_task_chat_identity BEFORE UPDATE ON agent_sessions
                FOR EACH ROW EXECUTE FUNCTION fleet_guard_task_chat();
             CREATE INDEX session_messages_history_idx ON session_messages(session_id, created_at DESC, id DESC);
             CREATE TABLE pm_run_bindings (
                session_run_id uuid PRIMARY KEY REFERENCES session_agent_runs(id),
                session_id uuid NOT NULL REFERENCES task_chat_bindings(session_id),
                agent_id uuid NOT NULL REFERENCES agents(id),
                reservation jsonb NOT NULL,
                dispatch_operation_key text NOT NULL CHECK (length(dispatch_operation_key) BETWEEN 1 AND 128),
                runtime_session_id text NOT NULL,
                hermes_run_ref text CHECK (length(hermes_run_ref) BETWEEN 1 AND 512),
                hermes_session_ref text CHECK (length(hermes_session_ref) BETWEEN 1 AND 512),
                CHECK ((hermes_run_ref IS NULL) = (hermes_session_ref IS NULL)),
                terminal_status text CHECK (terminal_status IN ('completed','failed','cancelled','stopped')),
                created_at timestamptz NOT NULL DEFAULT now(),
                observed_at timestamptz,
                UNIQUE(agent_id, dispatch_operation_key),
                UNIQUE(agent_id, hermes_run_ref)
             );
             CREATE FUNCTION fleet_guard_pm_run() RETURNS trigger AS $$
             BEGIN
                IF NEW.session_run_id IS DISTINCT FROM OLD.session_run_id
                   OR NEW.session_id IS DISTINCT FROM OLD.session_id
                   OR NEW.agent_id IS DISTINCT FROM OLD.agent_id
                   OR NEW.reservation IS DISTINCT FROM OLD.reservation
                   OR NEW.dispatch_operation_key IS DISTINCT FROM OLD.dispatch_operation_key
                   OR NEW.runtime_session_id IS DISTINCT FROM OLD.runtime_session_id
                   OR (OLD.hermes_run_ref IS NOT NULL AND NEW.hermes_run_ref IS DISTINCT FROM OLD.hermes_run_ref)
                   OR (OLD.hermes_session_ref IS NOT NULL AND NEW.hermes_session_ref IS DISTINCT FROM OLD.hermes_session_ref)
                   OR (OLD.terminal_status IS NOT NULL AND NEW.terminal_status IS DISTINCT FROM OLD.terminal_status) THEN
                    RAISE EXCEPTION 'PM run proof is immutable' USING ERRCODE = '23514';
                END IF;
                RETURN NEW;
             END;
             $$ LANGUAGE plpgsql;
             CREATE TRIGGER fleet_pm_run_identity BEFORE UPDATE ON pm_run_bindings
                FOR EACH ROW EXECUTE FUNCTION fleet_guard_pm_run();
             CREATE INDEX pm_run_bindings_session_idx ON pm_run_bindings(session_id,created_at);
             CREATE TABLE runtime_approval_decisions (
                id uuid PRIMARY KEY,
                session_id uuid NOT NULL REFERENCES agent_sessions(id),
                approval_id uuid NOT NULL UNIQUE REFERENCES runtime_approval_requests(id),
                session_run_id uuid NOT NULL REFERENCES session_agent_runs(id),
                actor_user_id uuid NOT NULL REFERENCES users(id),
                choice text NOT NULL CHECK (choice IN ('once','deny')),
                idempotency_key text NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
                state text NOT NULL CHECK (state IN ('pending','delivered','uncertain','failed')),
                created_at timestamptz NOT NULL DEFAULT now(),
                delivered_at timestamptz,
                UNIQUE(actor_user_id,idempotency_key)
             );
             CREATE FUNCTION fleet_guard_approval_decision() RETURNS trigger AS $$
             BEGIN
                IF (to_jsonb(NEW) - 'state' - 'delivered_at') IS DISTINCT FROM (to_jsonb(OLD) - 'state' - 'delivered_at')
                   OR (OLD.state IN ('delivered','failed') AND NEW IS DISTINCT FROM OLD)
                   OR (NEW.state IS DISTINCT FROM OLD.state AND NOT (OLD.state = 'uncertain' AND NEW.state IN ('delivered','failed'))) THEN
                    RAISE EXCEPTION 'approval decision is immutable' USING ERRCODE = '23514';
                END IF;
                RETURN NEW;
             END;
             $$ LANGUAGE plpgsql;
             CREATE TRIGGER fleet_approval_decision_guard BEFORE UPDATE ON runtime_approval_decisions
                FOR EACH ROW EXECUTE FUNCTION fleet_guard_approval_decision();
             CREATE TRIGGER fleet_approval_decision_event AFTER INSERT OR UPDATE ON runtime_approval_decisions
                FOR EACH ROW EXECUTE FUNCTION fleet_record_session_event();"
        ).await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP TABLE runtime_approval_decisions; DROP FUNCTION fleet_guard_approval_decision(); DROP TABLE pm_run_bindings; DROP FUNCTION fleet_guard_pm_run();
             DROP TABLE tracker_event_inbox; DROP FUNCTION fleet_guard_tracker_receipt(); DROP TABLE tracker_event_cursors;
             DROP TRIGGER fleet_task_chat_identity ON agent_sessions;
             DROP FUNCTION fleet_guard_task_chat();
             DROP INDEX session_messages_history_idx;
             DROP TABLE task_chat_bindings;",
            )
            .await?;
        Ok(())
    }
}
