-- Keep v2 drafts inert even while an owner ACK is missing.
CREATE OR REPLACE FUNCTION guard_namespace_session_start() RETURNS trigger LANGUAGE plpgsql AS $$
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
 IF EXISTS(SELECT 1 FROM agent_sessions s WHERE s.id=session AND s.idempotency_key LIKE 'namespace-v2:%') OR EXISTS(SELECT 1 FROM session_execution_contexts c WHERE c.session_id=session) THEN
  RAISE EXCEPTION 'namespace_execution_adapter_not_enabled' USING ERRCODE='55000';
 END IF;
 RETURN NEW;
END $$;
