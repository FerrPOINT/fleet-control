# Hermes Adapter Contract

## Current Approval Recovery Candidate

Authenticated GET of the original accepted native run/session may restore its
current approval.request snapshot with exact request_id and bounded once/deny
choices. No historical queue, unknown-run lookup, SSE reconnect or resubmission
is authorized. Exact-target decision preflight verifies current original scope,
capability and pending action before one POST with resolve_all:false. Lost ACK
stays uncertain across restart/replay; GET cannot infer delivered or retry it.
Known terminal GET uses the inherited atomic mirror. Source-only; real native
process compatibility and exact Linux acceptance remain pending.

## Durable Runtime Controls Unit13

Controls require fresh exact original origin/credential/native run/session,
verified POST capabilities and authenticated current run GET. Send one POST
only after the durable submitted claim; validate bounded exact ACK shape,
run identity, status, boolean acceptance, MIME and encoding. Invalid/lost ACK
retains submitted/uncertain hold without retry. Stopping acknowledgement is not
terminal proof. Independent terminal mirror commits prompt/run/optional answer
and events atomically; scoped control GETs never reconnect or dispatch.
Native installed compatibility and active-run worker recovery after Fleet
process restart remain separate from this source candidate.

## Hermes Journal Release Unit12

Free-chat dispatch requires the exact authenticated server-agent capabilities,
durable run idempotency with 86400-second retention and verified run/status/SSE/
stop endpoint definitions. Persist the exact serialized request before claiming
one submission permit; POST those bytes once with the original message UUID.
Commit the native ACK atomically before reading the effective session. Recovery
never repeats POST, changes origin/credential context or synthesizes legacy
journal entries. Invalid terminal run/session evidence retains capacity.

Hermes is the first implemented runtime.

Environment:

- `HERMES_HOME=agentN/config`
- `API_SERVER_ENABLED=true`
- `API_SERVER_KEY=<derived per-agent token>`
- `HERMES_SERVE_HEADLESS=1`

Working directory:

- `agentN/workspace`

Managed files:

- `config/config.yaml`
- `config/SOUL.md`
- `config/.env`
- `config/skills`
- `runtime/source.json`

Lifecycle:

- start: configured Hermes command with `serve --host 127.0.0.1 --port <api_port>`
- stop: terminate tracked process
- restart: stop then start
- health: reconcile tracked process state through `/health`
- readiness: `/health` plus `/v1/capabilities` containing `run_status`,
  `run_events_sse` and `run_stop`

Native stdout/stderr writes use the shared repository's exact persisted-row
acknowledgement after redaction. See [Logging Standards](../LOGGING_STANDARDS.md).
This does not implement production Docker collection, replay or secret snapshots.

Session control:

- Fleet stores transcript/control mirrors in `session_messages`.
- Fleet dispatches through the runtime supervisor boundary.
- Fleet creates Hermes runs with `POST /v1/runs`, `input` and
  `session_id=fleet:<session_id>:<agent_id>`.
- Fleet mirrors events from `GET /v1/runs/{run_id}/events`.
- Fleet forwards run controls to `/v1/runs/{run_id}/steer`,
  `/v1/runs/{run_id}/stop` and `/v1/runs/{run_id}/approval`.
- For executor sessions, the runtime dispatch target is the primary executor
  even when the mirrored message author is the selected leader.
- Fleet must not write directly into Hermes SessionDB.
- Hermes serve/JSON-RPC is the intended programmatic chat surface.
- Dashboard remains a separate UI surface and is not the source of truth for
  Fleet message writes.
