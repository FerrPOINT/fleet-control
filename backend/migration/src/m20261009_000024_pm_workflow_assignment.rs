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
       OR (NEW.operation - 'draft' - 'input' - 'reservation' - 'session_id' - 'credentials' - 'execution_lease' - 'workflow_assignment')
         IS DISTINCT FROM (OLD.operation - 'draft' - 'input' - 'reservation' - 'session_id' - 'credentials' - 'execution_lease' - 'workflow_assignment')
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
            "CREATE FUNCTION fleet_guard_pm_workflow_assignment() RETURNS trigger AS $$
             DECLARE journal jsonb; intent jsonb; command jsonb; compatibility jsonb; receipt jsonb; reservation jsonb; item text;
             BEGIN
                journal := NEW.operation->'workflow_assignment';
                IF journal IS NULL THEN
                    IF TG_OP = 'UPDATE' AND OLD.operation ? 'workflow_assignment' THEN
                        RAISE EXCEPTION 'PM Workflow journal cannot be removed' USING ERRCODE = '23514';
                    END IF;
                    RETURN NEW;
                END IF;
                intent := journal->'intent'; command := intent->'command'; receipt := journal->'receipt';
                compatibility := command->'runtime_compatibility'; reservation := NEW.operation->'reservation';
                IF jsonb_typeof(journal) IS DISTINCT FROM 'object'
                   OR NOT journal ?& ARRAY['intent','receipt'] OR journal - 'intent' - 'receipt' <> '{}'::jsonb
                   OR jsonb_typeof(intent) IS DISTINCT FROM 'object'
                   OR NOT intent ?& ARRAY['command','request_sha256','workflow_origin','credential_fingerprint']
                   OR intent - 'command' - 'request_sha256' - 'workflow_origin' - 'credential_fingerprint' <> '{}'::jsonb
                   OR jsonb_typeof(intent->'request_sha256') IS DISTINCT FROM 'string'
                   OR jsonb_typeof(intent->'credential_fingerprint') IS DISTINCT FROM 'string'
                   OR COALESCE(intent->>'request_sha256','') !~ '^[0-9a-f]{64}$'
                   OR COALESCE(intent->>'credential_fingerprint','') !~ '^[0-9a-f]{64}$'
                   OR jsonb_typeof(intent->'workflow_origin') IS DISTINCT FROM 'string'
                   OR COALESCE(length(intent->>'workflow_origin'),0) NOT BETWEEN 1 AND 1024
                   OR NEW.operation->'session_id' IS NULL OR NEW.operation->'session_id' = 'null'::jsonb
                   OR jsonb_typeof(NEW.operation->'execution_lease'->'receipt') IS DISTINCT FROM 'object'
                   OR jsonb_typeof(reservation) IS DISTINCT FROM 'object' THEN
                    RAISE EXCEPTION 'Invalid PM Workflow intent' USING ERRCODE = '23514';
                END IF;
                IF jsonb_typeof(compatibility) IS DISTINCT FROM 'object'
                   OR NOT compatibility ?& ARRAY['catalogVersion','catalogRevision','catalogSha256','skillsRevision','skillsManifestSha256','capabilityRevision','capabilitySha256']
                   OR compatibility - 'catalogVersion' - 'catalogRevision' - 'catalogSha256' - 'skillsRevision' - 'skillsManifestSha256' - 'capabilityRevision' - 'capabilitySha256' <> '{}'::jsonb
                   OR jsonb_typeof(compatibility->'catalogVersion') IS DISTINCT FROM 'number'
                   OR compatibility->>'catalogVersion' IS DISTINCT FROM '2'
                   OR compatibility->>'capabilityRevision' IS DISTINCT FROM 'hermes-sdlc-runtime/v2' THEN
                    RAISE EXCEPTION 'Invalid PM Workflow compatibility' USING ERRCODE = '23514';
                END IF;
                FOREACH item IN ARRAY ARRAY['catalogRevision','skillsRevision'] LOOP
                    IF jsonb_typeof(compatibility->item) IS DISTINCT FROM 'string'
                       OR COALESCE(compatibility->>item,'') !~ '^[0-9a-f]{40}$' THEN
                        RAISE EXCEPTION 'Invalid PM Workflow source pin' USING ERRCODE = '23514';
                    END IF;
                END LOOP;
                FOREACH item IN ARRAY ARRAY['catalogSha256','skillsManifestSha256','capabilitySha256'] LOOP
                    IF jsonb_typeof(compatibility->item) IS DISTINCT FROM 'string'
                       OR COALESCE(compatibility->>item,'') !~ '^[0-9a-f]{64}$' THEN
                        RAISE EXCEPTION 'Invalid PM Workflow source hash' USING ERRCODE = '23514';
                    END IF;
                END LOOP;
                IF command IS DISTINCT FROM jsonb_build_object(
                    'task',reservation->'execution'->'key','execution_ref',reservation->'assignment'->'execution_id',
                    'tracker_instance_ref',reservation->'binding'->'tracker_instance_id',
                    'tracker_project_ref',reservation->'binding'->'project_id',
                    'task_ref',reservation->'binding'->'task_id','root_ref',reservation->'binding'->'root_task_id',
                    'agent_ref',reservation->'assignment'->'agent_id',
                    'assignment_operation_key',reservation->'assignment_operation_key',
                    'assignment_ref',reservation->'assignment'->'assignment_id',
                    'assignment_revision',reservation->'assignment'->'version',
                    'owner_version',reservation->'owner_cas'->'version',
                    'input_snapshot_ref',reservation->'input'->'snapshot_ref',
                    'input_sha256',reservation->'input'->'sha256','runtime_compatibility',compatibility)
                   OR command->>'assignment_revision' IS DISTINCT FROM '1'
                   OR command->>'owner_version' IS DISTINCT FROM '1'
                   OR command->'root_ref' IS DISTINCT FROM command->'task_ref' THEN
                    RAISE EXCEPTION 'PM Workflow command differs from reservation' USING ERRCODE = '23514';
                END IF;
                IF receipt IS DISTINCT FROM 'null'::jsonb THEN
                    IF jsonb_typeof(receipt) IS DISTINCT FROM 'object'
                       OR NOT receipt ?& ARRAY['workflow_id','mode_id','phase_id','phase_code']
                       OR receipt - 'workflow_id' - 'mode_id' - 'phase_id' - 'phase_code' <> '{}'::jsonb
                       OR jsonb_typeof(receipt->'phase_code') IS DISTINCT FROM 'string'
                       OR receipt->>'phase_code' IS DISTINCT FROM 'PM-DRAFT-01'
                       OR COALESCE(length(receipt->>'phase_code'),0) NOT BETWEEN 1 AND 128
                       OR receipt->>'phase_code' ~ '[[:space:][:cntrl:]]' THEN
                        RAISE EXCEPTION 'Invalid PM Workflow receipt' USING ERRCODE = '23514';
                    END IF;
                    FOREACH item IN ARRAY ARRAY['workflow_id','mode_id','phase_id'] LOOP
                        IF jsonb_typeof(receipt->item) IS DISTINCT FROM 'number'
                           OR COALESCE(receipt->>item,'') !~ '^[1-9][0-9]{0,18}$'
                           OR (receipt->>item)::numeric > 9223372036854775807 THEN
                            RAISE EXCEPTION 'Invalid PM Workflow receipt ID' USING ERRCODE = '23514';
                        END IF;
                    END LOOP;
                END IF;
                IF TG_OP = 'UPDATE' AND OLD.operation ? 'workflow_assignment' THEN
                    IF intent IS DISTINCT FROM OLD.operation->'workflow_assignment'->'intent'
                       OR (OLD.operation->'workflow_assignment'->'receipt' <> 'null'::jsonb
                           AND receipt IS DISTINCT FROM OLD.operation->'workflow_assignment'->'receipt') THEN
                        RAISE EXCEPTION 'PM Workflow intent and acknowledgement are immutable' USING ERRCODE = '23514';
                    END IF;
                END IF;
                RETURN NEW;
             END;
             $$ LANGUAGE plpgsql;
             CREATE TRIGGER fleet_pm_workflow_assignment_guard BEFORE INSERT OR UPDATE ON pm_draft_creation_operations
                FOR EACH ROW EXECUTE FUNCTION fleet_guard_pm_workflow_assignment();",
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "LOCK TABLE pm_draft_creation_operations IN ACCESS EXCLUSIVE MODE;
             DO $$ BEGIN
                IF EXISTS (SELECT 1 FROM pm_draft_creation_operations WHERE operation ? 'workflow_assignment') THEN
                    RAISE EXCEPTION 'PM Workflow journals require explicit reconciliation before downgrade' USING ERRCODE = '23514';
                END IF;
             END $$;
             DROP TRIGGER fleet_pm_workflow_assignment_guard ON pm_draft_creation_operations;
             DROP FUNCTION fleet_guard_pm_workflow_assignment();",
        ).await?;
        db.execute_unprepared(&CREATION_GUARD.replace(" - 'workflow_assignment'", ""))
            .await?;
        Ok(())
    }
}
