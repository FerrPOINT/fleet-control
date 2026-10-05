use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "ALTER TABLE runtime_control_commands ADD COLUMN outcome_required boolean NOT NULL DEFAULT false;
            CREATE TABLE runtime_control_outcomes (
                command_id uuid PRIMARY KEY REFERENCES runtime_control_commands(id),
                context jsonb NOT NULL CHECK(jsonb_typeof(context)='object'
                    AND octet_length(context::text)<=262144),
                state text NOT NULL DEFAULT 'submitted' CHECK(state IN ('submitted','acknowledged')),
                acknowledgement text CHECK(acknowledgement IN ('steered','stopping')),
                created_at timestamptz NOT NULL DEFAULT now(),
                acknowledged_at timestamptz,
                CHECK((state='acknowledged')=(acknowledgement IS NOT NULL)),
                CHECK((state='acknowledged')=(acknowledged_at IS NOT NULL))
            );
            CREATE INDEX runtime_control_outcomes_pending ON runtime_control_outcomes(command_id)
                WHERE state='submitted';
            CREATE FUNCTION guard_runtime_control_outcome() RETURNS trigger LANGUAGE plpgsql AS $$
            DECLARE c runtime_control_commands%ROWTYPE;
            BEGIN
                IF TG_OP='DELETE' THEN
                    RAISE EXCEPTION 'runtime control outcome history cannot be removed';
                END IF;
                SELECT id,state,outcome_required,runtime_run_id,operation,api_origin,credential_fingerprint
                    INTO STRICT c.id,c.state,c.outcome_required,c.runtime_run_id,c.operation,c.api_origin,c.credential_fingerprint
                    FROM runtime_control_commands WHERE id=NEW.command_id;
                IF TG_OP='INSERT' THEN
                    IF c.state <> 'submitted' OR NOT c.outcome_required
                        OR NEW.state <> 'submitted' OR NEW.acknowledgement IS NOT NULL
                        OR NEW.acknowledged_at IS NOT NULL THEN
                        RAISE EXCEPTION 'runtime control outcome requires an original claim';
                    END IF;
                    IF NEW.context->>'command_id' IS DISTINCT FROM c.id::text
                        OR NEW.context->>'run_id' IS DISTINCT FROM c.runtime_run_id
                        OR NEW.context->>'operation' IS DISTINCT FROM c.operation
                        OR NEW.context->>'origin' IS DISTINCT FROM c.api_origin
                        OR NEW.context->>'credential_fingerprint' IS DISTINCT FROM c.credential_fingerprint
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
                        OR NEW.context->'capabilities'->>'store_id' = '00000000-0000-0000-0000-000000000000'
                        OR COALESCE(NEW.context->'capabilities'->>'scope_fingerprint','') !~ '^[a-f0-9]{64}$'
                        OR (c.operation='stop' AND NEW.context->>'request_body' <> '') THEN
                        RAISE EXCEPTION 'runtime control outcome context does not match its command';
                    END IF;
                ELSE
                    IF (NEW.command_id,NEW.context,NEW.created_at) IS DISTINCT FROM
                       (OLD.command_id,OLD.context,OLD.created_at)
                        OR OLD.state <> 'submitted' OR NEW.state <> 'acknowledged' THEN
                        RAISE EXCEPTION 'runtime control outcome identity is immutable';
                    END IF;
                    IF NOT ((c.operation='steer' AND NEW.acknowledgement='steered')
                        OR (c.operation='stop' AND NEW.acknowledgement='stopping')) THEN
                        RAISE EXCEPTION 'runtime control outcome ACK does not match';
                    END IF;
                END IF;
                RETURN NEW;
            END $$;
            CREATE TRIGGER runtime_control_outcome_immutable BEFORE INSERT OR UPDATE OR DELETE
                ON runtime_control_outcomes FOR EACH ROW EXECUTE FUNCTION guard_runtime_control_outcome();
            CREATE OR REPLACE FUNCTION guard_runtime_control_command() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF (NEW.id,NEW.session_id,NEW.session_run_id,NEW.agent_id,NEW.actor_user_id,
                    NEW.operation,NEW.idempotency_key,NEW.payload_sha256,NEW.runtime_run_id,
                    NEW.runtime_session_id,NEW.original_request_sha256,NEW.api_origin,
                    NEW.credential_fingerprint,NEW.created_at)
                  IS DISTINCT FROM
                   (OLD.id,OLD.session_id,OLD.session_run_id,OLD.agent_id,OLD.actor_user_id,
                    OLD.operation,OLD.idempotency_key,OLD.payload_sha256,OLD.runtime_run_id,
                    OLD.runtime_session_id,OLD.original_request_sha256,OLD.api_origin,
                    OLD.credential_fingerprint,OLD.created_at) THEN
                    RAISE EXCEPTION 'runtime control identity is immutable';
                END IF;
                IF NEW.outcome_required IS DISTINCT FROM OLD.outcome_required
                    AND NOT (OLD.state='reserved' AND NEW.state='submitted'
                        AND NOT OLD.outcome_required AND NEW.outcome_required) THEN
                    RAISE EXCEPTION 'runtime control outcome mode is immutable after claim';
                END IF;
                IF NEW.state='acknowledged' AND NEW.outcome_required AND NOT EXISTS(
                    SELECT 1 FROM runtime_control_outcomes o WHERE o.command_id=NEW.id
                        AND o.state='acknowledged' AND o.acknowledgement=NEW.acknowledgement) THEN
                    RAISE EXCEPTION 'runtime control ACK requires its original outcome';
                END IF;
                IF NOT ((OLD.state='reserved' AND NEW.state IN ('submitted','rejected'))
                    OR (OLD.state='submitted' AND NEW.state IN ('acknowledged','uncertain','terminal_observed'))
                    OR (OLD.state='uncertain' AND NEW.state='terminal_observed')
                    OR (OLD.state='uncertain' AND NEW.state='acknowledged' AND NEW.outcome_required)) THEN
                    RAISE EXCEPTION 'runtime control transition is not allowed';
                END IF;
                RETURN NEW;
            END $$;
            CREATE FUNCTION require_runtime_control_outcome() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF EXISTS(SELECT 1 FROM runtime_control_commands WHERE id=NEW.id AND outcome_required)
                    AND NOT EXISTS(SELECT 1 FROM runtime_control_outcomes WHERE command_id=NEW.id) THEN
                    RAISE EXCEPTION 'runtime control original outcome context is missing';
                END IF;
                RETURN NULL;
            END $$;
            CREATE CONSTRAINT TRIGGER runtime_control_outcome_required AFTER INSERT OR UPDATE
                ON runtime_control_commands DEFERRABLE INITIALLY DEFERRED
                FOR EACH ROW EXECUTE FUNCTION require_runtime_control_outcome();
            CREATE FUNCTION require_runtime_control_outcome_ack() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF EXISTS(SELECT 1 FROM runtime_control_outcomes o
                    JOIN runtime_control_commands c ON c.id=o.command_id WHERE o.command_id=NEW.command_id
                        AND o.state='acknowledged' AND NOT (c.outcome_required
                            AND ((c.state='acknowledged' AND c.acknowledgement=o.acknowledgement)
                                OR c.state='terminal_observed'))) THEN
                    RAISE EXCEPTION 'runtime control outcome ACK requires an atomic receipt';
                END IF;
                RETURN NULL;
            END $$;
            CREATE CONSTRAINT TRIGGER runtime_control_outcome_atomic_ack AFTER INSERT OR UPDATE
                ON runtime_control_outcomes DEFERRABLE INITIALLY DEFERRED
                FOR EACH ROW EXECUTE FUNCTION require_runtime_control_outcome_ack();"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "LOCK TABLE runtime_control_commands,runtime_control_outcomes IN ACCESS EXCLUSIVE MODE;
            DO $$ BEGIN
                IF EXISTS(SELECT 1 FROM runtime_control_outcomes)
                    OR EXISTS(SELECT 1 FROM runtime_control_commands WHERE outcome_required) THEN
                    RAISE EXCEPTION 'nonempty runtime control outcome history cannot be removed';
                END IF;
            END $$;
            DROP TRIGGER runtime_control_outcome_required ON runtime_control_commands;
            DROP FUNCTION require_runtime_control_outcome();
            DROP TABLE runtime_control_outcomes;
            DROP FUNCTION guard_runtime_control_outcome();
            DROP FUNCTION require_runtime_control_outcome_ack();
            ALTER TABLE runtime_control_commands DROP COLUMN outcome_required;
            CREATE OR REPLACE FUNCTION guard_runtime_control_command() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF (NEW.id,NEW.session_id,NEW.session_run_id,NEW.agent_id,NEW.actor_user_id,
                    NEW.operation,NEW.idempotency_key,NEW.payload_sha256,NEW.runtime_run_id,
                    NEW.runtime_session_id,NEW.original_request_sha256,NEW.api_origin,
                    NEW.credential_fingerprint,NEW.created_at)
                  IS DISTINCT FROM
                   (OLD.id,OLD.session_id,OLD.session_run_id,OLD.agent_id,OLD.actor_user_id,
                    OLD.operation,OLD.idempotency_key,OLD.payload_sha256,OLD.runtime_run_id,
                    OLD.runtime_session_id,OLD.original_request_sha256,OLD.api_origin,
                    OLD.credential_fingerprint,OLD.created_at) THEN
                    RAISE EXCEPTION 'runtime control identity is immutable';
                END IF;
                IF NOT ((OLD.state='reserved' AND NEW.state IN ('submitted','rejected'))
                    OR (OLD.state='submitted' AND NEW.state IN ('acknowledged','uncertain','terminal_observed'))
                    OR (OLD.state='uncertain' AND NEW.state='terminal_observed')) THEN
                    RAISE EXCEPTION 'runtime control transition is not allowed';
                END IF;
                RETURN NEW;
            END $$;"
        ).await?;
        Ok(())
    }
}
