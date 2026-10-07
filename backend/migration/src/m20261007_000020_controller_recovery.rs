use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE runtime_controller_recoveries (
                id uuid PRIMARY KEY,
                launch_id uuid NOT NULL REFERENCES runtime_launches(id),
                agent_id uuid NOT NULL REFERENCES agents(id),
                controller_id uuid NOT NULL,
                predecessor_id uuid REFERENCES runtime_controller_recoveries(id),
                epoch bigint NOT NULL CHECK(epoch>0),
                request jsonb NOT NULL CHECK(jsonb_typeof(request)='object'
                    AND octet_length(request::text)<=8192
                    AND request ?& ARRAY['id','launch_id','agent_id','controller_id','predecessor_id']
                    AND (request->>'id'=id::text AND request->>'launch_id'=launch_id::text
                    AND request->>'agent_id'=agent_id::text AND request->>'controller_id'=controller_id::text
                    AND request->>'predecessor_id' IS NOT DISTINCT FROM predecessor_id::text) IS TRUE),
                request_sha256 text NOT NULL CHECK(request_sha256 ~ '^[a-f0-9]{64}$'),
                state text NOT NULL DEFAULT 'reserved'
                    CHECK(state IN ('reserved','acknowledged','superseded')),
                lease_version bigint NOT NULL DEFAULT 1 CHECK(lease_version>0),
                created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
                lease_expires_at timestamptz NOT NULL,
                native_receipt_sha256 text CHECK(native_receipt_sha256 ~ '^[a-f0-9]{64}$'),
                acknowledged_at timestamptz,
                CHECK((native_receipt_sha256 IS NOT NULL)=(acknowledged_at IS NOT NULL)),
                CHECK((state='reserved')=(native_receipt_sha256 IS NULL)),
                CHECK(lease_expires_at>created_at),
                UNIQUE(launch_id,epoch)
            );
            CREATE UNIQUE INDEX runtime_controller_recoveries_current
                ON runtime_controller_recoveries(launch_id) WHERE state<>'superseded';
            CREATE FUNCTION fleet_guard_controller_recovery() RETURNS trigger LANGUAGE plpgsql AS $$
            DECLARE prior runtime_controller_recoveries%ROWTYPE;
            BEGIN
                IF TG_OP IN ('DELETE','TRUNCATE') THEN
                    RAISE EXCEPTION 'controller recovery history cannot be removed' USING ERRCODE='23514';
                END IF;
                IF TG_OP='INSERT' THEN
                    IF NOT EXISTS(SELECT 1 FROM runtime_launches l WHERE l.id=NEW.launch_id
                        AND l.agent_id=NEW.agent_id AND l.state='gateway_started'
                        AND l.controller_id<>NEW.controller_id) THEN
                        RAISE EXCEPTION 'controller recovery requires its original started launch' USING ERRCODE='23514';
                    END IF;
                    IF NEW.state<>'reserved' OR NEW.lease_version<>1
                        OR NEW.native_receipt_sha256 IS NOT NULL
                        OR NEW.acknowledged_at IS NOT NULL
                        OR NEW.lease_expires_at>clock_timestamp()+interval '30 seconds'
                        OR NEW.lease_expires_at<=clock_timestamp() THEN
                        RAISE EXCEPTION 'controller recovery must begin with a bounded reservation' USING ERRCODE='23514';
                    END IF;
                    IF NEW.predecessor_id IS NULL THEN
                        IF NEW.epoch<>1 OR EXISTS(SELECT 1 FROM runtime_controller_recoveries WHERE launch_id=NEW.launch_id) THEN
                            RAISE EXCEPTION 'controller epoch predecessor is missing' USING ERRCODE='23514';
                        END IF;
                    ELSE
                        SELECT id,launch_id,agent_id,controller_id,predecessor_id,epoch,request,request_sha256,
                            state,lease_version,created_at,lease_expires_at,native_receipt_sha256,acknowledged_at
                            INTO prior FROM runtime_controller_recoveries WHERE id=NEW.predecessor_id;
                        IF NOT FOUND OR prior.launch_id<>NEW.launch_id OR prior.agent_id<>NEW.agent_id
                            OR prior.controller_id=NEW.controller_id OR prior.state<>'superseded'
                            OR NEW.epoch<>prior.epoch+1 THEN
                            RAISE EXCEPTION 'controller epoch predecessor changed' USING ERRCODE='23514';
                        END IF;
                    END IF;
                    RETURN NEW;
                END IF;
                IF (to_jsonb(NEW)-'state'-'lease_version'-'lease_expires_at'-'native_receipt_sha256'-'acknowledged_at')
                    IS DISTINCT FROM
                    (to_jsonb(OLD)-'state'-'lease_version'-'lease_expires_at'-'native_receipt_sha256'-'acknowledged_at')
                    OR (OLD.state='superseded' AND NEW IS DISTINCT FROM OLD)
                    OR (OLD.native_receipt_sha256 IS NOT NULL AND
                        (NEW.native_receipt_sha256 IS DISTINCT FROM OLD.native_receipt_sha256
                        OR NEW.acknowledged_at IS DISTINCT FROM OLD.acknowledged_at)) THEN
                    RAISE EXCEPTION 'original controller recovery identity is immutable' USING ERRCODE='23514';
                END IF;
                IF NEW.lease_version IS DISTINCT FROM OLD.lease_version
                    OR NEW.lease_expires_at IS DISTINCT FROM OLD.lease_expires_at THEN
                    IF NEW.state<>OLD.state OR OLD.state='superseded'
                        OR OLD.lease_expires_at<=clock_timestamp()
                        OR NEW.lease_version<>OLD.lease_version+1
                        OR NEW.lease_expires_at<=OLD.lease_expires_at
                        OR NEW.lease_expires_at>clock_timestamp()+interval '30 seconds'
                        OR NEW.native_receipt_sha256 IS DISTINCT FROM OLD.native_receipt_sha256
                        OR NEW.acknowledged_at IS DISTINCT FROM OLD.acknowledged_at THEN
                        RAISE EXCEPTION 'controller heartbeat is stale or unbounded' USING ERRCODE='23514';
                    END IF;
                END IF;
                IF NEW.state IS DISTINCT FROM OLD.state THEN
                    IF NOT (OLD.state='reserved' AND NEW.state='acknowledged'
                        AND OLD.lease_expires_at>clock_timestamp() AND NEW.native_receipt_sha256 IS NOT NULL
                        OR OLD.state='acknowledged' AND NEW.state='superseded'
                        AND OLD.lease_expires_at<=clock_timestamp()) THEN
                        RAISE EXCEPTION 'controller recovery transition is not fenced' USING ERRCODE='23514';
                    END IF;
                ELSIF NEW.native_receipt_sha256 IS DISTINCT FROM OLD.native_receipt_sha256
                    OR NEW.acknowledged_at IS DISTINCT FROM OLD.acknowledged_at THEN
                    RAISE EXCEPTION 'controller receipt requires acknowledgement' USING ERRCODE='23514';
                END IF;
                RETURN NEW;
            END $$;
            CREATE TRIGGER fleet_controller_recovery_guard BEFORE INSERT OR UPDATE OR DELETE
                ON runtime_controller_recoveries FOR EACH ROW EXECUTE FUNCTION fleet_guard_controller_recovery();
            CREATE TRIGGER fleet_controller_recovery_no_truncate BEFORE TRUNCATE
                ON runtime_controller_recoveries FOR EACH STATEMENT EXECUTE FUNCTION fleet_guard_controller_recovery();"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "DO $$ BEGIN
                IF EXISTS(SELECT 1 FROM runtime_controller_recoveries) THEN
                    RAISE EXCEPTION 'controller recovery history prevents downgrade' USING ERRCODE='23514';
                END IF;
            END $$;
            DROP TABLE runtime_controller_recoveries; DROP FUNCTION fleet_guard_controller_recovery();"
        ).await?;
        Ok(())
    }
}
