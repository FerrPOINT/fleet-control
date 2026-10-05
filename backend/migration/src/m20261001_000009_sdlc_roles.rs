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
             CREATE INDEX agents_sdlc_role_idx ON agents(sdlc_role) WHERE archived_at IS NULL;"
        ).await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE agents DROP COLUMN sdlc_role;")
            .await?;
        Ok(())
    }
}
