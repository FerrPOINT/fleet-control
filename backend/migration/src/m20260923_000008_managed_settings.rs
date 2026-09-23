use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS managed_settings_versions (
                    id uuid PRIMARY KEY,
                    version bigint GENERATED ALWAYS AS IDENTITY UNIQUE NOT NULL,
                    snapshot jsonb NOT NULL,
                    created_by_user_id uuid REFERENCES users(id) ON DELETE SET NULL,
                    rollback_of_version bigint NULL REFERENCES managed_settings_versions(version),
                    created_at timestamptz NOT NULL DEFAULT now(),
                    is_active boolean NOT NULL DEFAULT true
                );
                CREATE UNIQUE INDEX IF NOT EXISTS managed_settings_one_active_idx
                    ON managed_settings_versions (is_active) WHERE is_active;
                CREATE INDEX IF NOT EXISTS managed_settings_created_idx
                    ON managed_settings_versions (created_at DESC, version DESC);",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS managed_settings_versions;")
            .await?;
        Ok(())
    }
}
