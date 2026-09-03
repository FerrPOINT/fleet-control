use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
CREATE EXTENSION IF NOT EXISTS pgcrypto;

ALTER TABLE agents
  ADD COLUMN IF NOT EXISTS product_role text NOT NULL DEFAULT 'executor';

CREATE SEQUENCE IF NOT EXISTS agent_ordinal_seq;
SELECT setval(
  'agent_ordinal_seq',
  GREATEST(COALESCE((SELECT MAX(ordinal) FROM agents), 0) + 1, 1),
  false
);
ALTER SEQUENCE agent_ordinal_seq OWNED BY agents.ordinal;

ALTER TABLE agents DROP CONSTRAINT IF EXISTS agents_role_check;
ALTER TABLE agents DROP CONSTRAINT IF EXISTS agents_product_role_check;
ALTER TABLE agents
  ADD CONSTRAINT agents_role_check CHECK (role IN ('developer', 'tester', 'it_lead', 'custom')),
  ADD CONSTRAINT agents_product_role_check CHECK (product_role IN ('leader', 'executor'));

UPDATE agents
SET product_role = 'executor'
WHERE product_role IS NULL;

ALTER TABLE agent_sessions
  ADD COLUMN IF NOT EXISTS leader_agent_id uuid REFERENCES agents(id) ON DELETE SET NULL,
  ADD COLUMN IF NOT EXISTS parent_session_id uuid REFERENCES agent_sessions(id) ON DELETE SET NULL,
  ADD COLUMN IF NOT EXISTS created_by_leader_agent_id uuid REFERENCES agents(id) ON DELETE SET NULL,
  ADD COLUMN IF NOT EXISTS visibility text NOT NULL DEFAULT 'private';

ALTER TABLE agent_sessions DROP CONSTRAINT IF EXISTS agent_sessions_visibility_check;
ALTER TABLE agent_sessions
  ADD CONSTRAINT agent_sessions_visibility_check CHECK (visibility IN ('private', 'leader_scoped'));

CREATE TABLE IF NOT EXISTS leader_executors (
  leader_agent_id uuid NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  executor_agent_id uuid NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  created_by_user_id uuid REFERENCES users(id) ON DELETE SET NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (leader_agent_id, executor_agent_id),
  CONSTRAINT leader_executors_not_self CHECK (leader_agent_id <> executor_agent_id)
);

CREATE TABLE IF NOT EXISTS session_participants (
  id uuid PRIMARY KEY,
  session_id uuid NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
  participant_type text NOT NULL,
  user_id uuid REFERENCES users(id) ON DELETE CASCADE,
  agent_id uuid REFERENCES agents(id) ON DELETE CASCADE,
  session_role text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT session_participants_type_check CHECK (participant_type IN ('user', 'agent')),
  CONSTRAINT session_participants_role_check CHECK (session_role IN ('owner', 'primary', 'leader', 'executor', 'observer')),
  CONSTRAINT session_participants_one_subject CHECK (
    (participant_type = 'user' AND user_id IS NOT NULL AND agent_id IS NULL)
    OR
    (participant_type = 'agent' AND agent_id IS NOT NULL AND user_id IS NULL)
  )
);

CREATE UNIQUE INDEX IF NOT EXISTS session_participants_user_unique
  ON session_participants(session_id, user_id, session_role)
  WHERE user_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS session_participants_agent_unique
  ON session_participants(session_id, agent_id, session_role)
  WHERE agent_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS session_messages (
  id uuid PRIMARY KEY,
  session_id uuid NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
  author_type text NOT NULL,
  author_user_id uuid REFERENCES users(id) ON DELETE SET NULL,
  author_agent_id uuid REFERENCES agents(id) ON DELETE SET NULL,
  body text NOT NULL,
  message_kind text NOT NULL,
  runtime_message_id text,
  created_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT session_messages_author_type_check CHECK (author_type IN ('user', 'agent', 'system')),
  CONSTRAINT session_messages_kind_check CHECK (message_kind IN ('user_prompt', 'assistant_message', 'tool_event', 'system_event', 'control')),
  CONSTRAINT session_messages_author_subject_check CHECK (
    (author_type = 'user' AND author_user_id IS NOT NULL AND author_agent_id IS NULL)
    OR
    (author_type = 'agent' AND author_agent_id IS NOT NULL AND author_user_id IS NULL)
    OR
    (author_type = 'system' AND author_user_id IS NULL AND author_agent_id IS NULL)
  )
);

