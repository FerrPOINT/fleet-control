use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE pm_dispatch_journal (
                session_run_id uuid PRIMARY KEY REFERENCES pm_run_bindings(session_run_id),
                intent jsonb NOT NULL,
                submitted boolean NOT NULL DEFAULT false,
                hermes_run_ref text,
                guidance_body text CHECK(octet_length(guidance_body) BETWEEN 1 AND 32768),
                guidance_attempted boolean NOT NULL DEFAULT false,
                guidance_delivered boolean NOT NULL DEFAULT false,
                terminal_committed boolean NOT NULL DEFAULT false,
                created_at timestamptz NOT NULL DEFAULT now(),
                CHECK((intent->>'session_run_id'=session_run_id::text) IS TRUE),
                CHECK((intent->'submitted'='false'::jsonb AND intent->'hermes_run_ref'='null'::jsonb) IS TRUE),
                CHECK((octet_length(intent->>'request_body') BETWEEN 1 AND 1048576) IS TRUE),
                CHECK((length(intent->>'origin') BETWEEN 1 AND 1024) IS TRUE),
                CHECK((length(intent->>'workflow_origin') BETWEEN 1 AND 1024) IS TRUE),
                CHECK((intent->>'credential_fingerprint' ~ '^[0-9a-f]{64}$') IS TRUE),
                CHECK((intent->>'workflow_credential_fingerprint' ~ '^[0-9a-f]{64}$') IS TRUE),
                CHECK(hermes_run_ref IS NULL OR (submitted AND length(hermes_run_ref) BETWEEN 1 AND 512
                    AND hermes_run_ref ~ '^[A-Za-z0-9_-]+$')),
                CHECK(guidance_attempted = (guidance_body IS NOT NULL)),
                CHECK(NOT guidance_attempted OR hermes_run_ref IS NOT NULL),
                CHECK(NOT guidance_delivered OR guidance_attempted),
                CHECK(NOT terminal_committed OR hermes_run_ref IS NOT NULL)
            );
            CREATE FUNCTION guard_pm_dispatch_journal() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF TG_OP='DELETE' THEN RAISE EXCEPTION 'PM dispatch custody cannot be deleted'; END IF;
                IF TG_OP='INSERT' THEN
                    IF NEW.submitted OR NEW.hermes_run_ref IS NOT NULL OR NEW.guidance_attempted OR NEW.guidance_delivered OR NEW.terminal_committed THEN
                        RAISE EXCEPTION 'PM dispatch must start unsubmitted';
                    END IF;
                ELSE
                    IF (NEW.session_run_id,NEW.intent,NEW.created_at) IS DISTINCT FROM
                       (OLD.session_run_id,OLD.intent,OLD.created_at)
                       OR (OLD.submitted AND NOT NEW.submitted)
                       OR (OLD.hermes_run_ref IS NOT NULL AND NEW.hermes_run_ref IS DISTINCT FROM OLD.hermes_run_ref)
                       OR (OLD.guidance_attempted AND (NOT NEW.guidance_attempted OR NEW.guidance_body IS DISTINCT FROM OLD.guidance_body))
                       OR (OLD.guidance_delivered AND NOT NEW.guidance_delivered)
                       OR (OLD.terminal_committed AND NOT NEW.terminal_committed) THEN
                        RAISE EXCEPTION 'PM dispatch custody is immutable';
                    END IF;
                END IF;
                RETURN NEW;
            END $$;
            CREATE TRIGGER pm_dispatch_immutable BEFORE INSERT OR UPDATE OR DELETE ON pm_dispatch_journal
                FOR EACH ROW EXECUTE FUNCTION guard_pm_dispatch_journal();
            CREATE UNIQUE INDEX pm_creation_session ON pm_draft_creation_operations((operation->>'session_id'))
                WHERE operation->>'session_id' IS NOT NULL;
            CREATE TABLE pm_tool_commands (
                session_run_id uuid NOT NULL REFERENCES pm_run_bindings(session_run_id),
                operation_key text NOT NULL CHECK(length(operation_key) BETWEEN 1 AND 128),
                kind text NOT NULL CHECK(kind IN ('question','revision','stop','workflow_step')),
                request jsonb NOT NULL CHECK(jsonb_typeof(request)='object' AND octet_length(request::text)<=262144),
                attempted boolean NOT NULL DEFAULT false,
                result jsonb CHECK(octet_length(result::text)<=262144),
                PRIMARY KEY(session_run_id,operation_key),
                CHECK(result IS NULL OR attempted)
            );
            CREATE FUNCTION guard_pm_tool_command() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF TG_OP='DELETE' THEN RAISE EXCEPTION 'PM tool custody cannot be deleted'; END IF;
                IF TG_OP='INSERT' THEN
                    IF NEW.attempted OR NEW.result IS NOT NULL THEN RAISE EXCEPTION 'PM tool must start unsubmitted'; END IF;
                ELSE
                    IF (NEW.session_run_id,NEW.operation_key,NEW.kind,NEW.request) IS DISTINCT FROM
                       (OLD.session_run_id,OLD.operation_key,OLD.kind,OLD.request)
                       OR (OLD.attempted AND NOT NEW.attempted)
                       OR (OLD.result IS NOT NULL AND NEW.result IS DISTINCT FROM OLD.result) THEN
                        RAISE EXCEPTION 'PM tool custody is immutable';
                    END IF;
                END IF;
                RETURN NEW;
            END $$;
            CREATE TRIGGER pm_tool_immutable BEFORE INSERT OR UPDATE OR DELETE ON pm_tool_commands
                FOR EACH ROW EXECUTE FUNCTION guard_pm_tool_command();"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "LOCK TABLE pm_dispatch_journal, pm_tool_commands IN ACCESS EXCLUSIVE MODE;
             DO $$ BEGIN
                 IF EXISTS(SELECT 1 FROM pm_dispatch_journal) OR EXISTS(SELECT 1 FROM pm_tool_commands) THEN
                     RAISE EXCEPTION 'PM dispatch custody prevents downgrade';
                 END IF;
             END $$;
             DROP TABLE IF EXISTS pm_tool_commands; DROP FUNCTION IF EXISTS guard_pm_tool_command();
             DROP INDEX IF EXISTS pm_creation_session;
             DROP TABLE IF EXISTS pm_dispatch_journal; DROP FUNCTION IF EXISTS guard_pm_dispatch_journal();"
        ).await?;
        Ok(())
    }
}
