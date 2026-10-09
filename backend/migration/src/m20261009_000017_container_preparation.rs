use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
CREATE TABLE runtime_container_preparations (
    agent_id uuid PRIMARY KEY REFERENCES agents(id),
    generation uuid NOT NULL UNIQUE,
    operation_id uuid NOT NULL UNIQUE,
    claim jsonb NOT NULL CHECK (jsonb_typeof(claim)='object' AND octet_length(claim::text)<=65536),
    attempted boolean NOT NULL DEFAULT false,
    receipt jsonb CHECK (jsonb_typeof(receipt)='object' AND octet_length(receipt::text)<=65536),
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK ((claim->>'agent_id'=agent_id::text AND claim->>'generation'=generation::text AND claim->>'operation_id'=operation_id::text
        AND claim->>'intent_sha256' ~ '^[a-f0-9]{64}$') IS TRUE),
    CHECK (claim ?& ARRAY['agent_id','generation','operation_id','paths','api_port','configuration_revision','configuration_sha256','intent_sha256']
        AND claim - ARRAY['agent_id','generation','operation_id','paths','api_port','configuration_revision','configuration_sha256','intent_sha256'] = '{}'::jsonb),
    CHECK (agent_id<>'00000000-0000-0000-0000-000000000000'::uuid
        AND generation<>'00000000-0000-0000-0000-000000000000'::uuid AND operation_id<>'00000000-0000-0000-0000-000000000000'::uuid),
    CHECK (receipt IS NULL OR (attempted AND receipt->>'agent_id'=agent_id::text
        AND receipt#>>'{container,registration,resource_id}'=agent_id::text
        AND receipt#>>'{container,registration,generation}'=generation::text
        AND receipt#>>'{container,registration,operation_id}'=operation_id::text
        AND receipt->'paths'=claim->'paths' AND receipt->'api_port'=claim->'api_port'
        AND receipt->'configuration_revision'=claim->'configuration_revision'
        AND receipt->'configuration_sha256'=claim->'configuration_sha256') IS TRUE)
);
CREATE FUNCTION fleet_guard_container_preparation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='DELETE' THEN RAISE EXCEPTION 'Original preparation history is immutable'; END IF;
    IF (NEW.agent_id,NEW.generation,NEW.operation_id,NEW.claim,NEW.created_at)
        IS DISTINCT FROM (OLD.agent_id,OLD.generation,OLD.operation_id,OLD.claim,OLD.created_at)
        OR (OLD.attempted AND NOT NEW.attempted)
        OR (OLD.receipt IS NOT NULL AND NEW.receipt IS DISTINCT FROM OLD.receipt)
    THEN RAISE EXCEPTION 'Original preparation intent or receipt is immutable'; END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER runtime_container_preparation_guard BEFORE UPDATE OR DELETE ON runtime_container_preparations
FOR EACH ROW EXECUTE FUNCTION fleet_guard_container_preparation();
CREATE FUNCTION fleet_guard_preparation_launch() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p runtime_container_preparations%ROWTYPE;
BEGIN
    SELECT * INTO p FROM runtime_container_preparations WHERE agent_id=NEW.agent_id;
    IF FOUND AND (p.receipt IS NULL OR NEW.generation<>p.generation OR NEW.prepared IS DISTINCT FROM p.receipt)
    THEN RAISE EXCEPTION 'Launch requires original acknowledged preparation'; END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER runtime_container_preparation_launch BEFORE INSERT ON runtime_container_launches
FOR EACH ROW EXECUTE FUNCTION fleet_guard_preparation_launch();
CREATE FUNCTION fleet_guard_preparation_config() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='DELETE' THEN
        IF EXISTS(SELECT 1 FROM runtime_container_preparations WHERE agent_id=OLD.agent_id)
        THEN RAISE EXCEPTION 'Prepared Docker configuration authority is immutable'; END IF;
        RETURN OLD;
    END IF;
    IF EXISTS(SELECT 1 FROM runtime_container_preparations p WHERE p.agent_id=NEW.agent_id
        AND NEW.effective_revision::text IS DISTINCT FROM p.claim->>'configuration_revision')
    THEN RAISE EXCEPTION 'Prepared Docker generation requires separate configuration activation'; END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER runtime_container_preparation_config BEFORE INSERT OR UPDATE OR DELETE ON agent_config_heads
FOR EACH ROW EXECUTE FUNCTION fleet_guard_preparation_config();
"#).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
DO $$ BEGIN
    IF EXISTS(SELECT 1 FROM runtime_container_preparations)
    THEN RAISE EXCEPTION 'Preparation custody history prevents downgrade'; END IF;
END $$;
DROP TRIGGER runtime_container_preparation_config ON agent_config_heads;
DROP FUNCTION fleet_guard_preparation_config();
DROP TRIGGER runtime_container_preparation_launch ON runtime_container_launches;
DROP FUNCTION fleet_guard_preparation_launch();
DROP TABLE runtime_container_preparations;
DROP FUNCTION fleet_guard_container_preparation();
"#,
            )
            .await?;
        Ok(())
    }
}
