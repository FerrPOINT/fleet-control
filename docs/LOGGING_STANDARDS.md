# Logging Standards

Log classes:

- process output from runtime agents in `agent_logs`
- control-plane events in `agent_events`
- security and operator actions in `audit_log`
- backend application logs to stdout/stderr for deployment collection

Redaction:

- redact tokens, passwords, API keys, cookies and secret env values before
  persistence
- audit message writes with metadata rather than duplicating full prompt bodies
- settings responses return redacted secret placeholders

Operational expectations:

- Runtime process logs must include agent id/name and stream.
- Failed provisioning/start/stop actions write both event and audit entries.
- UI `/logs` separates process logs, events and audit trail.
