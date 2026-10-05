use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE runtime_control_commands (
                id uuid PRIMARY KEY,
                session_id uuid NOT NULL REFERENCES agent_sessions(id),
                session_run_id uuid NOT NULL REFERENCES hermes_dispatch_journal(run_id),
                agent_id uuid NOT NULL REFERENCES agents(id),
                actor_user_id uuid NOT NULL REFERENCES users(id),
                operation text NOT NULL CHECK(operation IN ('steer','stop')),
                idempotency_key text NOT NULL CHECK(length(idempotency_key) BETWEEN 1 AND 128),
                payload_sha256 text NOT NULL CHECK(payload_sha256 ~ '^[a-f0-9]{64}$'),
                runtime_run_id text NOT NULL,
                runtime_session_id text NOT NULL,
                original_request_sha256 text NOT NULL CHECK(original_request_sha256 ~ '^[a-f0-9]{64}$'),
                api_origin text NOT NULL,
                credential_fingerprint text NOT NULL CHECK(credential_fingerprint ~ '^[a-f0-9]{64}$'),
                state text NOT NULL DEFAULT 'reserved'
                    CHECK(state IN ('reserved','submitted','acknowledged','uncertain','rejected','terminal_observed')),
                acknowledgement text CHECK(acknowledgement IN ('steered','stopping','already_terminal')),
                observed_run_state text CHECK(observed_run_state IN ('completed','failed','cancelled')),
                created_at timestamptz NOT NULL DEFAULT now(),
                updated_at timestamptz NOT NULL DEFAULT now(),
                UNIQUE(actor_user_id,idempotency_key),
                CHECK((state='acknowledged') = (acknowledgement IS NOT NULL)),
                CHECK((state='terminal_observed') = (observed_run_state IS NOT NULL))
            );
            CREATE UNIQUE INDEX runtime_control_active_run ON runtime_control_commands(session_run_id)
                WHERE state IN ('reserved','submitted','uncertain');
            CREATE INDEX runtime_control_reconcile ON runtime_control_commands(id)
                WHERE state IN ('reserved','submitted','uncertain');
            CREATE INDEX runtime_control_session ON runtime_control_commands(session_id,id);
            CREATE INDEX runtime_control_run_history ON runtime_control_commands(session_run_id,created_at DESC,id);
            CREATE FUNCTION guard_runtime_control_command() RETURNS trigger LANGUAGE plpgsql AS $$
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
            END $$;
            CREATE TRIGGER runtime_control_immutable BEFORE UPDATE ON runtime_control_commands
                FOR EACH ROW EXECUTE FUNCTION guard_runtime_control_command();"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "LOCK TABLE runtime_control_commands IN ACCESS EXCLUSIVE MODE;
            DO $$ BEGIN
                IF EXISTS (SELECT 1 FROM runtime_control_commands) THEN
                    RAISE EXCEPTION 'nonempty runtime control history cannot be removed';
                END IF;
            END $$;
            DROP TABLE runtime_control_commands;
            DROP FUNCTION guard_runtime_control_command();",
            )
            .await?;
        Ok(())
    }
}
