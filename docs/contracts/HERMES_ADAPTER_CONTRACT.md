# Hermes Adapter Contract

## PM Integration Candidate

PM consumes unchanged Hermes through the same native run/control protocol and
shared stream/recovery path. A task-bound steer is guidance for the already
accepted run, not a Tracker clarification answer or another model run. Stop
acknowledgement retains capacity until exact native terminal status and the
committed Fleet mirror agree. Original-key control replay must not resend.

Saved-answer delivery and PM continuation have independent durable states.
Continuation needs the original answer/checkpoint, prior-run terminal proof,
verified Workflow rebind and exact native acceptance; EOF, unavailable runtime
or a successful Tracker answer cannot stand in for these receipts. Idle PM
free-form launch remains unsupported under the inspected Workflow contract.
This source integration is not native/live acceptance; no Hermes patch or
custom pre-model handshake is required.

## Original-Key And Pinned Recovery Candidate

The default-off [recovery extension](HERMES_RECOVERY_V1.md) freezes verified
original store/scope/source facts before submission and performs only a
non-dispatch original-key lookup after unknown acceptance. No repeated run POST,
legacy backfill, new key, task/PM model admission or capacity release follows a
negative result. Pinned active runs recover with authenticated status GET only.
Exact terminal run/session proof commits mirror/delivery/state in one transaction;
replay is read-only, contradictions fail. [Stream bounds](HERMES_EVENT_STREAM_V1.md)
require a complete frame or independent terminal status at EOF. Native producer
and managed-runtime compatibility are not yet accepted for this candidate.

## Current Approval Recovery Candidate

Authenticated GET of the original accepted native run/session may restore its
current approval.request snapshot with exact request_id and bounded once/deny
choices. No historical queue, unknown-run lookup, SSE reconnect or resubmission
is authorized. Exact-target decision preflight verifies current original scope,
capability and pending action before one POST with resolve_all:false. Lost ACK
stays uncertain across restart/replay; GET cannot infer delivered or retry it.
Known terminal GET uses the inherited atomic mirror. Source-only; real native
process compatibility and exact Linux acceptance remain pending.

## Steer Transcript Follow-Up

Steer ACK persistence requires the exact originally hashed input and atomically
stores its redacted Fleet transcript mirror. Native POST remains single-use.
Acknowledged replay can repair a historical missing mirror from the exact payload;
GET recovery cannot recover discarded text or infer guidance acceptance.
An ACK whose DB transaction fails remains submitted/held without transcript
delivery or another POST. No native protocol or installed compatibility is claimed.

## Durable Runtime Controls Unit13

The Fleet-local controls/lookup GET reads a command's original actor/key receipt
without contacting Hermes. Missing/404, an old server or a failed read retains
the unknown-effect hold. The existing Hermes protocol and submission permit are
unchanged; this lookup is not native acceptance or permission to send again.

Controls require fresh exact original origin/credential/native run/session,
verified POST capabilities and authenticated current run GET. Send one POST
only after the durable submitted claim; validate bounded exact ACK shape,
run identity, status, boolean acceptance, MIME and encoding. Invalid/lost ACK
retains submitted/uncertain hold without retry. Stopping acknowledgement is not
terminal proof. Independent terminal mirror commits prompt/run/optional answer
and events atomically; scoped control GETs never reconnect or dispatch.
Native installed compatibility remains separate from this source candidate.
The combined candidate restores pinned active-run GET-only recovery after Fleet
restart through the recovery slice; fixtures are not installed-runtime acceptance.

The acceptance readback worker finishes the keyset scan before its five-second
idle poll. Nonempty pages yield cooperatively without adding one idle interval
per page; an empty page resets the cursor, and an empty page or queue-read failure
keeps the idle delay. Each original run is still checked through the existing
identity, credential, ownership, accepted-context and terminal-proof fences.
This scheduling change grants no POST/redispatch or capacity-release authority.
Its multi-page regression is not yet PostgreSQL-qualified for the new source.

## Docker Activation Restart Recovery

The [Base4 opt-in consumer](../RECOVERED_ACTIVATION_CONSUMER.md) requires
proved original custody, including sequential revisions over committed/rolled-
back effective children. New plans preserve root authority and predecessor
receipts/credentials; failed-next rollback restores current effective after
physical candidate exit and full readiness. Sealed original Docker activation
can also continue under that same proved recovered custody. Original API
credentials, image, isolated layout, command IDs and receipts stay fixed; agents
receive no Docker socket or controller evidence. Physical/file/API readiness is
not loaded-model/tool self-attestation or production admission. Historical Base169
recovery limits below do not describe the additional protocol4 capability.

Docker config18 [restart reconciliation](../CONTAINER_ACTIVATION_RECOVERY_RELEASE.md)
preserves the original private plan, recipe, credentials and native IDs. An
interrupted pre-plan claim is discoverable without clearing/reclaiming its queue
timestamp. Foreign/recovered custody permits only diagnostic original readback
here, not prepare/start/stop, config writes or effective publication. Typed held
and audited recovery action replace silent stranding, never runtime readiness.

Docker config18 [preflight fixes](../CONTAINER_ACTIVATION_PREFLIGHT_FIX.md) bound
all rendered targets before sealing/stop and retry transient original read-only
observe/health failures under the same live custodian. Unknown lifecycle effects
and sealed evidence retain their original holds, not a new preparation permit.

The configured Docker path can activate a revision on a fresh original generation
under live original custody. Failed bounded API readiness requires candidate exit
proof before exact previous-file restoration and fresh rollback preparation.
Only original Base readback derives endpoints. See
[unit18 source contract](../CONTAINER_ACTIVATION_RELEASE.md); health is not admission.
The [standalone integration](../CONTAINER_ACTIVATION_INTEGRATION.md) uses canonical
Base Unicode JSON hashing for config and generation intent as well as mapping.
Sibling activation/readiness work cannot queue recovery heartbeats or lifecycle
operations behind a global agent-operation lock. Same-agent custody stays exclusive.

[Unit17](../AUTOMATIC_CONTAINER_PREPARATION_RELEASE.md) automates configured first
generation preparation and original unknown readback; operator-prepared input is
only the legacy path. Native acceptance and replacement remain pending.

Unit16 supports original mapped v3 custody and same-container physical controller
restart recovery without changing original run origins, credentials or POST
permits. Automatic generation preparation/replacement and config activation
remain pending. See [the unit contract](../MAPPED_CONTROLLER_RECOVERY_RELEASE.md).
Canonical Git utility bytes and Base ASCII mapping hashes are mandatory;
sibling native delays do not serialize renewals, and an unknown foreign-owner
heartbeat remains held without mutation. No journal/POST permit is reopened.

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

## Configuration Foundation Candidate

Package-marked snapshots use the pinned Base role instruction and allowlist.
Read-only HOME verification rechecks Git, expected managed bytes and the closed
flat SKILL.md inventory, refusing unattested support files/aliases without
deleting them. Legacy non-package verification remains managed-only.
Project/external/plugin discovery, loaded model/tools/limits and physical adapter
acceptance are separate pending gates. No configuration observation authorizes
dispatch; `runtime_ready` stays false. Workflow owner preflight precedes apply.
