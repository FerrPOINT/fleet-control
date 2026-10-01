# Runtime

Runtime health is not SDLC readiness. Fleet returns
`workflow_assignment_protocol_not_verified` until the assignment, workflow
step/rebind and receipt protocols are verified. See
[SDLC implementation](SDLC_IMPLEMENTATION.md) for remaining runtime gates.

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
- Prompt outbox is transactional. Unknown POST acceptance is not automatically
  retried; the agent remains occupied pending reconciliation.
- Stream EOF is not completion. Fleet requires a terminal event or terminal
  status readback, and deduplicates the final mirror response.
- Configuration is draft/validated/activating/active/failed with desired and
  effective revisions. Activation drains runs and checks files/runtime before
  releasing the agent. Failed rollback keeps the agent drained.

Java Agent:

- Existing externally provisioned Java jar lifecycle is retained. Chat/control
  and configuration activation are phase 2 and cannot enter automatic SDLC.
- Reserved fields: `AGENT_SERVER_PORT`,
  `SPRING_CONFIG_ADDITIONAL_LOCATION`, `/actuator/health`,
  `/api/v1/agent/chat/stream`, `/api/v2/sessions`, `/v1/capabilities`.
- Start requires the managed `runtime/backend.jar`, JDK command and db-only
  `/actuator/health/readiness` health. Missing jar fails validation; no fake
  successful chat/control is returned.
