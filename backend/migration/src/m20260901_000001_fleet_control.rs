use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
CREATE TABLE IF NOT EXISTS users (
  id uuid PRIMARY KEY,
  email text NOT NULL UNIQUE,
  username text NOT NULL UNIQUE,
  display_name text NOT NULL,
  password_hash text NOT NULL,
  refresh_token_hash text,
  is_system_admin boolean NOT NULL DEFAULT false,
  is_active boolean NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS runtime_templates (
  kind text PRIMARY KEY,
  display_name text NOT NULL,
  implemented boolean NOT NULL DEFAULT false,
  enabled boolean NOT NULL DEFAULT true,
  description text NOT NULL,
  capabilities jsonb NOT NULL DEFAULT '{}'::jsonb,
  updated_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT runtime_templates_kind_check CHECK (kind IN ('hermes', 'java_agent'))
);

CREATE TABLE IF NOT EXISTS agents (
  id uuid PRIMARY KEY,
  ordinal integer NOT NULL UNIQUE,
  name text NOT NULL UNIQUE,
  kind text NOT NULL,
  role text NOT NULL,
  status text NOT NULL,
  display_name text NOT NULL,
  description text,
  namespace_id text,
  workflow_id text,
  runtime_version text,
  dashboard_port integer,
  api_port integer,
  runtime_path text NOT NULL,
  config_path text NOT NULL,
  workspace_path text NOT NULL,
  logs_path text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  archived_at timestamptz,
  CONSTRAINT agents_kind_check CHECK (kind IN ('hermes', 'java_agent')),
  CONSTRAINT agents_role_check CHECK (role IN ('developer', 'tester', 'custom')),
  CONSTRAINT agents_status_check CHECK (status IN ('provisioning', 'ready', 'starting', 'running', 'stopped', 'failed', 'archived'))
);

CREATE TABLE IF NOT EXISTS agent_runtime (
  agent_id uuid PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
  desired_state text NOT NULL DEFAULT 'stopped',
  pid integer,
  health_status text,
  health_detail text,
  command_preview text NOT NULL,
  env_preview jsonb NOT NULL DEFAULT '{}'::jsonb,
  started_at timestamptz,
  stopped_at timestamptz,
  last_health_at timestamptz,
  CONSTRAINT agent_runtime_desired_check CHECK (desired_state IN ('running', 'stopped'))
);

CREATE TABLE IF NOT EXISTS agent_configs (
  agent_id uuid PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
  config_json jsonb NOT NULL DEFAULT '{}'::jsonb,
  soul_md text NOT NULL DEFAULT '',
  env_json jsonb NOT NULL DEFAULT '{}'::jsonb,
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS agent_skills (
  id uuid PRIMARY KEY,
  agent_id uuid NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  name text NOT NULL,
  title text NOT NULL,
  state text NOT NULL,
  source text NOT NULL,
  content text,
  updated_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT agent_skills_state_check CHECK (state IN ('enabled', 'disabled', 'missing', 'dirty')),
  CONSTRAINT agent_skills_agent_name_unique UNIQUE(agent_id, name)
);

CREATE TABLE IF NOT EXISTS agent_sessions (
  id uuid PRIMARY KEY,
  agent_id uuid NOT NULL REFERENCES agents(id) ON DELETE RESTRICT,
  title text NOT NULL,
  task_key text,
  state text NOT NULL,
  namespace_id text,
  external_session_id text,
  last_message_preview text,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT agent_sessions_state_check CHECK (state IN ('draft', 'active', 'handoff_requested', 'blocked', 'done', 'archived'))
);

CREATE TABLE IF NOT EXISTS workflow_bindings (
  id uuid PRIMARY KEY,
  agent_id uuid NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  namespace_id text,
  namespace_name text,
  workflow_id text,
  workflow_name text,
  binding_status text NOT NULL DEFAULT 'pending',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT workflow_bindings_agent_unique UNIQUE(agent_id)
);

CREATE TABLE IF NOT EXISTS agent_events (
  id uuid PRIMARY KEY,
  agent_id uuid REFERENCES agents(id) ON DELETE SET NULL,
  event_type text NOT NULL,
  message text NOT NULL,
  payload jsonb NOT NULL DEFAULT '{}'::jsonb,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS agent_logs (
  id uuid PRIMARY KEY,
  agent_id uuid NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  stream text NOT NULL,
  message text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS audit_log (
  id uuid PRIMARY KEY,
  actor_user_id uuid REFERENCES users(id) ON DELETE SET NULL,
  action text NOT NULL,
  entity_type text NOT NULL,
  entity_id text,
  payload jsonb NOT NULL DEFAULT '{}'::jsonb,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS agents_kind_status_idx ON agents(kind, status);
CREATE INDEX IF NOT EXISTS agents_role_status_idx ON agents(role, status);
CREATE INDEX IF NOT EXISTS agent_sessions_agent_state_idx ON agent_sessions(agent_id, state, updated_at DESC);
CREATE INDEX IF NOT EXISTS agent_sessions_task_key_idx ON agent_sessions(task_key) WHERE task_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS workflow_bindings_namespace_idx ON workflow_bindings(namespace_id) WHERE namespace_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS agent_events_created_idx ON agent_events(created_at DESC);
CREATE INDEX IF NOT EXISTS agent_logs_agent_created_idx ON agent_logs(agent_id, created_at DESC);
"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
DROP TABLE IF EXISTS audit_log;
DROP TABLE IF EXISTS agent_logs;
DROP TABLE IF EXISTS agent_events;
DROP TABLE IF EXISTS workflow_bindings;
DROP TABLE IF EXISTS agent_sessions;
DROP TABLE IF EXISTS agent_skills;
DROP TABLE IF EXISTS agent_configs;
DROP TABLE IF EXISTS agent_runtime;
DROP TABLE IF EXISTS agents;
DROP TABLE IF EXISTS runtime_templates;
DROP TABLE IF EXISTS users;
"#,
        )
        .await?;
        Ok(())
    }
}
