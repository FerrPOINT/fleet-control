use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const CREATION_GUARD: &str = "CREATE OR REPLACE FUNCTION fleet_guard_pm_creation() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE'
       OR NEW.id IS DISTINCT FROM OLD.id
       OR NEW.owner_user_id IS DISTINCT FROM OLD.owner_user_id
       OR NEW.idempotency_key IS DISTINCT FROM OLD.idempotency_key
       OR NEW.created_at IS DISTINCT FROM OLD.created_at
       OR (NEW.operation - 'draft' - 'input' - 'reservation' - 'session_id' - 'credentials')
         IS DISTINCT FROM (OLD.operation - 'draft' - 'input' - 'reservation' - 'session_id' - 'credentials')
       OR (OLD.operation->'draft' <> 'null'::jsonb AND NEW.operation->'draft' IS DISTINCT FROM OLD.operation->'draft')
       OR (OLD.operation->'input' <> 'null'::jsonb AND NEW.operation->'input' IS DISTINCT FROM OLD.operation->'input')
       OR (OLD.operation->'reservation' <> 'null'::jsonb AND NEW.operation->'reservation' IS DISTINCT FROM OLD.operation->'reservation')
       OR (OLD.operation->'session_id' <> 'null'::jsonb AND NEW.operation->'session_id' IS DISTINCT FROM OLD.operation->'session_id') THEN
        RAISE EXCEPTION 'PM creation identity and receipts are immutable' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(CREATION_GUARD).await?;
        db.execute_unprepared(
            "CREATE FUNCTION fleet_guard_pm_credentials() RETURNS trigger AS $$
             DECLARE journal jsonb; intent jsonb; command jsonb; receipt jsonb; assignment jsonb; expected_scope text;
             BEGIN
                journal := NEW.operation->'credentials';
                IF journal IS NULL THEN
                    IF TG_OP = 'UPDATE' AND OLD.operation ? 'credentials' THEN
                        RAISE EXCEPTION 'PM credential journal cannot be removed' USING ERRCODE = '23514';
                    END IF;
                    RETURN NEW;
                END IF;
                intent := journal->'intent'; command := intent->'command'; receipt := journal->'receipt';
                assignment := NEW.operation->'reservation'->'assignment';
                IF jsonb_typeof(journal) IS DISTINCT FROM 'object'
                   OR NOT journal ?& ARRAY['intent','receipt']
                   OR journal - 'intent' - 'receipt' <> '{}'::jsonb
                   OR jsonb_typeof(intent) IS DISTINCT FROM 'object'
                   OR NOT intent ?& ARRAY['command','request_sha256','parent_fingerprint','base_origin','tracker_origin','machine_subject']
                   OR intent - 'command' - 'request_sha256' - 'parent_fingerprint' - 'base_origin' - 'tracker_origin' - 'machine_subject' <> '{}'::jsonb
                   OR NEW.operation->'session_id' IS NULL OR NEW.operation->'session_id' = 'null'::jsonb
                   OR jsonb_typeof(assignment) IS DISTINCT FROM 'object'
                   OR jsonb_typeof(command) IS DISTINCT FROM 'object'
                   OR jsonb_typeof(command->'expires_in_seconds') IS DISTINCT FROM 'number'
                   OR COALESCE(command->>'expires_in_seconds','') !~ '^[1-9][0-9]{0,3}$'
                   OR (command->>'expires_in_seconds')::integer NOT BETWEEN 1 AND 1800
                   OR jsonb_typeof(intent->'request_sha256') IS DISTINCT FROM 'string'
                   OR jsonb_typeof(intent->'parent_fingerprint') IS DISTINCT FROM 'string'
                   OR jsonb_typeof(intent->'base_origin') IS DISTINCT FROM 'string'
                   OR jsonb_typeof(intent->'tracker_origin') IS DISTINCT FROM 'string'
                   OR jsonb_typeof(intent->'machine_subject') IS DISTINCT FROM 'string'
                   OR COALESCE(intent->>'request_sha256','') !~ '^[0-9a-f]{64}$'
                   OR COALESCE(intent->>'parent_fingerprint','') !~ '^[0-9a-f]{64}$'
                   OR COALESCE(intent->>'base_origin','') = '' OR COALESCE(intent->>'tracker_origin','') = ''
                   OR COALESCE(intent->>'machine_subject','') !~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                   OR intent->>'machine_subject' = '00000000-0000-0000-0000-000000000000'
                   OR intent->>'machine_subject' IS DISTINCT FROM assignment->>'machine_subject' THEN
                    RAISE EXCEPTION 'Invalid PM credential intent' USING ERRCODE = '23514';
                END IF;
                expected_scope := 'task-tracker:sdlc:pm:' || (NEW.operation->'draft'->>'task_id') || ':'
                    || (assignment->>'assignment_id') || ':' || (assignment->>'execution_id') || ':'
                    || (assignment->>'agent_id') || ':' || (assignment->>'version');
                IF command IS DISTINCT FROM jsonb_build_object(
                    'service','task-tracker','label','PM assignment ' || (assignment->>'assignment_id'),
                    'scopes',jsonb_build_array('task-tracker:read',expected_scope,'task-tracker:write'),
                    'idempotency_key','fleet-pm-credential:' || NEW.id::text,
                    'expires_in_seconds',(command->>'expires_in_seconds')::integer) THEN
                    RAISE EXCEPTION 'PM credential command differs from its assignment' USING ERRCODE = '23514';
                END IF;
                IF receipt IS DISTINCT FROM 'null'::jsonb THEN
                    IF jsonb_typeof(receipt) IS DISTINCT FROM 'object'
                       OR NOT receipt ?& ARRAY['token_id','expires_at','scopes']
                       OR receipt - 'token_id' - 'expires_at' - 'scopes' <> '{}'::jsonb
                       OR jsonb_typeof(receipt->'token_id') IS DISTINCT FROM 'string'
                       OR COALESCE(receipt->>'token_id','') !~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                       OR receipt->>'token_id' = '00000000-0000-0000-0000-000000000000'
                       OR jsonb_typeof(receipt->'expires_at') IS DISTINCT FROM 'string'
                       OR COALESCE(receipt->>'expires_at','') !~ '^[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])T([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]([.][0-9]{1,9})?(Z|[+-]([01][0-9]|2[0-3]):[0-5][0-9])$'
                       OR receipt->'scopes' IS DISTINCT FROM command->'scopes' THEN
                        RAISE EXCEPTION 'Invalid PM credential receipt' USING ERRCODE = '23514';
                    END IF;
                    BEGIN
                        PERFORM (receipt->>'expires_at')::timestamptz;
                    EXCEPTION WHEN invalid_datetime_format OR datetime_field_overflow THEN
                        RAISE EXCEPTION 'Invalid PM credential expiry' USING ERRCODE = '23514';
                    END;
                END IF;
                IF TG_OP = 'UPDATE' AND OLD.operation ? 'credentials' THEN
                    IF intent IS DISTINCT FROM OLD.operation->'credentials'->'intent'
                       OR (OLD.operation->'credentials'->'receipt' <> 'null'::jsonb
                           AND receipt IS DISTINCT FROM OLD.operation->'credentials'->'receipt') THEN
                        RAISE EXCEPTION 'PM credential intent and acknowledgement are immutable' USING ERRCODE = '23514';
                    END IF;
                END IF;
                RETURN NEW;
             END;
             $$ LANGUAGE plpgsql;
             CREATE TRIGGER fleet_pm_credentials_guard BEFORE INSERT OR UPDATE ON pm_draft_creation_operations
                FOR EACH ROW EXECUTE FUNCTION fleet_guard_pm_credentials();",
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // Do not discard recovery material or restore a schema that cannot read live journals.
        db.execute_unprepared(
            "LOCK TABLE pm_draft_creation_operations IN ACCESS EXCLUSIVE MODE;
             DO $$ BEGIN
                IF EXISTS (SELECT 1 FROM pm_draft_creation_operations WHERE operation ? 'credentials') THEN
                    RAISE EXCEPTION 'PM credential journals require explicit reconciliation before downgrade' USING ERRCODE = '23514';
                END IF;
             END $$;
             DROP TRIGGER fleet_pm_credentials_guard ON pm_draft_creation_operations;
             DROP FUNCTION fleet_guard_pm_credentials();",
        ).await?;
        db.execute_unprepared(&CREATION_GUARD.replace(" - 'credentials'", ""))
            .await?;
        Ok(())
    }
}
