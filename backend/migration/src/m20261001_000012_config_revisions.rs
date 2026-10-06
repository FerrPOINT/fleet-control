use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE agent_config_revisions (
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
            .execute_unprepared("DROP TABLE agent_config_heads; DROP TABLE agent_config_revisions;")
            .await?;
        Ok(())
    }
}
