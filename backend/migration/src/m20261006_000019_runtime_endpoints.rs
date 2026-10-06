use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

fn dispatch_guard(predicate: &str) -> String {
    // Both directions retain the previous immutable request/ACK rules verbatim.
    format!("{GUARD_PREFIX}{predicate}{GUARD_SUFFIX}")
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE runtime_launch_endpoints (
                launch_id uuid PRIMARY KEY REFERENCES runtime_launches(id),
                origin text NOT NULL CHECK (origin ~ '^http://[0-9.]+:[0-9]{4,5}$'),
                pid integer NOT NULL CHECK (pid>0),
                created_at timestamptz NOT NULL DEFAULT clock_timestamp()
             );
             CREATE FUNCTION fleet_guard_runtime_endpoint() RETURNS trigger AS $$
             BEGIN
                IF TG_OP<>'INSERT' OR NOT EXISTS (
                    SELECT 1 FROM runtime_launches l JOIN agents a ON a.id=l.agent_id
                    WHERE l.id=NEW.launch_id AND l.state='gateway_started' AND l.pid=NEW.pid
                      AND l.binding->>'kind'='hermes' AND a.kind='hermes' AND a.archived_at IS NULL
                      AND jsonb_typeof(l.binding->'container')='object'
                      AND a.api_port=(l.binding->>'api_port')::integer
                      AND split_part(NEW.origin,':',3)=a.api_port::text
                      AND host(split_part(substr(NEW.origin,8),':',1)::inet)=split_part(substr(NEW.origin,8),':',1)
                      AND (split_part(substr(NEW.origin,8),':',1)::inet <<= '10.0.0.0/8'::inet
                        OR split_part(substr(NEW.origin,8),':',1)::inet <<= '172.16.0.0/12'::inet
                        OR split_part(substr(NEW.origin,8),':',1)::inet <<= '192.168.0.0/16'::inet
                        OR split_part(substr(NEW.origin,8),':',1)::inet='127.0.0.1'::inet)) THEN
                    RAISE EXCEPTION 'Runtime endpoint requires original running container custody' USING ERRCODE='23514';
                END IF;
                RETURN NEW;
             END;
             $$ LANGUAGE plpgsql;
             CREATE TRIGGER fleet_runtime_endpoint_guard BEFORE INSERT OR UPDATE OR DELETE ON runtime_launch_endpoints
                FOR EACH ROW EXECUTE FUNCTION fleet_guard_runtime_endpoint();
             CREATE TRIGGER fleet_runtime_endpoint_no_truncate BEFORE TRUNCATE ON runtime_launch_endpoints
                FOR EACH STATEMENT EXECUTE FUNCTION fleet_guard_runtime_endpoint();
             CREATE FUNCTION fleet_hermes_origin_matches(agent uuid, endpoint text, facts jsonb)
             RETURNS boolean LANGUAGE sql STABLE AS $$
                SELECT CASE WHEN EXISTS (SELECT 1 FROM runtime_launches l WHERE l.agent_id=agent
                    AND l.state IN ('claimed','gateway_started') AND jsonb_typeof(l.binding->'container')='object')
                THEN EXISTS(SELECT 1 FROM runtime_launches l JOIN runtime_launch_endpoints e ON e.launch_id=l.id
                    WHERE l.agent_id=agent AND l.state='gateway_started' AND l.binding->>'kind'='hermes'
                      AND e.pid=l.pid AND e.origin=endpoint AND l.id::text=facts #>> '{fleet_launch,launch_id}')
                ELSE EXISTS(SELECT 1 FROM agents a WHERE a.id=agent AND endpoint='http://127.0.0.1:' || a.api_port::text)
                END
             $$;
             ALTER TABLE hermes_dispatch_journal DROP CONSTRAINT hermes_dispatch_journal_origin_check;
             ALTER TABLE hermes_dispatch_journal ADD CONSTRAINT hermes_dispatch_journal_origin_check
                CHECK (origin ~ '^http://[0-9.]+:[0-9]{4,5}$');"
        ).await?;
        manager
            .get_connection()
            .execute_unprepared(&dispatch_guard(
                "fleet_hermes_origin_matches(a.id,NEW.origin,NEW.capabilities)",
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "LOCK TABLE runtime_launch_endpoints,hermes_dispatch_journal IN ACCESS EXCLUSIVE MODE;
             DO $$ BEGIN
                IF EXISTS(SELECT 1 FROM runtime_launch_endpoints) OR EXISTS(
                    SELECT 1 FROM hermes_dispatch_journal WHERE origin !~ '^http://127[.]0[.]0[.]1:[0-9]{4,5}$') THEN
                    RAISE EXCEPTION 'Runtime endpoint history requires reconciliation before downgrade' USING ERRCODE='23514';
                END IF;
             END $$;"
        ).await?;
        manager
            .get_connection()
            .execute_unprepared(&dispatch_guard(
                "NEW.origin='http://127.0.0.1:' || a.api_port::text",
            ))
            .await?;
        manager.get_connection().execute_unprepared(
            "ALTER TABLE hermes_dispatch_journal DROP CONSTRAINT hermes_dispatch_journal_origin_check;
             ALTER TABLE hermes_dispatch_journal ADD CONSTRAINT hermes_dispatch_journal_origin_check
                CHECK (origin ~ '^http://127[.]0[.]0[.]1:[0-9]{4,5}$');
             DROP FUNCTION fleet_hermes_origin_matches(uuid,text,jsonb);
             DROP TABLE runtime_launch_endpoints;
             DROP FUNCTION fleet_guard_runtime_endpoint();"
        ).await?;
        Ok(())
    }
}

