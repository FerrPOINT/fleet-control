use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "ALTER TABLE runtime_approval_decisions
                ADD COLUMN outcome_required boolean NOT NULL DEFAULT false,
                ADD COLUMN submission_claimed boolean NOT NULL DEFAULT false;
            CREATE TABLE runtime_approval_outcomes (
                decision_id uuid PRIMARY KEY REFERENCES runtime_approval_decisions(id),
                context jsonb NOT NULL CHECK(jsonb_typeof(context)='object'
                    AND octet_length(context::text)<=262144),
                state text NOT NULL DEFAULT 'submitted' CHECK(state IN ('submitted','acknowledged')),
                created_at timestamptz NOT NULL DEFAULT now(),
                acknowledged_at timestamptz,
                CHECK((state='acknowledged')=(acknowledged_at IS NOT NULL))
            );
            CREATE INDEX runtime_approval_outcomes_pending ON runtime_approval_outcomes(decision_id)
                WHERE state='submitted';
            CREATE OR REPLACE FUNCTION fleet_guard_approval_decision() RETURNS trigger AS $$
            BEGIN
                IF TG_OP='DELETE' THEN
                    IF OLD.outcome_required THEN
                        RAISE EXCEPTION 'original approval decision history cannot be removed';
                    END IF;
                    RETURN OLD;
                END IF;
                IF TG_OP='INSERT' THEN
                    IF NEW.submission_claimed OR (NEW.outcome_required AND
                        (NEW.state<>'uncertain' OR NEW.delivered_at IS NOT NULL)) THEN
                        RAISE EXCEPTION 'original approval must begin without a submission claim';
                    END IF;
                    RETURN NEW;
                END IF;
                IF (to_jsonb(NEW)-'state'-'delivered_at'-'submission_claimed') IS DISTINCT FROM
                   (to_jsonb(OLD)-'state'-'delivered_at'-'submission_claimed')
                   OR (OLD.state IN ('delivered','failed') AND NEW IS DISTINCT FROM OLD)
                   OR (NEW.state IS DISTINCT FROM OLD.state AND NOT
                       (OLD.state='uncertain' AND NEW.state IN ('delivered','failed'))) THEN
                    RAISE EXCEPTION 'approval decision is immutable' USING ERRCODE='23514';
                END IF;
                IF NEW.submission_claimed IS DISTINCT FROM OLD.submission_claimed AND NOT
                    (NOT OLD.submission_claimed AND NEW.submission_claimed
                        AND OLD.outcome_required AND OLD.state='uncertain' AND NEW.state='uncertain') THEN
                    RAISE EXCEPTION 'approval submission claim is immutable' USING ERRCODE='23514';
                END IF;
                IF NEW.outcome_required AND NEW.state='failed' AND NEW.submission_claimed THEN
                    RAISE EXCEPTION 'submitted approval cannot release its uncertain hold' USING ERRCODE='23514';
                END IF;
                IF NEW.outcome_required AND NEW.state='delivered' AND NOT EXISTS(
                    SELECT 1 FROM runtime_approval_outcomes WHERE decision_id=NEW.id AND state='acknowledged') THEN
                    RAISE EXCEPTION 'approval delivery requires its original outcome' USING ERRCODE='23514';
                END IF;
                RETURN NEW;
            END $$ LANGUAGE plpgsql;
            CREATE TRIGGER fleet_original_approval_insert BEFORE INSERT ON runtime_approval_decisions
                FOR EACH ROW EXECUTE FUNCTION fleet_guard_approval_decision();
            CREATE TRIGGER fleet_original_approval_delete BEFORE DELETE ON runtime_approval_decisions
                FOR EACH ROW EXECUTE FUNCTION fleet_guard_approval_decision();
            CREATE FUNCTION guard_runtime_approval_outcome() RETURNS trigger LANGUAGE plpgsql AS $$
            DECLARE d runtime_approval_decisions%ROWTYPE;
                a runtime_approval_requests%ROWTYPE;
                r session_agent_runs%ROWTYPE;
                j hermes_dispatch_journal%ROWTYPE;
                body jsonb;
            BEGIN
                IF TG_OP='DELETE' THEN
                    RAISE EXCEPTION 'approval outcome history cannot be removed';
                END IF;
                SELECT id,session_id,approval_id,session_run_id,choice,state,outcome_required,submission_claimed
                    INTO STRICT d.id,d.session_id,d.approval_id,d.session_run_id,d.choice,d.state,d.outcome_required,d.submission_claimed
                    FROM runtime_approval_decisions WHERE id=NEW.decision_id;
                IF TG_OP='INSERT' THEN
                    IF NOT d.outcome_required OR NOT d.submission_claimed OR d.state<>'uncertain'
                        OR NEW.state<>'submitted' OR NEW.acknowledged_at IS NOT NULL THEN
                        RAISE EXCEPTION 'approval outcome requires an original claim';
                    END IF;
                    SELECT session_id,session_run_id,agent_id,runtime_run_id,runtime_approval_id
                        INTO STRICT a.session_id,a.session_run_id,a.agent_id,a.runtime_run_id,a.runtime_approval_id
                        FROM runtime_approval_requests WHERE id=d.approval_id;
                    SELECT session_id,agent_id,runtime_run_id,runtime_session_id
                        INTO STRICT r.session_id,r.agent_id,r.runtime_run_id,r.runtime_session_id
                        FROM session_agent_runs WHERE id=d.session_run_id;
                    SELECT session_id,agent_id,state,submitted_at,origin,credential_fingerprint
                        INTO STRICT j.session_id,j.agent_id,j.state,j.submitted_at,j.origin,j.credential_fingerprint
                        FROM hermes_dispatch_journal WHERE run_id=d.session_run_id;
                    IF a.session_id<>d.session_id OR a.session_run_id<>d.session_run_id
                        OR r.session_id<>d.session_id OR r.agent_id<>a.agent_id
                        OR r.runtime_run_id IS DISTINCT FROM a.runtime_run_id
                        OR j.session_id<>d.session_id OR j.agent_id<>a.agent_id
                        OR j.state<>'accepted' OR j.submitted_at IS NULL
                        OR r.runtime_run_id IS NULL OR r.runtime_session_id IS NULL
                        OR a.runtime_approval_id IS NULL
                        OR NEW.context->>'command_id' IS DISTINCT FROM d.id::text
                        OR NEW.context->>'operation' IS DISTINCT FROM 'approval'
                        OR NEW.context->>'run_id' IS DISTINCT FROM a.runtime_run_id
                        OR NEW.context->>'origin' IS DISTINCT FROM j.origin
                        OR NEW.context->>'credential_fingerprint' IS DISTINCT FROM j.credential_fingerprint
                        OR jsonb_typeof(NEW.context->'request_body') IS DISTINCT FROM 'string'
                        OR octet_length(NEW.context->>'request_body')>65536
                        OR NEW.context->>'request_sha256' IS DISTINCT FROM
                            encode(sha256(convert_to(NEW.context->>'request_body','UTF8')),'hex')
                        OR NEW.context->'capabilities'->>'contract_version' IS DISTINCT FROM '1'
                        OR NEW.context->'capabilities'->>'profile' IS DISTINCT FROM 'default'
                        OR NEW.context->'capabilities'->>'native_source_revision' IS DISTINCT FROM
                            'bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3'
                        OR COALESCE(NEW.context->'capabilities'->>'store_id','') !~
                            '^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$'
                        OR NEW.context->'capabilities'->>'store_id'='00000000-0000-0000-0000-000000000000'
                        OR COALESCE(NEW.context->'capabilities'->>'scope_fingerprint','') !~ '^[a-f0-9]{64}$' THEN
                        RAISE EXCEPTION 'approval outcome does not match its original context';
                    END IF;
                    body := (NEW.context->>'request_body')::jsonb;
                    IF jsonb_typeof(body) IS DISTINCT FROM 'object'
                        OR jsonb_typeof(body->'request_id') IS DISTINCT FROM 'string'
                        OR body->>'request_id' IS DISTINCT FROM a.runtime_approval_id
                        OR body->>'choice' IS DISTINCT FROM d.choice
                        OR body->'resolve_all' IS DISTINCT FROM 'false'::jsonb
                        OR (body-'request_id'-'choice'-'resolve_all')<>'{}'::jsonb THEN
                        RAISE EXCEPTION 'approval outcome action differs from the decision';
                    END IF;
                ELSE
                    IF (NEW.decision_id,NEW.context,NEW.created_at) IS DISTINCT FROM
                       (OLD.decision_id,OLD.context,OLD.created_at)
                        OR OLD.state<>'submitted' OR NEW.state<>'acknowledged' THEN
                        RAISE EXCEPTION 'approval outcome identity is immutable';
                    END IF;
                END IF;
                RETURN NEW;
            END $$;
            CREATE TRIGGER runtime_approval_outcome_immutable BEFORE INSERT OR UPDATE OR DELETE
                ON runtime_approval_outcomes FOR EACH ROW EXECUTE FUNCTION guard_runtime_approval_outcome();
            CREATE FUNCTION require_runtime_approval_outcome() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF EXISTS(SELECT 1 FROM runtime_approval_decisions WHERE id=NEW.id
                        AND outcome_required AND submission_claimed)
                    AND NOT EXISTS(SELECT 1 FROM runtime_approval_outcomes WHERE decision_id=NEW.id) THEN
                    RAISE EXCEPTION 'approval original context is missing';
                END IF;
                RETURN NULL;
            END $$;
            CREATE CONSTRAINT TRIGGER runtime_approval_outcome_required AFTER INSERT OR UPDATE
                ON runtime_approval_decisions DEFERRABLE INITIALLY DEFERRED
                FOR EACH ROW EXECUTE FUNCTION require_runtime_approval_outcome();
            CREATE FUNCTION require_runtime_approval_outcome_ack() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF EXISTS(SELECT 1 FROM runtime_approval_outcomes WHERE decision_id=NEW.decision_id
                    AND state='acknowledged') AND (
                    NOT EXISTS(SELECT 1 FROM runtime_approval_decisions WHERE id=NEW.decision_id
                        AND outcome_required AND submission_claimed AND state='delivered')
                    OR NOT EXISTS(SELECT 1 FROM audit_log WHERE entity_type='approval_decision'
                        AND entity_id=NEW.decision_id::text AND action='approval.decision_delivered')) THEN
                    RAISE EXCEPTION 'approval outcome ACK requires an atomic delivered receipt and audit';
                END IF;
                RETURN NULL;
            END $$;
            CREATE CONSTRAINT TRIGGER runtime_approval_outcome_atomic_ack AFTER INSERT OR UPDATE
                ON runtime_approval_outcomes DEFERRABLE INITIALLY DEFERRED
                FOR EACH ROW EXECUTE FUNCTION require_runtime_approval_outcome_ack();"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "LOCK TABLE runtime_approval_decisions,runtime_approval_outcomes IN ACCESS EXCLUSIVE MODE;
            DO $$ BEGIN
                IF EXISTS(SELECT 1 FROM runtime_approval_outcomes)
                    OR EXISTS(SELECT 1 FROM runtime_approval_decisions WHERE outcome_required) THEN
                    RAISE EXCEPTION 'nonempty approval outcome history cannot be removed';
                END IF;
            END $$;
            DROP TRIGGER fleet_original_approval_insert ON runtime_approval_decisions;
            DROP TRIGGER fleet_original_approval_delete ON runtime_approval_decisions;
            DROP TRIGGER runtime_approval_outcome_required ON runtime_approval_decisions;
            DROP FUNCTION require_runtime_approval_outcome();
            DROP TABLE runtime_approval_outcomes;
            DROP FUNCTION guard_runtime_approval_outcome();
            DROP FUNCTION require_runtime_approval_outcome_ack();
            ALTER TABLE runtime_approval_decisions DROP COLUMN outcome_required,DROP COLUMN submission_claimed;
            CREATE OR REPLACE FUNCTION fleet_guard_approval_decision() RETURNS trigger AS $$
            BEGIN
                IF (to_jsonb(NEW)-'state'-'delivered_at') IS DISTINCT FROM (to_jsonb(OLD)-'state'-'delivered_at')
                   OR (OLD.state IN ('delivered','failed') AND NEW IS DISTINCT FROM OLD)
                   OR (NEW.state IS DISTINCT FROM OLD.state AND NOT (OLD.state='uncertain' AND NEW.state IN ('delivered','failed'))) THEN
                    RAISE EXCEPTION 'approval decision is immutable' USING ERRCODE='23514';
                END IF;
                RETURN NEW;
            END $$ LANGUAGE plpgsql;"
        ).await?;
        Ok(())
    }
}
