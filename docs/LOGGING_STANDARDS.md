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

- API requests pass through the `shared::telemetry` middleware, which mirrors
  the `services-base` `sdlc-telemetry` request-id contract, and return an
  `x-request-id` header.
- Runtime process logs must include agent id/name and stream.
- Failed provisioning/start/stop actions write both event and audit entries.
- UI `/logs` separates process logs, events and audit trail.
- `SDLC_LOG_JSON=true` enables the fleet JSON log format; otherwise Fleet
  Control uses the compact local format.