CREATE TABLE IF NOT EXISTS session_agent_runs (
  id uuid PRIMARY KEY,
  session_id uuid NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
  agent_id uuid NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  runtime_session_id text,
  run_role text NOT NULL,
  state text NOT NULL,
  last_error text,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT session_agent_runs_role_check CHECK (run_role IN ('primary', 'leader', 'executor')),
  CONSTRAINT session_agent_runs_state_check CHECK (state IN ('pending', 'running', 'waiting', 'completed', 'failed', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS agents_product_role_status_idx ON agents(product_role, status);
CREATE INDEX IF NOT EXISTS leader_executors_executor_idx ON leader_executors(executor_agent_id, leader_agent_id);
CREATE INDEX IF NOT EXISTS agent_sessions_leader_state_idx ON agent_sessions(leader_agent_id, state, updated_at DESC) WHERE leader_agent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS agent_sessions_visibility_user_idx ON agent_sessions(visibility, user_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS agent_sessions_parent_idx ON agent_sessions(parent_session_id) WHERE parent_session_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS session_participants_agent_idx ON session_participants(agent_id, session_id) WHERE agent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS session_participants_user_idx ON session_participants(user_id, session_id) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS session_messages_session_created_idx ON session_messages(session_id, created_at ASC, id ASC);
CREATE INDEX IF NOT EXISTS session_agent_runs_agent_state_idx ON session_agent_runs(agent_id, state, updated_at DESC);
CREATE INDEX IF NOT EXISTS session_agent_runs_session_idx ON session_agent_runs(session_id, created_at ASC);
CREATE INDEX IF NOT EXISTS audit_log_actor_created_idx ON audit_log(actor_user_id, created_at DESC) WHERE actor_user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS audit_log_action_created_idx ON audit_log(action, created_at DESC);

INSERT INTO session_participants (id, session_id, participant_type, user_id, agent_id, session_role, created_at)
SELECT gen_random_uuid(), id, 'user', user_id, NULL, 'owner', created_at
FROM agent_sessions
ON CONFLICT DO NOTHING;

INSERT INTO session_participants (id, session_id, participant_type, user_id, agent_id, session_role, created_at)
SELECT gen_random_uuid(), id, 'agent', NULL, agent_id, 'primary', created_at
FROM agent_sessions
ON CONFLICT DO NOTHING;

INSERT INTO session_agent_runs (id, session_id, agent_id, runtime_session_id, run_role, state, created_at, updated_at)
SELECT gen_random_uuid(), id, agent_id, external_session_id, 'primary', 'pending', created_at, updated_at
FROM agent_sessions
ON CONFLICT DO NOTHING;
"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
DROP INDEX IF EXISTS session_agent_runs_session_idx;
DROP INDEX IF EXISTS session_agent_runs_agent_state_idx;
DROP INDEX IF EXISTS session_messages_session_created_idx;
DROP INDEX IF EXISTS audit_log_action_created_idx;
DROP INDEX IF EXISTS audit_log_actor_created_idx;
DROP INDEX IF EXISTS session_participants_user_idx;
DROP INDEX IF EXISTS session_participants_agent_idx;
DROP INDEX IF EXISTS agent_sessions_parent_idx;
DROP INDEX IF EXISTS agent_sessions_visibility_user_idx;
DROP INDEX IF EXISTS agent_sessions_leader_state_idx;
DROP INDEX IF EXISTS leader_executors_executor_idx;
DROP INDEX IF EXISTS agents_product_role_status_idx;

DROP TABLE IF EXISTS session_agent_runs;
DROP TABLE IF EXISTS session_messages;
DROP TABLE IF EXISTS session_participants;
DROP TABLE IF EXISTS leader_executors;

ALTER TABLE agent_sessions DROP CONSTRAINT IF EXISTS agent_sessions_visibility_check;
ALTER TABLE agent_sessions
  DROP COLUMN IF EXISTS visibility,
  DROP COLUMN IF EXISTS created_by_leader_agent_id,
  DROP COLUMN IF EXISTS parent_session_id,
  DROP COLUMN IF EXISTS leader_agent_id;

ALTER TABLE agents DROP CONSTRAINT IF EXISTS agents_product_role_check;
ALTER TABLE agents DROP CONSTRAINT IF EXISTS agents_role_check;
ALTER TABLE agents
  ADD CONSTRAINT agents_role_check CHECK (role IN ('developer', 'tester', 'custom'));
ALTER TABLE agents DROP COLUMN IF EXISTS product_role;
DROP SEQUENCE IF EXISTS agent_ordinal_seq;
"#,
        )
        .await?;
        Ok(())
    }
}
