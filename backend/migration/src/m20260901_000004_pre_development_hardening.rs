use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
ALTER TABLE users
  ADD COLUMN IF NOT EXISTS system_role text NOT NULL DEFAULT 'user';

UPDATE users
SET system_role = CASE WHEN is_system_admin THEN 'admin' ELSE 'user' END
WHERE system_role IS NULL OR system_role = '';

ALTER TABLE users DROP CONSTRAINT IF EXISTS users_system_role_check;
ALTER TABLE users
  ADD CONSTRAINT users_system_role_check CHECK (system_role IN ('admin', 'operator', 'user'));

ALTER TABLE agent_sessions
  ADD COLUMN IF NOT EXISTS idempotency_key text,
  ADD COLUMN IF NOT EXISTS idempotency_payload_hash text;

CREATE UNIQUE INDEX IF NOT EXISTS agent_sessions_user_idempotency_unique
  ON agent_sessions(user_id, idempotency_key)
  WHERE idempotency_key IS NOT NULL;

ALTER TABLE session_messages
  ADD COLUMN IF NOT EXISTS idempotency_key text,
  ADD COLUMN IF NOT EXISTS idempotency_payload_hash text,
  ADD COLUMN IF NOT EXISTS created_by_user_id uuid REFERENCES users(id) ON DELETE SET NULL;

UPDATE session_messages
SET created_by_user_id = author_user_id
WHERE created_by_user_id IS NULL
  AND author_user_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS session_messages_user_idempotency_unique
  ON session_messages(session_id, created_by_user_id, idempotency_key)
  WHERE idempotency_key IS NOT NULL AND created_by_user_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS deployment_jobs (
  id uuid PRIMARY KEY,
  job_kind text NOT NULL,
  state text NOT NULL,
  agent_id uuid REFERENCES agents(id) ON DELETE SET NULL,
  runtime_kind text,
  requested_by_user_id uuid REFERENCES users(id) ON DELETE SET NULL,
  title text NOT NULL,
  detail jsonb NOT NULL DEFAULT '{}'::jsonb,
  last_error text,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT deployment_jobs_kind_check CHECK (job_kind IN ('provision', 'runtime_update')),
  CONSTRAINT deployment_jobs_state_check CHECK (state IN ('queued', 'running', 'completed', 'failed', 'cancelled')),
  CONSTRAINT deployment_jobs_runtime_kind_check CHECK (runtime_kind IS NULL OR runtime_kind IN ('hermes', 'java_agent'))
);

CREATE INDEX IF NOT EXISTS deployment_jobs_state_created_idx
  ON deployment_jobs(state, created_at DESC);
CREATE INDEX IF NOT EXISTS deployment_jobs_agent_created_idx
  ON deployment_jobs(agent_id, created_at DESC)
  WHERE agent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS deployment_jobs_requested_by_idx
  ON deployment_jobs(requested_by_user_id, created_at DESC)
  WHERE requested_by_user_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS control_settings (
  key text PRIMARY KEY,
  value_json jsonb NOT NULL DEFAULT '{}'::jsonb,
  updated_by_user_id uuid REFERENCES users(id) ON DELETE SET NULL,
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS control_settings_updated_idx ON control_settings(updated_at DESC);
CREATE INDEX IF NOT EXISTS audit_log_entity_created_idx ON audit_log(entity_type, entity_id, created_at DESC);
"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
DROP INDEX IF EXISTS audit_log_entity_created_idx;
DROP INDEX IF EXISTS control_settings_updated_idx;
DROP TABLE IF EXISTS control_settings;

DROP INDEX IF EXISTS deployment_jobs_requested_by_idx;
DROP INDEX IF EXISTS deployment_jobs_agent_created_idx;
DROP INDEX IF EXISTS deployment_jobs_state_created_idx;
DROP TABLE IF EXISTS deployment_jobs;

DROP INDEX IF EXISTS session_messages_user_idempotency_unique;
ALTER TABLE session_messages
  DROP COLUMN IF EXISTS created_by_user_id,
  DROP COLUMN IF EXISTS idempotency_payload_hash,
  DROP COLUMN IF EXISTS idempotency_key;

DROP INDEX IF EXISTS agent_sessions_user_idempotency_unique;
ALTER TABLE agent_sessions
  DROP COLUMN IF EXISTS idempotency_payload_hash,
  DROP COLUMN IF EXISTS idempotency_key;

ALTER TABLE users DROP CONSTRAINT IF EXISTS users_system_role_check;
ALTER TABLE users DROP COLUMN IF EXISTS system_role;
"#,
        )
        .await?;
        Ok(())
    }
}
