use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE clarification_answer_commands (
                id uuid PRIMARY KEY,
                session_id uuid NOT NULL REFERENCES task_chat_bindings(session_id),
                question_id uuid NOT NULL,
                actor_user_id uuid NOT NULL REFERENCES users(id),
                owner_subject text NOT NULL,
                binding jsonb NOT NULL,
                idempotency_key text NOT NULL CHECK(length(idempotency_key) BETWEEN 1 AND 128),
                request_body text NOT NULL CHECK(octet_length(request_body) <= 300000),
                payload_sha256 text NOT NULL CHECK(payload_sha256 = encode(sha256(convert_to(request_body,'UTF8')),'hex')),
                state text NOT NULL DEFAULT 'stored' CHECK(state IN ('stored','delivering','uncertain','delivered','rejected')),
                attempt_id uuid,
                lease_until timestamptz,
                ever_uncertain boolean NOT NULL DEFAULT false,
                answer jsonb,
                rejection_status integer CHECK(rejection_status BETWEEN 400 AND 499),
                created_at timestamptz NOT NULL DEFAULT now(),
                updated_at timestamptz NOT NULL DEFAULT now(),
                UNIQUE(actor_user_id,idempotency_key),
                CHECK(request_body::jsonb->>'idempotency_key' = idempotency_key),
                CHECK(binding->>'owner_subject' = owner_subject),
                CHECK((state='delivered') = (answer IS NOT NULL)),
                CHECK((state='rejected') = (rejection_status IS NOT NULL)),
                CHECK(state <> 'rejected' OR NOT ever_uncertain),
                CHECK(state <> 'uncertain' OR ever_uncertain),
                CHECK((state='delivering') = (lease_until IS NOT NULL)),
                CHECK(state='stored' OR attempt_id IS NOT NULL)
            );
            CREATE UNIQUE INDEX clarification_answer_pending_question ON clarification_answer_commands(session_id,question_id)
                WHERE state IN ('stored','delivering','uncertain');
            CREATE UNIQUE INDEX clarification_answer_pending_task ON clarification_answer_commands
                ((binding->>'tracker_instance_id'),(binding->>'task_id'),question_id,actor_user_id)
                WHERE state IN ('stored','delivering','uncertain');
            CREATE INDEX clarification_answer_pending_session ON clarification_answer_commands(session_id,created_at,id)
                WHERE state IN ('stored','delivering','uncertain');
            CREATE FUNCTION guard_clarification_answer_command() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF TG_OP='INSERT' THEN
                    IF NEW.state <> 'stored' OR NEW.attempt_id IS NOT NULL OR NEW.ever_uncertain THEN
                        RAISE EXCEPTION 'clarification command must start in stored custody';
                    END IF;
                    RETURN NEW;
                END IF;
                IF TG_OP='DELETE' THEN
                    RAISE EXCEPTION 'clarification command history cannot be deleted';
                END IF;
                IF (NEW.id,NEW.session_id,NEW.question_id,NEW.actor_user_id,NEW.owner_subject,
                    NEW.binding,NEW.idempotency_key,NEW.request_body,NEW.payload_sha256,NEW.created_at)
                  IS DISTINCT FROM
                   (OLD.id,OLD.session_id,OLD.question_id,OLD.actor_user_id,OLD.owner_subject,
                    OLD.binding,OLD.idempotency_key,OLD.request_body,OLD.payload_sha256,OLD.created_at) THEN
                    RAISE EXCEPTION 'clarification command identity is immutable';
                END IF;
                IF NOT ((OLD.state IN ('stored','uncertain') AND NEW.state='delivering' AND NEW.attempt_id IS DISTINCT FROM OLD.attempt_id)
                    OR (OLD.state='delivering' AND NEW.state='delivering' AND NEW.ever_uncertain AND OLD.lease_until <= clock_timestamp()
                        AND NEW.attempt_id IS DISTINCT FROM OLD.attempt_id)
                    OR (OLD.state='delivering' AND NEW.state IN ('uncertain','delivered','rejected')
                        AND NEW.attempt_id=OLD.attempt_id)) THEN
                    RAISE EXCEPTION 'clarification command transition is not allowed';
                END IF;
                IF OLD.ever_uncertain AND NOT NEW.ever_uncertain THEN
                    RAISE EXCEPTION 'clarification uncertainty cannot be erased';
                END IF;
                RETURN NEW;
            END $$;
            CREATE TRIGGER clarification_answer_immutable BEFORE INSERT OR UPDATE OR DELETE ON clarification_answer_commands
                FOR EACH ROW EXECUTE FUNCTION guard_clarification_answer_command();"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "LOCK TABLE clarification_answer_commands IN ACCESS EXCLUSIVE MODE;
            DO $$ BEGIN
                IF EXISTS (SELECT 1 FROM clarification_answer_commands) THEN
                    RAISE EXCEPTION 'nonempty clarification command history cannot be removed';
                END IF;
            END $$;
            DROP TABLE clarification_answer_commands;
            DROP FUNCTION guard_clarification_answer_command();",
            )
            .await?;
        Ok(())
    }
}
