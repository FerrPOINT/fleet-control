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
- Effective configuration readiness now reads the actual managed files on each
  request using the persisted effective head, independently of the last-100
  configuration history page. Validate/activate also use exact agent/revision
  lookup; only the current desired validated revision can activate. It compares
  `config.yaml`, `SOUL.md`, `.env`, enabled/disabled skills
  and the revision marker against the persisted effective snapshot, not against
  hashes or paths provided by a marker. Missing/changed managed files,
  re-enabled disabled skills, foreign markers and symlink/junction paths fail closed.
  Pinned Base revisions additionally verify the actual Git package/snapshot and
  closed HOME skill inventory, including nested/unlisted skill files and special
  entries (4096 entries / 16 levels maximum). No unexpected files are removed.
  Legacy revisions keep managed-only behavior; `.bundled_manifest` is never
  authority. Plugin/project/external discovery and loaded model/tool settings
  remain outside this observation; the separate
  `runtime_skill_inventory_not_verified` blocker remains until real inventory
  and native provenance are integrated.
  Verification does not create or repair directories/files. Secret values and
  hashes are not returned. This observation is not a fenced admission, proof of
  runtime-loaded configuration, or task-specific deployment workspace receipt.

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
