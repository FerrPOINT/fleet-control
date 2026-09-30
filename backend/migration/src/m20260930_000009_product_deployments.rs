use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE deployment_jobs DROP CONSTRAINT deployment_jobs_kind_check;
                 ALTER TABLE deployment_jobs ADD CONSTRAINT deployment_jobs_kind_check
                   CHECK (job_kind IN ('provision', 'runtime_update', 'product_deploy', 'product_rollback'));
                 ALTER TABLE deployment_jobs ADD COLUMN idempotency_key uuid;
                 CREATE UNIQUE INDEX deployment_jobs_product_idempotency_idx
                   ON deployment_jobs (idempotency_key)
                   WHERE idempotency_key IS NOT NULL;",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP INDEX IF EXISTS deployment_jobs_product_idempotency_idx;
                 ALTER TABLE deployment_jobs DROP COLUMN idempotency_key;
                 ALTER TABLE deployment_jobs DROP CONSTRAINT deployment_jobs_kind_check;
                 ALTER TABLE deployment_jobs ADD CONSTRAINT deployment_jobs_kind_check
                   CHECK (job_kind IN ('provision', 'runtime_update'));",
            )
            .await?;
        Ok(())
    }
}
