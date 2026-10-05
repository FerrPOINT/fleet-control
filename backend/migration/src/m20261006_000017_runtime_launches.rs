use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE runtime_launches (
                id uuid PRIMARY KEY,
                agent_id uuid NOT NULL REFERENCES agents(id),
                controller_id uuid NOT NULL,
                binding jsonb NOT NULL CHECK(jsonb_typeof(binding)='object'
                    AND octet_length(binding::text)<=16384
                    AND binding ?& ARRAY['id','agent_id','controller_id']
                    AND (binding->>'id'=id::text AND binding->>'agent_id'=agent_id::text
                    AND binding->>'controller_id'=controller_id::text) IS TRUE),
                state text NOT NULL DEFAULT 'claimed'
                    CHECK(state IN ('claimed','gateway_started','gateway_exited','spawn_failed')),
                pid integer CHECK(pid>0),
                created_at timestamptz NOT NULL DEFAULT now(),
                observed_at timestamptz,
                CHECK((state='claimed')=(observed_at IS NULL)),
                CHECK((state IN ('gateway_started','gateway_exited'))=(pid IS NOT NULL))
            );
            CREATE UNIQUE INDEX runtime_launches_open_agent ON runtime_launches(agent_id)
                WHERE state IN ('claimed','gateway_started');
            CREATE FUNCTION fleet_guard_runtime_launch() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF TG_OP IN ('DELETE','TRUNCATE') THEN
                    RAISE EXCEPTION 'runtime launch history cannot be removed' USING ERRCODE='23514';
                END IF;
                IF TG_OP='INSERT' THEN
                    IF NEW.state<>'claimed' OR NEW.pid IS NOT NULL OR NEW.observed_at IS NOT NULL THEN
                        RAISE EXCEPTION 'runtime launch must begin before spawn' USING ERRCODE='23514';
                    END IF;
                    RETURN NEW;
                END IF;
                IF (to_jsonb(NEW)-'state'-'pid'-'observed_at') IS DISTINCT FROM
                    (to_jsonb(OLD)-'state'-'pid'-'observed_at')
                    OR (OLD.state IN ('gateway_exited','spawn_failed') AND NEW IS DISTINCT FROM OLD)
                    OR (OLD.pid IS NOT NULL AND NEW.pid IS DISTINCT FROM OLD.pid)
                    OR (NEW.state=OLD.state AND NEW.observed_at IS DISTINCT FROM OLD.observed_at)
                    OR (NEW.state IS DISTINCT FROM OLD.state AND NOT
                        (OLD.state='claimed' AND NEW.state IN ('gateway_started','gateway_exited','spawn_failed')
                        OR OLD.state='gateway_started' AND NEW.state='gateway_exited')) THEN
                    RAISE EXCEPTION 'original runtime launch is immutable' USING ERRCODE='23514';
                END IF;
                RETURN NEW;
            END $$;
            CREATE TRIGGER fleet_runtime_launch_guard BEFORE INSERT OR UPDATE OR DELETE ON runtime_launches
                FOR EACH ROW EXECUTE FUNCTION fleet_guard_runtime_launch();
            CREATE TRIGGER fleet_runtime_launch_no_truncate BEFORE TRUNCATE ON runtime_launches
                FOR EACH STATEMENT EXECUTE FUNCTION fleet_guard_runtime_launch();"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DO $$ BEGIN
                    IF EXISTS(SELECT 1 FROM runtime_launches) THEN
                        RAISE EXCEPTION 'runtime launch history prevents downgrade' USING ERRCODE='23514';
                    END IF;
                END $$;
                DROP TABLE runtime_launches; DROP FUNCTION fleet_guard_runtime_launch();",
            )
            .await?;
        Ok(())
    }
}
