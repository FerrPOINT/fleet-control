# Runtime

Runtime contract:

- `provision`
- `start`
- `stop`
- `restart`
- `health`
- `open_session`
- `send_message`
- `stream_events`
- `list_capabilities`

Hermes:

- Folders: `runtime`, `config`, `workspace`, `logs`.
- Env: `HERMES_HOME=<agents_root>/agentN/config`,
  `API_SERVER_ENABLED=true`, derived per-agent `API_SERVER_KEY`.
- Cwd: `<agents_root>/agentN/workspace`.
- Programmatic surface: `hermes serve --host 127.0.0.1 --port <api_port>`.
- Readiness requires `/health` and `/v1/capabilities` with `run_status`,
  `run_events_sse` and `run_stop`.
- Message dispatch uses `POST /v1/runs`, Fleet session ids formatted as
  `fleet:<session_id>:<agent_id>`, and `GET /v1/runs/{run_id}/events` for SSE
  mirror updates.
- Runtime controls use `/steer`, `/stop` and `/approval` endpoints when the
  capability matrix allows them.
- Dashboard is an operator link, not the write channel for messages.

Java Agent:

- Phase 2 runtime.
- Reserved fields: `AGENT_SERVER_PORT`,
  `SPRING_CONFIG_ADDITIONAL_LOCATION`, `/actuator/health`,
  `/api/v1/agent/chat/stream`, `/api/v2/sessions`, `/v1/capabilities`.
- Provision/start/chat operations return typed `not_implemented` until adapter
  implementation is complete.
