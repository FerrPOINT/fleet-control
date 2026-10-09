# Hermes Adapter Contract

[Unit17](../AUTOMATIC_CONTAINER_PREPARATION_RELEASE.md) automates configured first
generation preparation and original unknown readback; operator-prepared input is
only the legacy path. Native acceptance and replacement remain pending.

Unit16 supports original mapped v3 custody and same-container physical controller
restart recovery without changing original run origins, credentials or POST
permits. Automatic generation preparation/replacement and config activation
remain pending. See [the unit contract](../MAPPED_CONTROLLER_RECOVERY_RELEASE.md).

Docker opt-in starts only an operator-prepared original Base v2 container with
isolated `/config` HOME/HERMES_HOME, `/workspace` cwd and four guarded agent areas.
Free-chat journal capabilities seal the original container generation and origin;
readback/control cannot adopt another generation. Configuration replacement and
container log collection remain unavailable. See
[bounded Docker contract](../DOCKER_LIFECYCLE_RELEASE.md).

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
