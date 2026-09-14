use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
CREATE TABLE IF NOT EXISTS fleet_alerts (
  id uuid PRIMARY KEY,
  agent_id uuid REFERENCES agents(id) ON DELETE CASCADE,
  kind text NOT NULL,
  severity text NOT NULL DEFAULT 'warning',
  detail jsonb NOT NULL DEFAULT '{}'::jsonb,
  state text NOT NULL DEFAULT 'open',
  opened_at timestamptz NOT NULL DEFAULT now(),
  resolved_at timestamptz,
  acknowledged_at timestamptz,
  acknowledged_by_user_id uuid REFERENCES users(id) ON DELETE SET NULL,
  CONSTRAINT fleet_alerts_kind_check CHECK (kind IN ('agent_down', 'agent_recovered', 'agent_restart_loop', 'heartbeat_stale')),
  CONSTRAINT fleet_alerts_severity_check CHECK (severity IN ('info', 'warning', 'critical')),
  CONSTRAINT fleet_alerts_state_check CHECK (state IN ('open', 'acknowledged', 'resolved'))
);

CREATE INDEX IF NOT EXISTS fleet_alerts_agent_idx ON fleet_alerts(agent_id);
CREATE INDEX IF NOT EXISTS fleet_alerts_state_idx ON fleet_alerts(state);
"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(r#"DROP TABLE IF EXISTS fleet_alerts;"#)
            .await?;
        Ok(())
    }
}
