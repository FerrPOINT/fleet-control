# Threat Model

Primary risks:

- accidental sharing of one `HERMES_HOME` between agents
- path traversal from workspace/config inputs
- leaking model/API tokens through env previews or logs
- stale process status after backend restart
- starting Java Agent before its isolation policy is complete
- showing private executor sessions to a leader without explicit selection
- allowing a leader to write into an executor session outside its team binding
- ordinary user expanding session filters to other users
- duplicate prompt delivery during browser retries
- role/settings/deployment changes without audit evidence
- direct mutation of Hermes SessionDB instead of using the runtime boundary

Controls:

- sequential DB identity with unique paths
- guarded path helpers and folder markers
- per-agent runtime API keys derived from
  `FLEET_CONTROL_FLEET__RUNTIME_TOKEN_SECRET`
- redaction at write time
- status reconciliation through health checks
- Java Agent adapter disabled for launch in MVP
- backend current-user filtering by default
- admin/operator-only expansion to all/multiple users
- backend RBAC for all fleet infrastructure routes
- `leader_executors` validation before leader assignment
- idempotency keys and payload hashes for session/message create
- deployment/settings/role changes recorded in `audit_log`
- `session_messages` as Fleet mirror and runtime supervisor dispatch boundary
