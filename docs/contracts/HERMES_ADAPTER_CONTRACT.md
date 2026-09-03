# Hermes Adapter Contract

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
