CREATE TABLE session_execution_contexts (
 session_id uuid PRIMARY KEY REFERENCES agent_sessions(id) ON DELETE RESTRICT,
 operation_id uuid NOT NULL UNIQUE,
 context jsonb NOT NULL,
 verified_projection jsonb NOT NULL,
 created_by_user_id uuid NOT NULL REFERENCES users(id),
 created_at timestamptz NOT NULL DEFAULT now(),
 CHECK(COALESCE((context->>'schema_version')::integer=2,false)),
 CHECK(COALESCE((verified_projection->>'runtime_ready')::boolean=false,false)),
 CHECK(COALESCE((verified_projection->>'dispatch_allowed')::boolean=false,false))
);
-- Context v2 is stored independently of profiles and strict SDLC v1 packets.
-- A future agreed adapter must explicitly enable execution; UI cannot bypass this gate.
CREATE FUNCTION guard_namespace_session_start() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE session uuid;
BEGIN
 IF TG_TABLE_NAME='session_messages' THEN
  session:=NEW.session_id;
  IF NEW.message_kind<>'user_prompt' THEN RETURN NEW; END IF;
 ELSIF TG_TABLE_NAME='session_agent_runs' THEN
  session:=NEW.session_id;
  IF NEW.state NOT IN ('pending','running','waiting','stopping') OR (TG_OP='UPDATE' AND OLD.state IN ('running','waiting','stopping')) THEN RETURN NEW; END IF;
 ELSE
  SELECT m.session_id INTO session FROM session_messages m WHERE m.id=NEW.message_id;
 END IF;
 PERFORM id FROM agent_sessions WHERE id=session FOR UPDATE;
 IF EXISTS(SELECT 1 FROM session_execution_contexts c WHERE c.session_id=session) THEN
  RAISE EXCEPTION 'namespace_execution_adapter_not_enabled' USING ERRCODE='55000';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER namespace_session_prompt BEFORE INSERT ON session_messages FOR EACH ROW EXECUTE FUNCTION guard_namespace_session_start();
CREATE TRIGGER namespace_session_dispatch BEFORE INSERT OR UPDATE ON message_dispatch_outbox FOR EACH ROW EXECUTE FUNCTION guard_namespace_session_start();
CREATE TRIGGER namespace_session_run BEFORE INSERT OR UPDATE ON session_agent_runs FOR EACH ROW EXECUTE FUNCTION guard_namespace_session_start();
