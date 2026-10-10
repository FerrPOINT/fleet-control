# Runtime

## Recovery Candidate Boundary

Default-off `fleet.container_control.recovered_activation` enables the
[Base4 original-plan consumer](RECOVERED_ACTIVATION_CONSUMER.md). It retains the
original activation/commands/receipts, native recovered anchor and run fences.
Only supported proved phases continue; missing evidence/unknown old effects hold.
Successful readiness/publication is not SDLC admission or native acceptance.
Sequential desired revisions can use the proved effective committed/rolled-back
child, preserving the root journal/authority and exact predecessor lineage.
Failed-next rollback restores current effective, not obsolete root config; expiry,
unknown effects or missing original proofs still hold before new native permits.

Free-chat unknown acceptance may recover only the original native run ID through
the default-off [durable witness lookup](contracts/HERMES_RECOVERY_V1.md).
Original bytes/key/hash/origin/credential/scope/store epoch must match. Known-ID
pending/running/waiting/stopping runs recover by authenticated status GET; an
already pinned run never attaches a replacement SSE consumer. Terminal evidence
must match its run/session and commit atomically before capacity is released.
The [bounded stream profile](contracts/HERMES_EVENT_STREAM_V1.md) discards an
unterminated EOF frame and requires independent status proof. It does not replay
missed tools/approvals, prove safe process stop or authorize task/PM execution.
Native compatibility and exact-source acceptance remain pending.
Docker config activation now has a bounded live-custodian fresh-generation path;
see [unit18 source contract](CONTAINER_ACTIVATION_RELEASE.md). Desired/effective
publication, drain, original safe-stop and exact rollback remain separate gates.
Native acceptance and production admission are still pending.

The [standalone integration](CONTAINER_ACTIVATION_INTEGRATION.md) retains the
unit16 ownership/heartbeat guards with unit17 preparation and unit18 activation.
Reconcile, recovery renewal and Docker activation have independent per-agent
workers. Docker lifecycle and activation serialize only the same agent; recovery
renewal never waits for an activation readiness poll. Drain and proof gates remain.

Opt-in original Docker lifecycle is available as a bounded source release. It
uses Base utility169 separately from the unchanged SDK pin, one container journal
migration and private operator-prepared v2 containers. See
[Docker lifecycle release](DOCKER_LIFECYCLE_RELEASE.md) for exact dependencies,
physical readiness/stop gates, holds and remaining acceptance gaps.

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
  request. It compares `config.yaml`, `SOUL.md`, `.env`, enabled/disabled skills
  and the revision marker against the persisted effective snapshot, not against
  hashes or paths provided by a marker. Missing/changed managed files,
  re-enabled disabled skills, foreign markers and symlink/junction paths fail closed.
  Hermes-owned categories and `.bundled_manifest` are preserved and not used as
  authority. This check does not attest extra/runtime-owned skills; the separate
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

## Configuration Foundation Candidate

The [bounded release unit](plans/2026-10-09-runtime-config-release.md) adds pinned
package drafts and Workflow owner checks to the existing configuration lifecycle,
not a second installer. Supervisor owner preflight runs before apply; failed
preflight preserves old files/head and releases drain. Unknown rollback retains
the existing drain guard. Exact managed-file/role-package readback is not native
runtime inventory or business completion; machine `runtime_ready=false`.
Container lifecycle, endpoint attach/readback, native-context and later runtime
schema/migration changes are not included.
