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
- When the Hermes API is unreachable and this supervisor has no tracked child,
  health marks the runtime stopped while retaining its desired state. An agent
  whose desired state is running is then restarted by the reconciler. A tracked
  process with an unhealthy API remains degraded and is probed again.
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

PM continuation proof is separate from runtime health and business completion.
The internal Workflow callback performs a fresh bounded/no-redirect HTTP probe
against the managed agent port, with Fleet's derived per-agent credential. It
verifies the acknowledged Hermes run and effective session identity. Queued,
running, approval-wait and stopping are non-terminal; interrupted is failed, not
successful or safely stopped. Terminal proof is immutable. Missing acceptance
mapping or inaccessible runtime blocks continuation; neither an SSE EOF nor a
database status can substitute for the probe.

Fresh terminal proof reconciles the matching Fleet run in the same transaction,
releasing runtime capacity even when the event stream was lost. A delayed EOF or
cached running/waiting event cannot reopen the old run; unknown acceptance still
holds the agent slot. A generic terminal cache update without verified PM proof
is rejected. Transcript finalization and Tracker stage completion remain separate.
