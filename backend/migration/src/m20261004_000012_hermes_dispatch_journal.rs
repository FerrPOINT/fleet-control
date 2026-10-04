use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE hermes_dispatch_journal (
                message_id uuid PRIMARY KEY REFERENCES session_messages(id),
                run_id uuid NOT NULL UNIQUE REFERENCES session_agent_runs(id),
                session_id uuid NOT NULL REFERENCES agent_sessions(id),
                agent_id uuid NOT NULL REFERENCES agents(id),
                run_role text NOT NULL CHECK (run_role IN ('primary','leader')),
                requested_session_id text NOT NULL,
                request_body text NOT NULL CHECK (octet_length(request_body) BETWEEN 1 AND 1048576),
                request_hash text NOT NULL CHECK (request_hash ~ '^[0-9a-f]{64}$'),
                idempotency_key text NOT NULL UNIQUE CHECK (idempotency_key = message_id::text),
                origin text NOT NULL CHECK (origin ~ '^http://127[.]0[.]0[.]1:[0-9]{4,5}$'),
                credential_fingerprint text NOT NULL CHECK (credential_fingerprint ~ '^[0-9a-f]{64}$'),
                capabilities jsonb NOT NULL CHECK (jsonb_typeof(capabilities) = 'object' AND octet_length(capabilities::text) <= 262144),
                retention_seconds integer NOT NULL DEFAULT 86400 CHECK (retention_seconds = 86400),
                created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
                recovery_deadline timestamptz NOT NULL,
                state text NOT NULL DEFAULT 'prepared' CHECK (state IN ('prepared','submitted','accepted')),
                submitted_at timestamptz,
                accepted_at timestamptz,
                CHECK (recovery_deadline = created_at + interval '86340 seconds'),
                CHECK ((state = 'prepared' AND submitted_at IS NULL AND accepted_at IS NULL)
                    OR (state = 'submitted' AND submitted_at IS NOT NULL AND accepted_at IS NULL)
                    OR (state = 'accepted' AND submitted_at IS NOT NULL AND accepted_at IS NOT NULL)),
                CHECK (submitted_at IS NULL OR submitted_at >= created_at),
                CHECK (accepted_at IS NULL OR accepted_at >= submitted_at),
                CHECK (request_hash = encode(sha256(convert_to(request_body,'UTF8')),'hex')),
                CHECK (requested_session_id = 'fleet:' || session_id::text || ':' || agent_id::text),
                CHECK ((capabilities #> '{features,runs_idempotency,supported}') IS NOT DISTINCT FROM 'true'::jsonb
                    AND (capabilities #> '{features,runs_idempotency,durable}') IS NOT DISTINCT FROM 'true'::jsonb
                    AND (capabilities #> '{features,runs_idempotency,retention_seconds}') IS NOT DISTINCT FROM '86400'::jsonb
                    AND (capabilities #>> '{features,runs_idempotency,retention_seconds}') IS NOT DISTINCT FROM '86400'
                    AND (capabilities->>'object') IS NOT DISTINCT FROM 'hermes.api_server.capabilities'
                    AND (capabilities->>'platform') IS NOT DISTINCT FROM 'hermes-agent'
                    AND (capabilities #>> '{auth,type}') IS NOT DISTINCT FROM 'bearer'
                    AND (capabilities #> '{auth,required}') IS NOT DISTINCT FROM 'true'::jsonb
                    AND (capabilities #>> '{runtime,mode}') IS NOT DISTINCT FROM 'server_agent'
                    AND (capabilities #>> '{runtime,tool_execution}') IS NOT DISTINCT FROM 'server'
                    AND (capabilities #> '{runtime,split_runtime}') IS NOT DISTINCT FROM 'false'::jsonb
                    AND (capabilities #> '{features,run_submission}') IS NOT DISTINCT FROM 'true'::jsonb
                    AND (capabilities #> '{features,run_status}') IS NOT DISTINCT FROM 'true'::jsonb
                    AND (capabilities #> '{features,run_events_sse}') IS NOT DISTINCT FROM 'true'::jsonb
                    AND (capabilities #> '{features,run_stop}') IS NOT DISTINCT FROM 'true'::jsonb
                    AND (capabilities #>> '{endpoints,runs,method}') IS NOT DISTINCT FROM 'POST'
                    AND (capabilities #>> '{endpoints,runs,path}') IS NOT DISTINCT FROM '/v1/runs'
                    AND (capabilities #>> '{endpoints,run_status,method}') IS NOT DISTINCT FROM 'GET'
                    AND (capabilities #>> '{endpoints,run_status,path}') IS NOT DISTINCT FROM '/v1/runs/{run_id}'
                    AND (capabilities #>> '{endpoints,run_events,method}') IS NOT DISTINCT FROM 'GET'
                    AND (capabilities #>> '{endpoints,run_events,path}') IS NOT DISTINCT FROM '/v1/runs/{run_id}/events'
                    AND (capabilities #>> '{endpoints,run_stop,method}') IS NOT DISTINCT FROM 'POST'
                    AND (capabilities #>> '{endpoints,run_stop,path}') IS NOT DISTINCT FROM '/v1/runs/{run_id}/stop')
             );
             CREATE INDEX hermes_dispatch_unresolved_idx ON hermes_dispatch_journal(state,created_at)
                WHERE state <> 'accepted';
             CREATE UNIQUE INDEX hermes_dispatch_agent_guard ON hermes_dispatch_journal(agent_id)
                WHERE state <> 'accepted';
             CREATE FUNCTION fleet_guard_hermes_dispatch() RETURNS trigger AS $$
             BEGIN
                IF TG_OP = 'DELETE' THEN
                    RAISE EXCEPTION 'Hermes dispatch journal cannot be deleted' USING ERRCODE='23514';
                END IF;
                IF TG_OP = 'INSERT' THEN
                    IF NEW.state <> 'prepared' OR NEW.submitted_at IS NOT NULL OR NEW.accepted_at IS NOT NULL
                       OR jsonb_typeof(NEW.request_body::jsonb) IS DISTINCT FROM 'object'
                       OR jsonb_typeof(NEW.request_body::jsonb->'input') IS DISTINCT FROM 'string'
                       OR NEW.request_body::jsonb->>'session_id' IS DISTINCT FROM NEW.requested_session_id
                       OR NEW.request_body::jsonb - 'input' - 'session_id' - 'model' - 'provider' - 'model_options' <> '{}'::jsonb
                       OR NOT EXISTS (SELECT 1 FROM session_agent_runs r JOIN agent_sessions s ON s.id=r.session_id
                            JOIN agents a ON a.id=r.agent_id
                            JOIN session_messages m ON m.id=NEW.message_id
                            JOIN message_dispatch_outbox o ON o.message_id=m.id
                            WHERE r.id=NEW.run_id AND r.session_id=NEW.session_id AND r.agent_id=NEW.agent_id
                              AND s.agent_id=NEW.agent_id AND m.session_id=NEW.session_id AND o.agent_id=NEW.agent_id
                              AND a.kind='hermes' AND a.status='running' AND a.archived_at IS NULL
                              AND a.api_port BETWEEN 1024 AND 65535 AND NEW.origin='http://127.0.0.1:' || a.api_port::text
                              AND NOT EXISTS (SELECT 1 FROM agent_config_heads WHERE agent_id=a.id AND draining)
                              AND NEW.request_body::jsonb->>'input' = CASE
                                  WHEN m.author_type='agent' AND m.author_agent_id IS DISTINCT FROM NEW.agent_id THEN
                                      E'[Fleet Control]\nSession: ' || s.title || E'\nTask: ' || COALESCE(s.task_key,'not set')
                                      || E'\nMessage from leader agent: ' || COALESCE(m.author_agent_id::text,'unknown') || E'\n\n' || m.body
                                  ELSE m.body END
                              AND r.run_role=NEW.run_role AND r.state='pending' AND r.runtime_run_id IS NULL
                              AND r.runtime_session_id=NEW.requested_session_id
                              AND (NEW.request_body::jsonb->'model') IS NOT DISTINCT FROM to_jsonb(r.model)
                              AND (NEW.request_body::jsonb->'provider') IS NOT DISTINCT FROM to_jsonb(r.provider)
                              AND (NEW.request_body::jsonb->'model_options') IS NOT DISTINCT FROM
                                  CASE WHEN r.model_options='{}'::jsonb THEN NULL ELSE r.model_options END
                              AND m.message_kind IN ('user_prompt','control') AND m.delivery_state='pending'
                              AND m.runtime_message_id IS NULL AND o.state IN ('dispatching','uncertain'))
                       OR EXISTS (SELECT 1 FROM task_chat_bindings WHERE session_id=NEW.session_id)
                       OR EXISTS (SELECT 1 FROM pm_run_bindings WHERE session_run_id=NEW.run_id) THEN
                        RAISE EXCEPTION 'Invalid free-chat Hermes dispatch journal' USING ERRCODE='23514';
                    END IF;
                ELSE
                    IF (to_jsonb(NEW) - 'state' - 'submitted_at' - 'accepted_at') IS DISTINCT FROM
                       (to_jsonb(OLD) - 'state' - 'submitted_at' - 'accepted_at')
                       OR (OLD.submitted_at IS NOT NULL AND NEW.submitted_at IS DISTINCT FROM OLD.submitted_at)
                       OR (OLD.accepted_at IS NOT NULL AND NEW.accepted_at IS DISTINCT FROM OLD.accepted_at)
                       OR (NEW.state IS DISTINCT FROM OLD.state AND NOT
                           ((OLD.state='prepared' AND NEW.state='submitted') OR (OLD.state='submitted' AND NEW.state='accepted')))
                       OR (NEW.state=OLD.state AND NEW IS DISTINCT FROM OLD) THEN
                        RAISE EXCEPTION 'Hermes dispatch identity and progress are immutable' USING ERRCODE='23514';
                    END IF;
                    IF OLD.state='prepared' AND NEW.state='submitted' THEN
                        NEW.submitted_at := clock_timestamp();
                        IF NEW.submitted_at >= OLD.recovery_deadline THEN
                            RAISE EXCEPTION 'Hermes submission horizon expired' USING ERRCODE='23514';
                        END IF;
                    END IF;
                    IF OLD.state='submitted' AND NEW.state='accepted' THEN
                        IF NOT EXISTS (SELECT 1 FROM session_agent_runs r JOIN session_messages m ON m.id=NEW.message_id
                            JOIN message_dispatch_outbox o ON o.message_id=m.id
                            WHERE r.id=NEW.run_id AND r.runtime_run_id IS NOT NULL
                              AND m.runtime_message_id=r.runtime_run_id AND m.delivery_state IN ('dispatched','completed','failed')
                              AND o.state='dispatched') THEN
                            RAISE EXCEPTION 'Hermes journal acceptance requires persisted ACK' USING ERRCODE='23514';
                        END IF;
                        NEW.accepted_at := clock_timestamp();
                    END IF;
                END IF;
                RETURN NEW;
             END;
             $$ LANGUAGE plpgsql;
             CREATE TRIGGER fleet_hermes_dispatch_guard BEFORE INSERT OR UPDATE OR DELETE ON hermes_dispatch_journal
                FOR EACH ROW EXECUTE FUNCTION fleet_guard_hermes_dispatch();"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "LOCK TABLE hermes_dispatch_journal IN ACCESS EXCLUSIVE MODE;
             DO $$ BEGIN
                IF EXISTS (SELECT 1 FROM hermes_dispatch_journal) THEN
                    RAISE EXCEPTION 'Hermes journals require explicit reconciliation before downgrade' USING ERRCODE='23514';
                END IF;
             END $$;
             DROP TABLE hermes_dispatch_journal;
             DROP FUNCTION fleet_guard_hermes_dispatch();"
        ).await?;
        Ok(())
    }
}
