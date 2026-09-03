use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
ALTER TABLE agents DROP CONSTRAINT IF EXISTS agents_status_check;
ALTER TABLE agents
  ADD CONSTRAINT agents_status_check CHECK (status IN ('provisioning', 'ready', 'starting', 'running', 'degraded', 'stopped', 'failed', 'archived'));

ALTER TABLE agent_runtime
  ADD COLUMN IF NOT EXISTS last_capabilities_json jsonb NOT NULL DEFAULT '{}'::jsonb,
  ADD COLUMN IF NOT EXISTS startup_command_redacted text;

ALTER TABLE session_agent_runs DROP CONSTRAINT IF EXISTS session_agent_runs_state_check;
ALTER TABLE session_agent_runs
  ADD COLUMN IF NOT EXISTS runtime_run_id text,
  ADD COLUMN IF NOT EXISTS last_event_at timestamptz,
  ADD COLUMN IF NOT EXISTS model text,
  ADD COLUMN IF NOT EXISTS provider text,
  ADD COLUMN IF NOT EXISTS model_options jsonb NOT NULL DEFAULT '{}'::jsonb,
  ADD CONSTRAINT session_agent_runs_state_check CHECK (state IN ('pending', 'running', 'waiting', 'completed', 'failed', 'cancelled', 'stopping'));

ALTER TABLE session_messages
  ADD COLUMN IF NOT EXISTS delivery_state text NOT NULL DEFAULT 'mirrored',
  ADD COLUMN IF NOT EXISTS delivery_error text;

ALTER TABLE session_messages DROP CONSTRAINT IF EXISTS session_messages_delivery_state_check;
ALTER TABLE session_messages
  ADD CONSTRAINT session_messages_delivery_state_check CHECK (delivery_state IN ('pending', 'dispatched', 'completed', 'failed', 'mirrored'));

CREATE TABLE IF NOT EXISTS runtime_approval_requests (
  id uuid PRIMARY KEY,
  session_id uuid NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
  session_run_id uuid NOT NULL REFERENCES session_agent_runs(id) ON DELETE CASCADE,
  agent_id uuid NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  runtime_run_id text NOT NULL,
  runtime_approval_id text,
  prompt text NOT NULL,
  detail jsonb NOT NULL DEFAULT '{}'::jsonb,
  state text NOT NULL DEFAULT 'pending',
  resolved_by_user_id uuid REFERENCES users(id) ON DELETE SET NULL,
  resolved_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT runtime_approval_requests_state_check CHECK (state IN ('pending', 'approved', 'denied', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS agent_runtime_health_idx ON agent_runtime(last_health_at DESC);
CREATE INDEX IF NOT EXISTS session_agent_runs_runtime_run_idx ON session_agent_runs(runtime_run_id) WHERE runtime_run_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS session_agent_runs_last_event_idx ON session_agent_runs(last_event_at DESC) WHERE last_event_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS session_messages_delivery_idx ON session_messages(session_id, delivery_state, created_at ASC);
CREATE INDEX IF NOT EXISTS runtime_approval_requests_session_state_idx ON runtime_approval_requests(session_id, state, created_at DESC);
CREATE UNIQUE INDEX IF NOT EXISTS runtime_approval_requests_runtime_unique
  ON runtime_approval_requests(session_run_id, runtime_approval_id)
  WHERE runtime_approval_id IS NOT NULL;
"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
DROP INDEX IF EXISTS runtime_approval_requests_runtime_unique;
DROP INDEX IF EXISTS runtime_approval_requests_session_state_idx;
DROP INDEX IF EXISTS session_messages_delivery_idx;
DROP INDEX IF EXISTS session_agent_runs_last_event_idx;
DROP INDEX IF EXISTS session_agent_runs_runtime_run_idx;
DROP INDEX IF EXISTS agent_runtime_health_idx;
DROP TABLE IF EXISTS runtime_approval_requests;

ALTER TABLE session_messages DROP CONSTRAINT IF EXISTS session_messages_delivery_state_check;
ALTER TABLE session_messages
  DROP COLUMN IF EXISTS delivery_error,
  DROP COLUMN IF EXISTS delivery_state;

ALTER TABLE session_agent_runs DROP CONSTRAINT IF EXISTS session_agent_runs_state_check;
ALTER TABLE session_agent_runs
  DROP COLUMN IF EXISTS model_options,
  DROP COLUMN IF EXISTS provider,
  DROP COLUMN IF EXISTS model,
  DROP COLUMN IF EXISTS last_event_at,
  DROP COLUMN IF EXISTS runtime_run_id,
  ADD CONSTRAINT session_agent_runs_state_check CHECK (state IN ('pending', 'running', 'waiting', 'completed', 'failed', 'cancelled'));

ALTER TABLE agent_runtime
  DROP COLUMN IF EXISTS startup_command_redacted,
  DROP COLUMN IF EXISTS last_capabilities_json;

ALTER TABLE agents DROP CONSTRAINT IF EXISTS agents_status_check;
ALTER TABLE agents
  ADD CONSTRAINT agents_status_check CHECK (status IN ('provisioning', 'ready', 'starting', 'running', 'stopped', 'failed', 'archived'));
"#,
        )
        .await?;
        Ok(())
    }
}
