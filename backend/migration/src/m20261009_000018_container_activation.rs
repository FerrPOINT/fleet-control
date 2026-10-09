use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
CREATE TABLE runtime_container_activations (
    id uuid PRIMARY KEY,
    agent_id uuid NOT NULL REFERENCES agents(id),
    revision bigint NOT NULL,
    record jsonb NOT NULL CHECK(jsonb_typeof(record)='object' AND octet_length(record::text)<=262144),
    UNIQUE(agent_id,revision),
    FOREIGN KEY(agent_id,revision) REFERENCES agent_config_revisions(agent_id,revision),
    CHECK ((record#>>'{claim,id}'=id::text AND record#>>'{claim,agent_id}'=agent_id::text
        AND record#>>'{claim,revision}'=revision::text AND record#>>'{claim,intent_sha256}' ~ '^[a-f0-9]{64}$') IS TRUE)
);
CREATE UNIQUE INDEX runtime_container_activation_open ON runtime_container_activations(agent_id)
    WHERE record->>'phase' NOT IN ('committed','rolled_back');
CREATE FUNCTION fleet_guard_container_activation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE edge text;
BEGIN
    IF TG_OP='DELETE' THEN RAISE EXCEPTION 'Activation custody is immutable'; END IF;
    IF TG_OP='INSERT' THEN
        IF NEW.record->>'phase'<>'planned' OR NEW.record->'previous_stop'<>'null'::jsonb
            OR NEW.record->'candidate'<>'null'::jsonb OR NEW.record->'rollback'<>'null'::jsonb
            OR NEW.record->'readiness'<>'null'::jsonb
        THEN RAISE EXCEPTION 'Activation must be planned before effects'; END IF;
    ELSE
        edge := (OLD.record->>'phase') || ':' || (NEW.record->>'phase');
        IF (NEW.id,NEW.agent_id,NEW.revision,NEW.record->'claim') IS DISTINCT FROM
            (OLD.id,OLD.agent_id,OLD.revision,OLD.record->'claim') OR edge NOT IN (
            'planned:stopping_previous','stopping_previous:previous_stopped','previous_stopped:applying_candidate',
            'applying_candidate:preparing_candidate','preparing_candidate:candidate_prepared',
            'candidate_prepared:starting_candidate','starting_candidate:candidate_running',
            'candidate_running:candidate_ready','candidate_ready:committed',
            'candidate_running:stopping_candidate','stopping_candidate:candidate_stopped',
            'candidate_stopped:applying_rollback','applying_candidate:applying_rollback',
            'applying_rollback:preparing_rollback','preparing_rollback:rollback_prepared',
            'rollback_prepared:starting_rollback','starting_rollback:rollback_running',
            'rollback_running:rollback_ready','rollback_ready:rolled_back')
            OR (OLD.record->'previous_stop'<>'null'::jsonb AND NEW.record->'previous_stop' IS DISTINCT FROM OLD.record->'previous_stop')
            OR (OLD.record->'candidate_stop'<>'null'::jsonb AND NEW.record->'candidate_stop' IS DISTINCT FROM OLD.record->'candidate_stop')
            OR (OLD.record->'readiness'<>'null'::jsonb AND NEW.record->'readiness' IS DISTINCT FROM OLD.record->'readiness')
        THEN RAISE EXCEPTION 'Activation identity, evidence or progress changed'; END IF;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER runtime_container_activation_guard BEFORE INSERT OR UPDATE OR DELETE ON runtime_container_activations
    FOR EACH ROW EXECUTE FUNCTION fleet_guard_container_activation();

-- Preserve the exact historical definitions/OIDs for an empty-history downgrade.
ALTER FUNCTION fleet_guard_preparation_launch() RENAME TO fleet_guard_preparation_launch_v17;
ALTER FUNCTION fleet_guard_preparation_config() RENAME TO fleet_guard_preparation_config_v17;
DROP TRIGGER runtime_container_preparation_launch ON runtime_container_launches;
DROP TRIGGER runtime_container_preparation_config ON agent_config_heads;
CREATE FUNCTION fleet_guard_preparation_launch() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE a jsonb; l jsonb;
BEGIN
    IF EXISTS(SELECT 1 FROM runtime_container_preparations p WHERE p.agent_id=NEW.agent_id
        AND p.generation=NEW.generation AND p.receipt=NEW.prepared) THEN RETURN NEW; END IF;
    SELECT record INTO a FROM runtime_container_activations WHERE agent_id=NEW.agent_id
        AND id::text=current_setting('fleet.container_activation_id',true);
    l := CASE WHEN a->>'phase'='candidate_prepared' THEN a->'candidate'
              WHEN a->>'phase'='rollback_prepared' THEN a->'rollback' END;
    IF (l->'prepared'=NEW.prepared AND l->>'controller_id'=NEW.controller_id::text
        AND l->>'stop_id'=NEW.stop_id::text AND l->>'state'='claimed'
        AND a#>>'{previous_stop,observation}'='namespace_exited'
        AND EXISTS(SELECT 1 FROM runtime_container_launches c WHERE c.agent_id=NEW.agent_id
            AND c.generation::text=a#>>'{claim,previous,prepared,container,registration,generation}' AND c.state='exited')
        AND EXISTS(SELECT 1 FROM agent_config_heads h WHERE h.agent_id=NEW.agent_id AND h.draining
            AND h.desired_revision::text=a#>>'{claim,revision}')) IS TRUE THEN RETURN NEW; END IF;
    IF EXISTS(SELECT 1 FROM runtime_container_preparations WHERE agent_id=NEW.agent_id)
        OR EXISTS(SELECT 1 FROM runtime_container_activations WHERE agent_id=NEW.agent_id)
    THEN RAISE EXCEPTION 'Replacement requires original stopped activation and acknowledged preparation'; END IF;
    RETURN NEW;
END $$;
CREATE FUNCTION fleet_guard_preparation_config() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE a jsonb; l jsonb;
BEGIN
    IF TG_OP='DELETE' THEN
        IF EXISTS(SELECT 1 FROM runtime_container_preparations WHERE agent_id=OLD.agent_id)
            OR EXISTS(SELECT 1 FROM runtime_container_activations WHERE agent_id=OLD.agent_id)
            OR EXISTS(SELECT 1 FROM runtime_container_launches WHERE agent_id=OLD.agent_id)
        THEN RAISE EXCEPTION 'Docker configuration custody is immutable'; END IF;
        RETURN OLD;
    END IF;
    IF TG_OP='UPDATE' AND NEW.effective_revision IS NOT DISTINCT FROM OLD.effective_revision
        AND (NEW.draining OR NOT OLD.draining) THEN RETURN NEW; END IF;
    IF TG_OP='INSERT' AND EXISTS(SELECT 1 FROM runtime_container_preparations p WHERE p.agent_id=NEW.agent_id
        AND NEW.effective_revision::text IS NOT DISTINCT FROM p.claim->>'configuration_revision') THEN RETURN NEW; END IF;
    IF NOT EXISTS(SELECT 1 FROM runtime_container_preparations WHERE agent_id=NEW.agent_id)
        AND NOT EXISTS(SELECT 1 FROM runtime_container_launches WHERE agent_id=NEW.agent_id) THEN RETURN NEW; END IF;
    SELECT record INTO a FROM runtime_container_activations WHERE agent_id=NEW.agent_id
        AND id::text=current_setting('fleet.container_activation_id',true);
    l := CASE WHEN a->>'phase'='committed' THEN a->'candidate'
              WHEN a->>'phase'='rolled_back' THEN a->'rollback' END;
    IF (NOT NEW.draining AND a#>>'{readiness,generation}'=l#>>'{prepared,container,registration,generation}'
        AND NEW.effective_revision::text IS NOT DISTINCT FROM l#>>'{prepared,configuration_revision}'
        AND a#>>'{previous_stop,observation}'='namespace_exited'
        AND EXISTS(SELECT 1 FROM runtime_container_launches c WHERE c.agent_id=NEW.agent_id AND c.state='running'
            AND c.prepared=l->'prepared' AND c.snapshot=l->'snapshot' AND c.origin=l->>'origin'
            AND c.controller_id::text=l->>'controller_id')) IS TRUE THEN RETURN NEW; END IF;
    RAISE EXCEPTION 'Effective Docker revision requires original physical and readiness proof';
END $$;
CREATE TRIGGER runtime_container_preparation_launch BEFORE INSERT ON runtime_container_launches
    FOR EACH ROW EXECUTE FUNCTION fleet_guard_preparation_launch();
CREATE TRIGGER runtime_container_preparation_config BEFORE INSERT OR UPDATE OR DELETE ON agent_config_heads
    FOR EACH ROW EXECUTE FUNCTION fleet_guard_preparation_config();
"#).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
LOCK TABLE runtime_container_activations IN ACCESS EXCLUSIVE MODE;
DO $$ BEGIN
    IF EXISTS(SELECT 1 FROM runtime_container_activations)
    THEN RAISE EXCEPTION 'Activation custody history prevents downgrade'; END IF;
END $$;
DROP TRIGGER runtime_container_preparation_launch ON runtime_container_launches;
DROP TRIGGER runtime_container_preparation_config ON agent_config_heads;
DROP FUNCTION fleet_guard_preparation_launch();
DROP FUNCTION fleet_guard_preparation_config();
ALTER FUNCTION fleet_guard_preparation_launch_v17() RENAME TO fleet_guard_preparation_launch;
ALTER FUNCTION fleet_guard_preparation_config_v17() RENAME TO fleet_guard_preparation_config;
CREATE TRIGGER runtime_container_preparation_launch BEFORE INSERT ON runtime_container_launches
    FOR EACH ROW EXECUTE FUNCTION fleet_guard_preparation_launch();
CREATE TRIGGER runtime_container_preparation_config BEFORE INSERT OR UPDATE OR DELETE ON agent_config_heads
    FOR EACH ROW EXECUTE FUNCTION fleet_guard_preparation_config();
DROP TABLE runtime_container_activations;
DROP FUNCTION fleet_guard_container_activation();
"#).await?;
        Ok(())
    }
}
