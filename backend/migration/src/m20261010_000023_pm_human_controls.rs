use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "LOCK TABLE runtime_control_commands, clarification_answer_commands IN ACCESS EXCLUSIVE MODE;
            ALTER TABLE runtime_control_commands DROP CONSTRAINT runtime_control_commands_session_run_id_fkey;
            ALTER TABLE runtime_control_commands ADD CONSTRAINT runtime_control_commands_session_run_id_fkey
                FOREIGN KEY(session_run_id) REFERENCES session_agent_runs(id);
            CREATE FUNCTION admit_runtime_control_custody() RETURNS trigger LANGUAGE plpgsql AS $$
            DECLARE legacy boolean; pm boolean;
            BEGIN
                SELECT EXISTS(SELECT 1 FROM hermes_dispatch_journal j JOIN session_agent_runs r ON r.id=j.run_id
                    JOIN agent_sessions s ON s.id=r.session_id
                    WHERE j.run_id=NEW.session_run_id AND j.state='accepted' AND j.submitted_at IS NOT NULL
                      AND j.session_id=NEW.session_id AND j.agent_id=NEW.agent_id
                      AND r.session_id=NEW.session_id AND r.agent_id=NEW.agent_id AND s.agent_id=r.agent_id
                      AND r.runtime_run_id=NEW.runtime_run_id AND r.runtime_session_id=NEW.runtime_session_id
                      AND j.request_hash=NEW.original_request_sha256 AND j.origin=NEW.api_origin
                      AND j.credential_fingerprint=NEW.credential_fingerprint
                      AND NOT EXISTS(SELECT 1 FROM task_chat_bindings WHERE session_id=s.id)
                      AND NOT EXISTS(SELECT 1 FROM pm_run_bindings WHERE session_run_id=r.id)) INTO legacy;
                SELECT EXISTS(SELECT 1 FROM pm_dispatch_journal j JOIN pm_run_bindings b USING(session_run_id)
                    JOIN session_agent_runs r ON r.id=b.session_run_id JOIN agent_sessions s ON s.id=b.session_id
                    JOIN task_chat_bindings t ON t.session_id=s.id JOIN users u ON u.id=s.user_id
                    JOIN agents a ON a.id=b.agent_id
                    WHERE j.session_run_id=NEW.session_run_id AND j.submitted AND j.hermes_run_ref=b.hermes_run_ref
                      AND b.terminal_status IS NULL AND b.session_id=NEW.session_id AND b.agent_id=NEW.agent_id
                      AND (b.reservation->>'checkpoint_ref' IS NOT NULL OR j.guidance_delivered)
                      AND NOT EXISTS(SELECT 1 FROM pm_tool_commands WHERE session_run_id=b.session_run_id AND kind='stop' AND attempted)
                      AND NOT EXISTS(SELECT 1 FROM runtime_control_commands WHERE session_run_id=b.session_run_id AND operation='stop' AND state='acknowledged')
                      AND NOT EXISTS(SELECT 1 FROM agent_config_heads WHERE agent_id=b.agent_id AND draining)
                      AND r.session_id=s.id AND r.agent_id=b.agent_id AND s.agent_id=b.agent_id
                      AND s.state='active' AND u.is_active
                      AND s.user_id=NEW.actor_user_id AND u.central_sub=t.owner_subject
                      AND a.kind='hermes' AND a.sdlc_role='project_manager' AND a.status='running' AND a.archived_at IS NULL
                      AND r.state IN ('running','waiting','stopping') AND (NEW.operation='stop' OR r.state='running')
                      AND b.hermes_run_ref=NEW.runtime_run_id AND b.hermes_session_ref=NEW.runtime_session_id
                      AND r.runtime_run_id=NEW.runtime_run_id
                      AND r.runtime_session_id=(j.intent->>'request_body')::jsonb->>'session_id'
                      AND encode(sha256(convert_to(j.intent->>'request_body','UTF8')),'hex')=NEW.original_request_sha256
                      AND j.intent->>'origin'=NEW.api_origin AND j.intent->>'credential_fingerprint'=NEW.credential_fingerprint
                      AND b.reservation->'identity'->>'task_ref'=t.task_id::text
                      AND b.reservation->'identity'->>'root_ref'=t.root_task_id::text
                      AND b.reservation->'identity'->>'tracker_instance_ref'=t.tracker_instance_id
                      AND b.reservation->'identity'->>'tracker_project_ref'=t.project_id::text
                      AND NOT EXISTS(SELECT 1 FROM hermes_dispatch_journal WHERE run_id=r.id)) INTO pm;
                IF NOT (legacy OR pm) OR NEW.state <> 'reserved' THEN
                    RAISE EXCEPTION 'runtime control requires original admitted dispatch custody';
                END IF;
                RETURN NEW;
            END $$;
            CREATE TRIGGER runtime_control_custody BEFORE INSERT ON runtime_control_commands
                FOR EACH ROW EXECUTE FUNCTION admit_runtime_control_custody();
            ALTER TABLE clarification_answer_commands ADD COLUMN continuation_state text NOT NULL DEFAULT 'not_required'
                CHECK(continuation_state IN ('not_required','pending','confirmed'));
            DROP TRIGGER clarification_answer_immutable ON clarification_answer_commands;
            ALTER FUNCTION guard_clarification_answer_command() RENAME TO guard_clarification_answer_command_v20;
            UPDATE clarification_answer_commands c SET continuation_state='pending'
                WHERE c.state <> 'rejected' AND (EXISTS(SELECT 1 FROM pm_draft_creation_operations p
                    WHERE p.owner_user_id=c.actor_user_id AND p.operation->>'session_id'=c.session_id::text)
                    OR EXISTS(SELECT 1 FROM pm_run_bindings WHERE session_id=c.session_id));
            ALTER TABLE clarification_answer_commands ADD CONSTRAINT clarification_continuation_confirmed
                CHECK(continuation_state <> 'confirmed' OR state='delivered');
            CREATE FUNCTION guard_clarification_answer_command() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF TG_OP='INSERT' THEN
                    IF NEW.state <> 'stored' OR NEW.attempt_id IS NOT NULL OR NEW.ever_uncertain
                        OR NEW.continuation_state <> 'not_required' THEN
                        RAISE EXCEPTION 'clarification command must start in stored custody';
                    END IF;
                    IF EXISTS(SELECT 1 FROM clarification_answer_commands c WHERE c.session_id=NEW.session_id
                        AND c.state='delivered' AND c.continuation_state='pending') THEN
                        RAISE EXCEPTION 'original PM answer continuation is pending';
                    END IF;
                    IF EXISTS(SELECT 1 FROM pm_draft_creation_operations p WHERE p.owner_user_id=NEW.actor_user_id
                        AND p.operation->>'session_id'=NEW.session_id::text)
                        OR EXISTS(SELECT 1 FROM pm_run_bindings WHERE session_id=NEW.session_id) THEN
                        NEW.continuation_state='pending';
                    END IF;
                    RETURN NEW;
                END IF;
                IF TG_OP='DELETE' THEN RAISE EXCEPTION 'clarification command history cannot be deleted'; END IF;
                IF (NEW.id,NEW.session_id,NEW.question_id,NEW.actor_user_id,NEW.owner_subject,
                    NEW.binding,NEW.idempotency_key,NEW.request_body,NEW.payload_sha256,NEW.created_at)
                    IS DISTINCT FROM (OLD.id,OLD.session_id,OLD.question_id,OLD.actor_user_id,OLD.owner_subject,
                    OLD.binding,OLD.idempotency_key,OLD.request_body,OLD.payload_sha256,OLD.created_at) THEN
                    RAISE EXCEPTION 'clarification command identity is immutable';
                END IF;
                IF OLD.continuation_state='not_required' AND NEW.continuation_state='not_required'
                    AND OLD.state='delivering' AND NEW.state='delivered'
                    AND (EXISTS(SELECT 1 FROM pm_draft_creation_operations p WHERE p.owner_user_id=NEW.actor_user_id
                        AND p.operation->>'session_id'=NEW.session_id::text)
                        OR EXISTS(SELECT 1 FROM pm_run_bindings WHERE session_id=NEW.session_id)) THEN
                    NEW.continuation_state='pending';
                END IF;
                IF NEW.continuation_state IS DISTINCT FROM OLD.continuation_state THEN
                    IF OLD.continuation_state='not_required' AND NEW.continuation_state='pending'
                        AND OLD.state='delivering' AND NEW.state='delivered'
                        AND (EXISTS(SELECT 1 FROM pm_draft_creation_operations p WHERE p.owner_user_id=NEW.actor_user_id
                            AND p.operation->>'session_id'=NEW.session_id::text)
                            OR EXISTS(SELECT 1 FROM pm_run_bindings WHERE session_id=NEW.session_id)) THEN
                        NULL;
                    ELSE
                    IF OLD.continuation_state <> 'pending' OR NEW.continuation_state <> 'confirmed'
                        OR OLD.state <> 'delivered' OR NEW.state <> 'delivered'
                        OR (to_jsonb(NEW)-'continuation_state'-'updated_at') IS DISTINCT FROM
                           (to_jsonb(OLD)-'continuation_state'-'updated_at') THEN
                        RAISE EXCEPTION 'clarification continuation cannot erase original custody';
                    END IF;
                    RETURN NEW;
                    END IF;
                END IF;
                IF NOT ((OLD.state IN ('stored','uncertain') AND NEW.state='delivering' AND NEW.attempt_id IS DISTINCT FROM OLD.attempt_id)
                    OR (OLD.state='delivering' AND NEW.state='delivering' AND NEW.ever_uncertain AND OLD.lease_until <= clock_timestamp()
                        AND NEW.attempt_id IS DISTINCT FROM OLD.attempt_id)
                    OR (OLD.state='delivering' AND NEW.state IN ('uncertain','delivered','rejected') AND NEW.attempt_id=OLD.attempt_id)) THEN
                    RAISE EXCEPTION 'clarification command transition is not allowed';
                END IF;
                IF OLD.ever_uncertain AND NOT NEW.ever_uncertain THEN RAISE EXCEPTION 'clarification uncertainty cannot be erased'; END IF;
                RETURN NEW;
            END $$;
            CREATE TRIGGER clarification_answer_immutable BEFORE INSERT OR UPDATE OR DELETE ON clarification_answer_commands
                FOR EACH ROW EXECUTE FUNCTION guard_clarification_answer_command();
            CREATE INDEX clarification_continuation_pending ON clarification_answer_commands(session_id,created_at,id)
                WHERE state='delivered' AND continuation_state='pending';"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "LOCK TABLE runtime_control_commands, clarification_answer_commands IN ACCESS EXCLUSIVE MODE;
            DO $$ BEGIN
                IF EXISTS(SELECT 1 FROM runtime_control_commands c WHERE NOT EXISTS
                    (SELECT 1 FROM hermes_dispatch_journal j WHERE j.run_id=c.session_run_id))
                    OR EXISTS(SELECT 1 FROM clarification_answer_commands WHERE continuation_state <> 'not_required') THEN
                    RAISE EXCEPTION 'PM custody history prevents human controls downgrade';
                END IF;
            END $$;
            DROP TRIGGER runtime_control_custody ON runtime_control_commands;
            DROP FUNCTION admit_runtime_control_custody();
            ALTER TABLE runtime_control_commands DROP CONSTRAINT runtime_control_commands_session_run_id_fkey;
            ALTER TABLE runtime_control_commands ADD CONSTRAINT runtime_control_commands_session_run_id_fkey
                FOREIGN KEY(session_run_id) REFERENCES hermes_dispatch_journal(run_id);
            DROP TRIGGER clarification_answer_immutable ON clarification_answer_commands;
            DROP FUNCTION guard_clarification_answer_command();
            ALTER FUNCTION guard_clarification_answer_command_v20() RENAME TO guard_clarification_answer_command;
            ALTER TABLE clarification_answer_commands DROP COLUMN continuation_state;
            CREATE TRIGGER clarification_answer_immutable BEFORE INSERT OR UPDATE OR DELETE ON clarification_answer_commands
                FOR EACH ROW EXECUTE FUNCTION guard_clarification_answer_command();"
        ).await?;
        Ok(())
    }
}
