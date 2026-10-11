use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "LOCK TABLE runtime_control_commands IN ACCESS EXCLUSIVE MODE;
            ALTER FUNCTION admit_runtime_control_custody() RENAME TO admit_runtime_control_custody_v25;
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
                      AND (NEW.operation='stop' OR b.reservation->>'checkpoint_ref' IS NOT NULL OR j.guidance_delivered)
                      AND NOT EXISTS(SELECT 1 FROM pm_tool_commands WHERE session_run_id=b.session_run_id AND kind='stop' AND attempted)
                      AND NOT EXISTS(SELECT 1 FROM runtime_control_commands WHERE session_run_id=b.session_run_id AND operation='stop' AND state='acknowledged')
                      AND (NEW.operation='stop' OR NOT EXISTS(SELECT 1 FROM agent_config_heads WHERE agent_id=b.agent_id AND draining))
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
            DROP TRIGGER runtime_control_custody ON runtime_control_commands;
            CREATE TRIGGER runtime_control_custody BEFORE INSERT ON runtime_control_commands
                FOR EACH ROW EXECUTE FUNCTION admit_runtime_control_custody();"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "LOCK TABLE runtime_control_commands, pm_run_bindings IN ACCESS EXCLUSIVE MODE;
            DO $guard$
            BEGIN
                IF EXISTS(SELECT 1 FROM runtime_control_commands c
                    JOIN pm_run_bindings b ON b.session_run_id=c.session_run_id
                    WHERE c.operation='stop') THEN
                    RAISE EXCEPTION 'PM stop custody prevents drain prerequisite downgrade';
                END IF;
            END $guard$;
            DROP TRIGGER runtime_control_custody ON runtime_control_commands;
            DROP FUNCTION admit_runtime_control_custody();
            ALTER FUNCTION admit_runtime_control_custody_v25() RENAME TO admit_runtime_control_custody;
            CREATE TRIGGER runtime_control_custody BEFORE INSERT ON runtime_control_commands
                FOR EACH ROW EXECUTE FUNCTION admit_runtime_control_custody();"
        ).await?;
        Ok(())
    }
}
