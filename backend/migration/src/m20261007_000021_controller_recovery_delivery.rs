use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE runtime_controller_recovery_deliveries (
                recovery_id uuid PRIMARY KEY REFERENCES runtime_controller_recoveries(id),
                command jsonb NOT NULL CHECK(jsonb_typeof(command)='object' AND octet_length(command::text)<=16384),
                command_sha256 text NOT NULL CHECK(command_sha256 ~ '^[a-f0-9]{64}$'),
                dispatch_claimed boolean NOT NULL DEFAULT false,
                native_receipt jsonb CHECK(jsonb_typeof(native_receipt)='object' AND octet_length(native_receipt::text)<=65536),
                native_receipt_sha256 text CHECK(native_receipt_sha256 ~ '^[a-f0-9]{64}$'),
                CHECK((native_receipt IS NULL)=(native_receipt_sha256 IS NULL)),
                CHECK(native_receipt IS NULL OR dispatch_claimed)
            );
            CREATE FUNCTION fleet_guard_controller_delivery() RETURNS trigger LANGUAGE plpgsql AS $$
            DECLARE prior runtime_controller_recoveries%ROWTYPE;
            BEGIN
                IF TG_OP IN ('DELETE','TRUNCATE') THEN
                    RAISE EXCEPTION 'controller delivery history cannot be removed' USING ERRCODE='23514';
                END IF;
                SELECT id,launch_id,agent_id,controller_id,predecessor_id,epoch,request,request_sha256,
                    state,lease_version,created_at,lease_expires_at,native_receipt_sha256,acknowledged_at
                    INTO prior FROM runtime_controller_recoveries WHERE id=NEW.recovery_id;
                IF NOT FOUND OR NEW.command->'request' IS DISTINCT FROM prior.request
                    OR (NEW.command->>'epoch')::bigint IS DISTINCT FROM prior.epoch
                    OR (NEW.command->>'lease_version')::bigint IS DISTINCT FROM 1::bigint
                    OR NOT (NEW.command ?& ARRAY['request','epoch','lease_version','lease_expires_at'])
                    OR (SELECT count(*) FROM jsonb_object_keys(NEW.command))<>4 THEN
                    RAISE EXCEPTION 'controller delivery requires its exact original command' USING ERRCODE='23514';
                END IF;
                IF TG_OP='INSERT' THEN
                    IF prior.state<>'reserved' OR prior.lease_version<>1
                        OR prior.lease_expires_at<=clock_timestamp()
                        OR (NEW.command->>'lease_expires_at')::timestamptz IS DISTINCT FROM prior.lease_expires_at
                        OR NEW.dispatch_claimed OR NEW.native_receipt IS NOT NULL THEN
                        RAISE EXCEPTION 'controller delivery must precede native dispatch' USING ERRCODE='23514';
                    END IF;
                    RETURN NEW;
                END IF;
                IF NEW.recovery_id<>OLD.recovery_id OR NEW.command IS DISTINCT FROM OLD.command
                    OR NEW.command_sha256<>OLD.command_sha256
                    OR (OLD.dispatch_claimed AND NOT NEW.dispatch_claimed)
                    OR (OLD.native_receipt IS NOT NULL AND NEW IS DISTINCT FROM OLD) THEN
                    RAISE EXCEPTION 'original controller delivery is immutable' USING ERRCODE='23514';
                END IF;
                IF NEW.dispatch_claimed IS DISTINCT FROM OLD.dispatch_claimed THEN
                    IF prior.state<>'reserved' OR prior.lease_expires_at<=clock_timestamp()
                        OR prior.lease_version<>1 OR NEW.native_receipt IS NOT NULL THEN
                        RAISE EXCEPTION 'controller dispatch requires a live original reservation' USING ERRCODE='23514';
                    END IF;
                END IF;
                IF NEW.native_receipt IS DISTINCT FROM OLD.native_receipt THEN
                    IF NOT OLD.dispatch_claimed OR NEW.native_receipt IS NULL
                        OR NEW.native_receipt->'recovery' IS DISTINCT FROM OLD.command
                        OR NEW.native_receipt->>'request_sha256' IS DISTINCT FROM OLD.command_sha256
                        OR NEW.native_receipt->>'state' IS DISTINCT FROM 'controller_recovered'
                        OR NEW.native_receipt->>'contract_version' IS DISTINCT FROM '1'
                        OR jsonb_typeof(NEW.native_receipt->'witness') IS DISTINCT FROM 'object'
                        OR (SELECT count(*) FROM jsonb_object_keys(NEW.native_receipt))<>5 THEN
                        RAISE EXCEPTION 'controller outcome differs from original command' USING ERRCODE='23514';
                    END IF;
                END IF;
                RETURN NEW;
            END $$;
            CREATE TRIGGER fleet_controller_delivery_guard BEFORE INSERT OR UPDATE OR DELETE
                ON runtime_controller_recovery_deliveries FOR EACH ROW EXECUTE FUNCTION fleet_guard_controller_delivery();
            CREATE TRIGGER fleet_controller_delivery_no_truncate BEFORE TRUNCATE
                ON runtime_controller_recovery_deliveries FOR EACH STATEMENT EXECUTE FUNCTION fleet_guard_controller_delivery();
            CREATE OR REPLACE FUNCTION fleet_guard_controller_recovery() RETURNS trigger LANGUAGE plpgsql AS $$
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
                        OR NEW.native_receipt_sha256 IS NOT NULL OR NEW.acknowledged_at IS NOT NULL
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
                        AND NEW.native_receipt_sha256 IS NOT NULL
                        AND (OLD.lease_expires_at>clock_timestamp() OR EXISTS(
                            SELECT 1 FROM runtime_controller_recovery_deliveries d
                            WHERE d.recovery_id=OLD.id AND d.dispatch_claimed AND d.native_receipt IS NOT NULL
                              AND d.native_receipt_sha256=NEW.native_receipt_sha256))
                        OR OLD.state='acknowledged' AND NEW.state='superseded'
                        AND OLD.lease_expires_at<=clock_timestamp()) THEN
                        RAISE EXCEPTION 'controller recovery transition is not fenced' USING ERRCODE='23514';
                    END IF;
                ELSIF NEW.native_receipt_sha256 IS DISTINCT FROM OLD.native_receipt_sha256
                    OR NEW.acknowledged_at IS DISTINCT FROM OLD.acknowledged_at THEN
                    RAISE EXCEPTION 'controller receipt requires acknowledgement' USING ERRCODE='23514';
                END IF;
                RETURN NEW;
            END $$;"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "DO $$ BEGIN
                IF EXISTS(SELECT 1 FROM runtime_controller_recovery_deliveries)
                    OR EXISTS(SELECT 1 FROM runtime_controller_recoveries) THEN
                    RAISE EXCEPTION 'controller recovery history prevents delivery downgrade' USING ERRCODE='23514';
                END IF;
            END $$;
            DROP TABLE runtime_controller_recovery_deliveries; DROP FUNCTION fleet_guard_controller_delivery();"
        ).await?;
        // Both tables are proven empty; restore the exact previous guard and schema.
        super::m20261007_000020_controller_recovery::Migration
            .down(manager)
            .await?;
        super::m20261007_000020_controller_recovery::Migration
            .up(manager)
            .await
    }
}
