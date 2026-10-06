use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE runtime_container_preparations (
                agent_id uuid NOT NULL REFERENCES agents(id),
                ordinal bigint NOT NULL CHECK(ordinal>=0),
                controller_id uuid NOT NULL CHECK(controller_id<>'00000000-0000-0000-0000-000000000000'),
                generation uuid NOT NULL UNIQUE CHECK(generation<>'00000000-0000-0000-0000-000000000000'),
                operation_id uuid NOT NULL UNIQUE CHECK(operation_id<>'00000000-0000-0000-0000-000000000000'),
                intent_sha256 text NOT NULL CHECK(intent_sha256 ~ '^[0-9a-f]{64}$'),
                created_at timestamptz NOT NULL DEFAULT now(),
                PRIMARY KEY(agent_id,ordinal)
            );
            CREATE FUNCTION fleet_guard_container_preparation() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                RAISE EXCEPTION 'container preparation history is immutable' USING ERRCODE='23514';
            END $$;
            CREATE TRIGGER fleet_container_preparation_guard
                BEFORE UPDATE OR DELETE ON runtime_container_preparations
                FOR EACH ROW EXECUTE FUNCTION fleet_guard_container_preparation();
            CREATE TRIGGER fleet_container_preparation_no_truncate
                BEFORE TRUNCATE ON runtime_container_preparations
                FOR EACH STATEMENT EXECUTE FUNCTION fleet_guard_container_preparation();"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "DO $$ BEGIN
                IF EXISTS(SELECT 1 FROM runtime_container_preparations) THEN
                    RAISE EXCEPTION 'container preparation history prevents downgrade' USING ERRCODE='23514';
                END IF;
            END $$;
            DROP TABLE runtime_container_preparations;
            DROP FUNCTION fleet_guard_container_preparation();"
        ).await?;
        Ok(())
    }
}
