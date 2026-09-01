use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
ALTER TABLE agent_sessions
  ADD COLUMN IF NOT EXISTS user_id uuid REFERENCES users(id) ON DELETE RESTRICT;

INSERT INTO users (
  id,
  email,
  username,
  display_name,
  password_hash,
  is_system_admin,
  is_active
)
SELECT
  '00000000-0000-0000-0000-000000000002'::uuid,
  'legacy-session-owner@fleet-control.local',
  'legacy-session-owner',
  'Legacy Session Owner',
  '!legacy-session-owner',
  false,
  false
WHERE EXISTS (SELECT 1 FROM agent_sessions WHERE user_id IS NULL)
  AND NOT EXISTS (SELECT 1 FROM users)
ON CONFLICT (id) DO NOTHING;

UPDATE agent_sessions
SET user_id = (
  SELECT id
  FROM users
  ORDER BY created_at ASC, id ASC
  LIMIT 1
)
WHERE user_id IS NULL
  AND EXISTS (SELECT 1 FROM users);

ALTER TABLE agent_sessions ALTER COLUMN user_id SET NOT NULL;

CREATE INDEX IF NOT EXISTS agent_sessions_user_state_idx ON agent_sessions(user_id, state, updated_at DESC);
CREATE INDEX IF NOT EXISTS agent_sessions_agent_user_idx ON agent_sessions(agent_id, user_id, updated_at DESC);
"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
DROP INDEX IF EXISTS agent_sessions_agent_user_idx;
DROP INDEX IF EXISTS agent_sessions_user_state_idx;
ALTER TABLE agent_sessions DROP COLUMN IF EXISTS user_id;
DELETE FROM users
WHERE id = '00000000-0000-0000-0000-000000000002'::uuid
  AND email = 'legacy-session-owner@fleet-control.local'
  AND username = 'legacy-session-owner';
"#,
        )
        .await?;
        Ok(())
    }
}
