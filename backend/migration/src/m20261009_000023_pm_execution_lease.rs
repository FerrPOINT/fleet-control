use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Keep the preceding release bytes unchanged; only the new guarded journal may progress.
const CREATION_GUARD: &str = "CREATE OR REPLACE FUNCTION fleet_guard_pm_creation() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE'
       OR NEW.id IS DISTINCT FROM OLD.id
       OR NEW.owner_user_id IS DISTINCT FROM OLD.owner_user_id
       OR NEW.idempotency_key IS DISTINCT FROM OLD.idempotency_key
       OR NEW.created_at IS DISTINCT FROM OLD.created_at
       OR (NEW.operation - 'draft' - 'input' - 'reservation' - 'session_id' - 'credentials' - 'execution_lease')
         IS DISTINCT FROM (OLD.operation - 'draft' - 'input' - 'reservation' - 'session_id' - 'credentials' - 'execution_lease')
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
            "CREATE FUNCTION fleet_guard_pm_execution_lease() RETURNS trigger AS $$
             DECLARE journal jsonb; claim jsonb; receipt jsonb; reservation jsonb; fence jsonb; lease jsonb;
             BEGIN
                journal := NEW.operation->'execution_lease';
                IF journal IS NULL THEN
                    IF TG_OP = 'UPDATE' AND OLD.operation ? 'execution_lease' THEN
                        RAISE EXCEPTION 'PM lease journal cannot be removed' USING ERRCODE = '23514';
                    END IF;
                    RETURN NEW;
                END IF;
                claim := journal->'claim'; receipt := journal->'receipt';
                reservation := NEW.operation->'reservation';
                fence := jsonb_build_object(
                    'assignment_id',reservation->'assignment'->'assignment_id',
                    'execution_id',reservation->'assignment'->'execution_id',
                    'agent_id',reservation->'assignment'->'agent_id',
                    'assignment_version',reservation->'assignment'->'version');
                IF jsonb_typeof(journal) IS DISTINCT FROM 'object'
                   OR NOT journal ?& ARRAY['claim','request_sha256','receipt']
                   OR journal - 'claim' - 'request_sha256' - 'receipt' <> '{}'::jsonb
                   OR jsonb_typeof(journal->'request_sha256') IS DISTINCT FROM 'string'
                   OR COALESCE(journal->>'request_sha256','') !~ '^[0-9a-f]{64}$'
                   OR NEW.operation->'session_id' IS NULL OR NEW.operation->'session_id' = 'null'::jsonb
                   OR jsonb_typeof(reservation) IS DISTINCT FROM 'object'
                   OR jsonb_typeof(NEW.operation->'credentials'->'receipt') IS DISTINCT FROM 'object'
                   OR claim IS DISTINCT FROM jsonb_build_object(
                       'expected_owner_version',reservation->'owner_cas'->'version',
                       'fence',fence,'idempotency_key','fleet-pm-lease:' || NEW.id::text) THEN
                    RAISE EXCEPTION 'Invalid PM lease claim journal' USING ERRCODE = '23514';
                END IF;
                IF receipt IS DISTINCT FROM 'null'::jsonb THEN
                    lease := receipt->'lease';
                    IF receipt IS DISTINCT FROM jsonb_build_object(
                        'contract_version',1,'binding',reservation->'binding',
                        'owner_version',reservation->'owner_cas'->'version','fence',fence,
                        'lease',lease,'ttl_seconds',30,'heartbeat_seconds',10,'dispatch_allowed',false)
                       OR jsonb_typeof(lease) IS DISTINCT FROM 'object'
                       OR NOT lease ?& ARRAY['lease_id','version','holder_subject','claimed_at','heartbeat_at','expires_at']
                       OR lease - 'lease_id' - 'version' - 'holder_subject' - 'claimed_at' - 'heartbeat_at' - 'expires_at' <> '{}'::jsonb
                       OR jsonb_typeof(lease->'lease_id') IS DISTINCT FROM 'string'
                       OR COALESCE(lease->>'lease_id','') !~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                       OR lease->>'lease_id' = '00000000-0000-0000-0000-000000000000'
                       OR lease->'version' IS DISTINCT FROM '1'::jsonb
                       OR lease->'holder_subject' IS DISTINCT FROM reservation->'assignment'->'machine_subject'
                       OR lease->'claimed_at' IS DISTINCT FROM lease->'heartbeat_at'
                       OR jsonb_typeof(lease->'claimed_at') IS DISTINCT FROM 'string'
                       OR jsonb_typeof(lease->'expires_at') IS DISTINCT FROM 'string'
                       OR COALESCE(lease->>'claimed_at','') !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}[.][0-9]{9}Z$'
                       OR COALESCE(lease->>'expires_at','') !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}[.][0-9]{9}Z$' THEN
                        RAISE EXCEPTION 'Invalid PM lease acknowledgement' USING ERRCODE = '23514';
                    END IF;
                    BEGIN
                        IF (lease->>'expires_at')::timestamptz IS DISTINCT FROM
                            (lease->>'claimed_at')::timestamptz + interval '30 seconds' THEN
                            RAISE EXCEPTION 'Invalid PM lease expiry' USING ERRCODE = '23514';
                        END IF;
                    EXCEPTION WHEN invalid_datetime_format OR datetime_field_overflow THEN
                        RAISE EXCEPTION 'Invalid PM lease timestamps' USING ERRCODE = '23514';
                    END;
                END IF;
                IF TG_OP = 'UPDATE' AND OLD.operation ? 'execution_lease' THEN
                    IF claim IS DISTINCT FROM OLD.operation->'execution_lease'->'claim'
                       OR journal->'request_sha256' IS DISTINCT FROM OLD.operation->'execution_lease'->'request_sha256'
                       OR (OLD.operation->'execution_lease'->'receipt' <> 'null'::jsonb
                           AND receipt IS DISTINCT FROM OLD.operation->'execution_lease'->'receipt') THEN
                        RAISE EXCEPTION 'PM lease claim and acknowledgement are immutable' USING ERRCODE = '23514';
                    END IF;
                END IF;
                RETURN NEW;
             END;
             $$ LANGUAGE plpgsql;
             CREATE TRIGGER fleet_pm_execution_lease_guard BEFORE INSERT OR UPDATE ON pm_draft_creation_operations
                FOR EACH ROW EXECUTE FUNCTION fleet_guard_pm_execution_lease();",
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "LOCK TABLE pm_draft_creation_operations IN ACCESS EXCLUSIVE MODE;
             DO $$ BEGIN
                IF EXISTS (SELECT 1 FROM pm_draft_creation_operations WHERE operation ? 'execution_lease') THEN
                    RAISE EXCEPTION 'PM lease journals require explicit reconciliation before downgrade' USING ERRCODE = '23514';
                END IF;
             END $$;
             DROP TRIGGER fleet_pm_execution_lease_guard ON pm_draft_creation_operations;
             DROP FUNCTION fleet_guard_pm_execution_lease();",
        ).await?;
        db.execute_unprepared(&CREATION_GUARD.replace(" - 'execution_lease'", ""))
            .await?;
        Ok(())
    }
}