const GUARD_PREFIX: &str = "CREATE OR REPLACE FUNCTION fleet_guard_hermes_dispatch() RETURNS trigger AS $$
BEGIN
 IF TG_OP='DELETE' THEN
  RAISE EXCEPTION 'Hermes dispatch journal cannot be deleted' USING ERRCODE='23514';
 END IF;
 IF TG_OP='INSERT' THEN
  IF NEW.state<>'prepared' OR NEW.submitted_at IS NOT NULL OR NEW.accepted_at IS NOT NULL
   OR jsonb_typeof(NEW.request_body::jsonb) IS DISTINCT FROM 'object'
   OR jsonb_typeof(NEW.request_body::jsonb->'input') IS DISTINCT FROM 'string'
   OR NEW.request_body::jsonb->>'session_id' IS DISTINCT FROM NEW.requested_session_id
   OR NEW.request_body::jsonb - 'input' - 'session_id' - 'model' - 'provider' - 'model_options'<>'{}'::jsonb
   OR NOT EXISTS(SELECT 1 FROM session_agent_runs r JOIN agent_sessions s ON s.id=r.session_id
    JOIN agents a ON a.id=r.agent_id JOIN session_messages m ON m.id=NEW.message_id
    JOIN message_dispatch_outbox o ON o.message_id=m.id
    WHERE r.id=NEW.run_id AND r.session_id=NEW.session_id AND r.agent_id=NEW.agent_id
     AND s.agent_id=NEW.agent_id AND m.session_id=NEW.session_id AND o.agent_id=NEW.agent_id
     AND a.kind='hermes' AND a.status='running' AND a.archived_at IS NULL
     AND a.api_port BETWEEN 1024 AND 65535 AND ";

const GUARD_SUFFIX: &str = "
     AND NOT EXISTS(SELECT 1 FROM agent_config_heads WHERE agent_id=a.id AND draining)
     AND NEW.request_body::jsonb->>'input'=CASE
      WHEN m.author_type='agent' AND m.author_agent_id IS DISTINCT FROM NEW.agent_id THEN
       E'[Fleet Control]\\nSession: ' || s.title || E'\\nTask: ' || COALESCE(s.task_key,'not set')
       || E'\\nMessage from leader agent: ' || COALESCE(m.author_agent_id::text,'unknown') || E'\\n\\n' || m.body
      ELSE m.body END
     AND r.run_role=NEW.run_role AND r.state='pending' AND r.runtime_run_id IS NULL
     AND r.runtime_session_id=NEW.requested_session_id
     AND (NEW.request_body::jsonb->'model') IS NOT DISTINCT FROM to_jsonb(r.model)
     AND (NEW.request_body::jsonb->'provider') IS NOT DISTINCT FROM to_jsonb(r.provider)
     AND (NEW.request_body::jsonb->'model_options') IS NOT DISTINCT FROM
      CASE WHEN r.model_options='{}'::jsonb THEN NULL ELSE r.model_options END
     AND m.message_kind IN ('user_prompt','control') AND m.delivery_state='pending'
     AND m.runtime_message_id IS NULL AND o.state IN ('dispatching','uncertain'))
   OR EXISTS(SELECT 1 FROM task_chat_bindings WHERE session_id=NEW.session_id)
   OR EXISTS(SELECT 1 FROM pm_run_bindings WHERE session_run_id=NEW.run_id) THEN
   RAISE EXCEPTION 'Invalid free-chat Hermes dispatch journal' USING ERRCODE='23514';
  END IF;
 ELSE
  IF (to_jsonb(NEW)-'state'-'submitted_at'-'accepted_at') IS DISTINCT FROM
      (to_jsonb(OLD)-'state'-'submitted_at'-'accepted_at')
   OR (OLD.submitted_at IS NOT NULL AND NEW.submitted_at IS DISTINCT FROM OLD.submitted_at)
   OR (OLD.accepted_at IS NOT NULL AND NEW.accepted_at IS DISTINCT FROM OLD.accepted_at)
   OR (NEW.state IS DISTINCT FROM OLD.state AND NOT
       ((OLD.state='prepared' AND NEW.state='submitted') OR (OLD.state='submitted' AND NEW.state='accepted')))
   OR (NEW.state=OLD.state AND NEW IS DISTINCT FROM OLD) THEN
   RAISE EXCEPTION 'Hermes dispatch identity and progress are immutable' USING ERRCODE='23514';
  END IF;
  IF OLD.state='prepared' AND NEW.state='submitted' THEN
   NEW.submitted_at:=clock_timestamp();
   IF NEW.submitted_at>=OLD.recovery_deadline THEN
    RAISE EXCEPTION 'Hermes submission horizon expired' USING ERRCODE='23514';
   END IF;
  END IF;
  IF OLD.state='submitted' AND NEW.state='accepted' THEN
   IF NOT EXISTS(SELECT 1 FROM session_agent_runs r JOIN session_messages m ON m.id=NEW.message_id
      JOIN message_dispatch_outbox o ON o.message_id=m.id
      WHERE r.id=NEW.run_id AND r.runtime_run_id IS NOT NULL
       AND m.runtime_message_id=r.runtime_run_id AND m.delivery_state IN ('dispatched','completed','failed')
       AND o.state='dispatched') THEN
    RAISE EXCEPTION 'Hermes journal acceptance requires persisted ACK' USING ERRCODE='23514';
   END IF;
   NEW.accepted_at:=clock_timestamp();
  END IF;
 END IF;
 RETURN NEW;
END;
$$ LANGUAGE plpgsql;";
