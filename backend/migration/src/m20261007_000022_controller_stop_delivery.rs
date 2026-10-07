use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE runtime_controller_stop_deliveries (
                launch_id uuid PRIMARY KEY REFERENCES runtime_launches(id),
                intent jsonb NOT NULL CHECK(jsonb_typeof(intent)='object'
                    AND octet_length(intent::text)<=8192
                    AND (intent->>'launch_id'=launch_id::text
                         AND intent->>'operation_id'=launch_id::text) IS TRUE),
                intent_sha256 text NOT NULL CHECK(intent_sha256 ~ '^[a-f0-9]{64}$'),
                dispatch_command jsonb CHECK(jsonb_typeof(dispatch_command)='object'
                    AND octet_length(dispatch_command::text)<=16384),
                dispatch_command_sha256 text CHECK(dispatch_command_sha256 ~ '^[a-f0-9]{64}$'),
                native_outcome jsonb CHECK(jsonb_typeof(native_outcome)='object'
                    AND octet_length(native_outcome::text)<=65536),
                native_outcome_sha256 text CHECK(native_outcome_sha256 ~ '^[a-f0-9]{64}$'),
                CHECK((dispatch_command IS NULL)=(dispatch_command_sha256 IS NULL)),
                CHECK((native_outcome IS NULL)=(native_outcome_sha256 IS NULL)),
                CHECK(native_outcome IS NULL OR dispatch_command IS NOT NULL)
            );
            CREATE FUNCTION fleet_guard_controller_stop_delivery() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF TG_OP IN ('DELETE','TRUNCATE') THEN
                    RAISE EXCEPTION 'controller stop history cannot be removed' USING ERRCODE='23514';
                END IF;
                PERFORM 1 FROM agents WHERE id=(NEW.intent->>'agent_id')::uuid FOR NO KEY UPDATE;
                IF TG_OP='INSERT' THEN
                    IF NEW.dispatch_command IS NOT NULL OR NEW.native_outcome IS NOT NULL
                        OR NOT EXISTS(SELECT 1 FROM runtime_launches l
                            WHERE l.id=NEW.launch_id AND l.agent_id::text=NEW.intent->>'agent_id'
                              AND l.state='gateway_started' AND l.pid=(NEW.intent->>'pid')::integer) THEN
                        RAISE EXCEPTION 'controller stop requires original started namespace' USING ERRCODE='23514';
                    END IF;
                    RETURN NEW;
                END IF;
                IF (to_jsonb(NEW)-'dispatch_command'-'dispatch_command_sha256'-'native_outcome'-'native_outcome_sha256')
                    IS DISTINCT FROM
                    (to_jsonb(OLD)-'dispatch_command'-'dispatch_command_sha256'-'native_outcome'-'native_outcome_sha256')
                    OR (OLD.dispatch_command IS NOT NULL AND
                        (NEW.dispatch_command IS DISTINCT FROM OLD.dispatch_command
                         OR NEW.dispatch_command_sha256 IS DISTINCT FROM OLD.dispatch_command_sha256))
                    OR (OLD.native_outcome IS NOT NULL AND NEW IS DISTINCT FROM OLD) THEN
                    RAISE EXCEPTION 'original controller stop identity is immutable' USING ERRCODE='23514';
                END IF;
                IF OLD.dispatch_command IS NULL AND NEW.dispatch_command IS NOT NULL THEN
                    IF NEW.native_outcome IS NOT NULL OR NOT EXISTS(
                        SELECT 1 FROM runtime_controller_recoveries e
                        JOIN runtime_launches l ON l.id=e.launch_id
                        JOIN runtime_controller_recovery_deliveries d ON d.recovery_id=e.id
                        WHERE e.launch_id=NEW.launch_id AND e.state='acknowledged'
                          AND e.lease_expires_at>clock_timestamp() AND l.state='gateway_started'
                          AND d.dispatch_claimed AND d.native_receipt IS NOT NULL
                          AND d.native_receipt_sha256=e.native_receipt_sha256
                          AND e.request=NEW.dispatch_command->'request'
                          AND e.epoch=(NEW.dispatch_command->>'epoch')::bigint
                          AND e.lease_version=(NEW.dispatch_command->>'lease_version')::bigint
                          AND e.lease_expires_at=(NEW.dispatch_command->>'lease_expires_at')::timestamptz) THEN
                        RAISE EXCEPTION 'controller stop dispatch requires exact current custody' USING ERRCODE='23514';
                    END IF;
                END IF;
                IF OLD.native_outcome IS NULL AND NEW.native_outcome IS NOT NULL THEN
                    IF OLD.dispatch_command IS NULL OR NOT coalesce(NEW.native_outcome->>'kind' IN ('stop','observe'),false)
                        OR NEW.native_outcome->'receipt'->>'state' IS DISTINCT FROM 'observed'
                        OR NEW.native_outcome->'receipt'->>'observation' IS DISTINCT FROM 'namespace_exited' THEN
                        RAISE EXCEPTION 'controller stop outcome requires confirmed original exit' USING ERRCODE='23514';
                    END IF;
                END IF;
                RETURN NEW;
            END $$;
            CREATE TRIGGER fleet_controller_stop_delivery_guard BEFORE INSERT OR UPDATE OR DELETE
                ON runtime_controller_stop_deliveries FOR EACH ROW EXECUTE FUNCTION fleet_guard_controller_stop_delivery();
            CREATE TRIGGER fleet_controller_stop_delivery_no_truncate BEFORE TRUNCATE
                ON runtime_controller_stop_deliveries FOR EACH STATEMENT EXECUTE FUNCTION fleet_guard_controller_stop_delivery();"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "DO $$ BEGIN
                IF EXISTS(SELECT 1 FROM runtime_controller_stop_deliveries) THEN
                    RAISE EXCEPTION 'controller stop history prevents downgrade' USING ERRCODE='23514';
                END IF;
            END $$;
            DROP TABLE runtime_controller_stop_deliveries;
            DROP FUNCTION fleet_guard_controller_stop_delivery();"
        ).await?;
        Ok(())
    }
}
