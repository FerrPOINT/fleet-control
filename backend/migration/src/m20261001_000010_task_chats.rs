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
             CREATE INDEX session_messages_history_idx ON session_messages(session_id, created_at DESC, id DESC);"
        ).await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP TRIGGER fleet_task_chat_identity ON agent_sessions;
             DROP FUNCTION fleet_guard_task_chat();
             DROP INDEX session_messages_history_idx;
             DROP TABLE task_chat_bindings;",
            )
            .await?;
        Ok(())
    }
}
