# Runtime

PM credential preparation now verifies the closed Tracker lease readback before
returning its memory-only child credential. Active/expired or inconsistent
evidence holds; no lease mutation, Workflow bind or model call follows this
primitive. [The contract](contracts/PM_EXECUTION_LEASE_READBACK_V1.md) keeps
historical receipts separate from current renewal and SDLC readiness.

## Configuration Activation Process Lock

Before planning backups or performing configuration effects, the Linux
activator acquires an exclusive nonblocking `flock` on a controller-private
`<agent-uuid>.activation.lock`. The open descriptor is retained by the journal
through runtime/file work, database settlement and journal acknowledgement.
The zero-byte lock file is mode0600, owned by the controller UID and single-link;
its descriptor and named inode must agree. Symlinks, altered permissions,
content or inode replacement hold the agent instead of admitting another writer.
The existing lifecycle mutex still serializes work within one supervisor.

The lock inode is never removed or replaced. Process death releases the OS lock,
not the sensitive recovery journal or the database drain. All controllers for
the same managed filesystem must use the same protected controller directory;
this does not certify distributed/NFS locking or authorize relocating storage.

Rollback first verifies the exact persisted journal and decodes its original
backup entries; a missing/changed document cannot fall back to in-memory bytes.
Ownership is checked before each file mutation and settlement. This is a
necessary recovery primitive, not proof of native acceptance or loaded readiness.
Windows activation remains fail-closed. No API, migration or deployment change.

## Signed Interrupted Configuration Recovery

New activation journals use authenticated v3 payloads with exact candidate and
previous effective snapshot hashes, original controller/launch and canonical
locations. A distinct default-off configuration recovery flag connects the
existing activator to the private loader; it acquires the same OS/lifecycle locks,
validates the HMAC and rendered plan, then reconciles the original operation.
Legacy v1/v2, changed bytes/roots/snapshots and rotated secrets remain held.

Stopped activations restore and durably read back original backup bytes/absence
before an atomic failed-revision settlement. That transaction rechecks claim,
head, snapshot hashes and absence of active runs or pending/uncertain dispatch,
preserves the effective revision and writes one digest-only audit. If persistence
fails, drain and journal remain; replay uses the same signed identity. A committed
candidate is only acknowledged after exact file and runtime readback, never
reapplied. Stopped agents do not acquire an implicit start.

For previously running agents, unknown candidate/foreign preparation or
unconfirmed process state prevents file restoration. Current-controller rollback
preparation readback is separately guarded below. Only the original or revision-bound replacement
container can be stopped through the existing custody controls. Restoration
requires original launch history and validated namespace-exit observation;
rollback uses a fresh generation and readiness before settlement. Native orphan
processes and recovered-owner degraded health cannot substitute for these proofs.
No unknown start/stop command is resent. The stopped recovery and transaction
boundaries have PostgreSQL regressions; actual candidate-running crash evidence
is scoped to one boundary, not complete loaded-config/recovery acceptance.
See [ADR0035](adr/0035-signed-configuration-recovery.md)
and [gap register](GAP_REGISTER.md#interrupted-configuration-activation).

## Original Preparation Readback

New preparation uses `prepare` exactly once after committing the immutable
database claim and saving the controller-private creation intent. Replay uses
the distinct Base `reconcile_preparation` action; it cannot create/start/kill a
container or repair missing native authority. Original process/env, source,
mapping, operation/generation, ordinal and configuration must still match.

Replay verifies the existing DB claim under the same agent/runtime/configuration
transaction locks; it never inserts a missing row. This applies both when only
the private creation intent survives and when `container-prepared.json` already
exists. A restored private receipt without its original DB claim cannot acquire
custody for the current or another controller. Missing documents, missing claim,
changed hash/configuration and foreign controller remain held without resubmit.

Interrupted activation may consume this readback only for a current-controller
rollback after exact backup bytes are restored and the original namespace exit
is confirmed. A candidate preparation or unknown start cannot use this path.
Only original Registered/NeverStarted observation permits the protected start;
readiness and atomic rollback settlement remain separate checks. No new runtime
generation is allocated for the lost preparation response. This does not authorize
task admission, restore an unknown runtime acceptance, or attest all loaded config.
The physical crash/ACK-loss gate and remaining limitations are recorded separately
in [verification](CHAT_CLARIFICATION_VERIFICATION.md).

## Namespace Exit And Chat State

The recovered-stop follow-up atomically cancels known accepted free-chat runs
with exact original launch/endpoint provenance. Pending approvals are cancelled
without a permission grant; prompt content and accepted dispatch history stay
unchanged. Existing durable events expose the terminal state to Chats. Replay
does not reopen or duplicate events. Legacy/unknown/task/PM runs and uncertain
native decisions remain held, not silently successful. See
[the recovery contract](contracts/CONTROLLER_RECOVERY_V1.md#generation-bound-free-chat-interruption).
This follow-up requires its own actual acceptance; the previous namespace-only
gate did not settle sessions and is not substituted as that evidence.

## Controller Recovery Storage

Candidate000020 retains separate immutable proposed-owner epochs under the same
agent-row lock, rather than replacing the original launch controller. DB-clock
leases and versioned heartbeats fence the storage saga; unknown reservations
stay held after expiry. Any recovery record fences original generation/queue/
permit/endpoint/lifecycle writes. A saved native receipt hash is not actual Base
ownership, so the new-owner effect path remains disabled. Candidate000021 connects
the trusted entry's private initial command, once-only dispatch and closed native
original-key readback/outcome transaction. Saving a historical ACK after expiry
does not renew the lease. An explicit default-off startup worker now connects
dual DB/native heartbeat on10-second ticks, with one in-flight cycle per agent.
Initial recovery has an8-second budget; dual heartbeat has20 seconds,
below the unchanged30-second lease. Acknowledged owners do not repeat the initial
handshake, and claimed commands use original-key readback without redispatch.
The current candidate requires Base `heartbeat_controller_live`: exact version
sync plus fresh physical/live lease verification in one closed native response.
Fleet uses two such calls around its DB CAS; historical ACK is not a fallback.
Graceful server shutdown/restart stops new custody cycles and waits for bounded
in-flight work through `quiesce_controller_recovery` (at most 42 seconds).
Concurrent callers share completion, and the same worker cannot be restarted.
This does not stop agents, grant effects or certify a live lease. Failed or
interrupted native delivery retains original-key reconciliation; forced OS exit
can still interrupt a cycle. The public HTTP surface is unchanged.
Lock admission and native work have separate20-second bounds inside a41-second
worker envelope. No lease is renewed while waiting; the admitted operation
rechecks current DB/native custody rather than trusting pre-lock observations.
It replays the last persisted heartbeat before extending PostgreSQL, checks live
native custody before and after extension, and never treats a historical ACK as
authority. Fresh new-owner effect checks and original run/configuration checkpoint
settlement remain required before restored execution or actual ongoing-Hermes
restart acceptance. See
[contract](contracts/CONTROLLER_RECOVERY_V1.md).

## Controller Restart Observation

The opt-in mapped container consumer can read Base's original-controller restart
witness and return degraded health without taking custody. It verifies the old
mapping/registration, distinct same-controller Engine start and fresh immutable
DB launch/ACK/PID. No runtime state, endpoint, capacity or configuration is changed;
no Hermes call is permitted. See
[private contract](contracts/CONTAINER_CONTROL_V1.md#original-controller-restart-observation).
This does not yet restore execution after Fleet restart. Durable fenced owner
transfer, interrupted activation and source/recreation/storage recovery require
their own implementation and actual supervisor acceptance.

## Container Endpoint Authority

Container HTTP origins are not user settings. After the original Base guard
verifies the running namespace, the owning supervisor seals its exact origin
against launch generation and PID in `runtime_launch_endpoints`. Exact readback
replays succeed; origin, PID or custody drift holds dispatch. Journal prepare and
claim join this record and private `fleet_launch` ID, while native free chats stay
localhost-only. No missing proof authorizes a host fallback or unknown prompt
resend. See [ADR 0030](adr/0030-generation-bound-container-endpoint.md).

The actual two-container controlled-model gate verifies basic chat, loaded
SOUL isolation, drain, fresh activation/restart and the separately opted-in
controlled readiness-timeout rollback. Controller restart/lost-private-journal
recovery and production log ingestion remain distinct open gates; task/PM
admission is still fail-closed. Details:
[acceptance runbook](CONTAINER_SUPERVISOR_ACCEPTANCE.md).

The new private `log_tail` client verifies the original acknowledged receipt and
two bounded binary streams. It is not a public logs DTO or production collector;
current launch custody and resolved-secret redaction are required before any
persistence. See [ADR0031](adr/0031-private-bounded-container-log-readback.md).

## Container Configuration Replacement Candidate

Configuration admission now runs before both create and start. A regular
preparation cannot bypass drain; activation/rollback bind claimed desired/prior
effective revisions and snapshot hashes under the database locks. The candidate
connects the private activation journal to original namespace stop, file
readback and fresh Docker generations. Stopped agents do not implicitly start.
Unknown creation/start/stop holds drain and candidate files; no rollback spawn
or native fallback is allowed until original custody is reconciled. See
[ADR0029](adr/0029-container-configuration-generation-replacement.md).
The original31 focused Linux/PostgreSQL cases and strict Clippy pass. Subsequent
actual mapped Hermes/model/config and controlled rollback evidence is recorded
in [verification](CHAT_CLARIFICATION_VERIFICATION.md). Earlier counts below
remain historical; they do not certify controller takeover or every failure mode.

## Container Preparation Custody

Additive000018 commits an immutable DB identity before the secret-bearing
intent file and Base physical create. The agent/history-ordinal/controller/
generation/operation/hash must replay exactly. Missing private storage cannot
grant another create or native fallback. Restoration requires the exact original
document and Base readback; a different controller remains held. Existing000017
launch/start authority and namespace-exit requirements remain unchanged.
See [ADR0028](adr/0028-durable-container-precreate-fence.md) and
[recovery operations](OPERATIONS.md#lost-container-preparation-files).

## Original-Controller Observation And Liveness

A replica without the original managed launch returns an observational degraded
health response without writing agent/runtime/journal metadata belonging to the
other controller. That observation cannot revoke the original controller's
pending delivery or heartbeat. It does not adopt the child or attest that the
old controller is dead; explicit reconciliation remains necessary.
The HTTP health action still records its audit, with observed and persisted
statuses. A nonpersisted observation cannot emit a shared health-transition
alert; only a response matching fresh stored state enters that alert path.

Managed generation verification now calls `try_wait` on the retained original
child before reading its PID. An unreaped exited process with a cached PID
cannot authorize prepared/actual dispatch. The check retains the launch and
child for normal exit reconciliation; it does not invent a namespace-empty
receipt, clear capacity or prove liveness after the observation. Host boundary,
loaded configuration and task/PM admission remain independent requirements.

## Pre-Spawn Launch Identity

The supervisor commits an immutable agent/configuration/controller launch in
PostgreSQL before native Hermes/Java spawn (additive `000017`). An unknown
launch cannot be replaced from Ready metadata or a healthy HTTP response. Failed
spawn acknowledgements retain child custody and block new dispatch/activation
until original committed-ACK readback with the same child succeeds. Runtime
metadata writes recheck outstanding ownership under the agent lock, preventing
a late exit write from clearing a newer gateway. New dispatch intents pin the
original launch ID and cannot follow a replacement gateway.
Activation and rollback pin their own exact source revisions. See the
[launch contract](contracts/RUNTIME_LAUNCH_JOURNAL_V1.md) and
[ADR](adr/0026-pre-spawn-runtime-launch-journal.md).

This records gateway launch/exit, not safe descendant stop, immutable loaded
configuration, a Base host-boundary receipt or SDLC admission. Existing native
exit/restart remains uncertified for those stronger guarantees.

The separate approval decision journal is now implemented through additive000016:
original mode is fixed at reservation, context/claim commit before any permitted
effect, and ACK/receipt/audit commit together. Late completion preserves cancelled
request and terminal run history. The default-off control-outcome flag now selects
original reservation in the approval API and enables exact-byte POST plus the
bounded UUID-keyset GET worker. Separate native positive/lost-ACK/OS-restart
approval and combined-extension evidence is recorded in the verification ledger;
it is not an installed rollout or full task admission proof.
See [approval journal ADR](adr/0025-original-approval-outcome-journal.md).

Approval preparation reuses exact native waiting-request/session/capability
checks before transactionally claiming the original journal. It sends command
UUID/store headers and saved action bytes once. Unknown transport or DB ACK
leaves the durable decision uncertain; replay never calls POST again. Recovery
does not prepare new capabilities: it verifies the saved origin, bearer scope,
epoch, native run, request and choice through GET only. Historical cancelled
requests remain eligible for their original witness, without reopening a run.
No PM/task admission or Java chat/control authority is added.

## Native Run Controls

The new opt-in Base [control outcome producer](contracts/HERMES_CONTROL_OUTCOME_V1.md)
records the original command before one native handler and exact ACK before
transport. GET-only outcomes survive producer restart; missing/uncertain never
authorize another effect. Fleet stop/steer consumes this protocol behind an
explicit default-false flag. Native/release acceptance remains required; do not
enable the plugin on installed agents or retrofit old submitted receipts.

The Rust wire-consumer prepares an opaque original context, sends its saved
action bytes/UUID/epoch once after claim and validates bounded exact ACK readback.
The internal repository now binds original stop/steer context to the single-use
claim and commits positive ACK/receipt/audit/event atomically (migration000015).
The supervisor uses that path when `hermes_control_outcome_enabled` is true;
its separate100-row UUID-keyset worker performs GET-only outcome recovery with
original accepted context, not a fresh epoch or control POST. Invalid records do
not starve later pages. Approval decisions have their separate original-outcome
consumer and journal above. A
deserialized context is revalidated before HTTP and is not a human/machine
authorization proof. See the outcome contract above.

The [control profile](contracts/HERMES_RUN_CONTROL_V1.md) separates acknowledgement
from terminal proof. Stop/steer require fresh Fleet/native identity, original
accepted journal/origin/credential context and advertised native capabilities.
HTTP200 JSON acknowledgements are bounded to64KiB/ten seconds; invalid/unknown
outcomes never retry or release capacity. Steer does not write running over a
concurrent state; stop ACK only sets stopping. A terminal race leaves final
mirror/commit to readback. Run-wide approval is retired inside the adapter too;
exact human decisions remain a separate flow. The additive control ledger now
reserves/claims commands before effects, replays identical keys without a second
POST and preserves submitted/uncertain holds across supervisor restart. ACK and
stopping commit with audit and durable events. Reconciliation observes only an
independent terminal mirror, never inferring unknown guidance acceptance.
Targeted approval now shares the original accepted free-chat context guard;
fresh capabilities and GET verify the waiting exact request in the pinned native
session before POST. Legacy/task context cannot substitute for admission.
ACK requires exact200, JSON MIME, identity encoding and64KiB/ten-second bounds.
A reserved preflight hold stays uncertain without automatic resend. This does
not attest loaded configuration generation. Current waiting-request GET recovery
is implemented separately; historical approval/unknown-decision recovery,
installed approval acceptance and safe process-tree stop remain gates.

## Bounded Hermes Stream

The [event-stream consumer profile](contracts/HERMES_EVENT_STREAM_V1.md) defines
strict authenticated HTTP/MIME/identity framing and resource/deadline budgets.
UTF-8 is decoded at complete lines, not individual network chunks. Unfinished
frames are not flushed at EOF, empty frames cannot leak an event name and invalid
JSON never becomes a tool/approval mirror. Frame, total input, event count,
transcript and cumulative delta snapshots are bounded. A limit retires only the
consumer; it does not stop the native run or release capacity. Independent
authenticated GET terminal proof remains necessary. Native event replay/control
reconciliation and task admission are separate gates, not supplied by the codec.

## Managed Native Acceptance Scope

The opt-in [native supervisor gate](../scripts/native_supervisor_live/README.md)
exercises real Fleet provisioning, renderer-2 activation, prompt dispatch and
transcript persistence with the Base launcher and two pinned native gateways.
Only model inference is a deterministic local fixture. It observes each loaded
SOUL, process HOME/cwd, assigned port, token boundary and persisted run identity
after restart. Private dotenv ownership is checked under a non-root QA identity.

This is not the installed runtime, complete native config/plugin attestation,
OS isolation or descendant safe-stop certification. Successful HTTP readiness,
tracked parent stop and terminal free-chat output do not grant task/PM admission.
The separate `recovery` scenario now verifies a real lost `202` followed by exit
of the dispatching Fleet process and recovery in a distinct Fleet process. The
actual Base witness lookup restores the original already-terminal run; request
context stays immutable and native observations require one POST/inference and
no SSE subscription. See the [verification ledger](CHAT_CLARIFICATION_VERIFICATION.md#managed-native-lost-ack-recovery-4-october-2026).
Running/tool/approval recovery and safe orphan ownership transfer remain separate
gates. These ignored tests are not run by ordinary cargo tests.

## Original-Key Recovery Candidate

The opt-in [Base native extension](contracts/HERMES_RECOVERY_V1.md) records the
original reservation witness in the same SQLite transaction, not SessionDB.
Fleet freezes verified scope/store epoch facts in its existing private dispatch
journal and requires the epoch header on the original POST. A submitted unknown
acceptance can only use non-dispatch lookup, then atomically commit that original
ID within the DB-clock horizon. Native GET pins the actual session and supplies
terminal evidence. No recovery path submits another run or uses negative lookup,
expiry, reset or EOF as permission to release capacity. Legacy intents remain
held; installed enablement and task admission remain independent gates.
[ADR 0019](adr/0019-native-original-key-recovery.md) records
the boundary. This protocol change adds no public Fleet route or migration.

Ordinary new Hermes snapshots use [renderer 2](adr/0017-versioned-native-hermes-renderer.md):
native `platforms.api_server.enabled`, loopback host/assigned port in config and
protected host/port/credential/HOME/CORS in dotenv. Launcher env alone is not
authoritative after Hermes dotenv loading. Version 1 preserves historical
config/env/marker bytes; existing files are never upgraded during provisioning.
Use normal drain/activation/rollback for a new revision. Native loader evidence
does not substitute for tracked-process/effective-revision/assignment attestation.

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
- Managed launch: Base compatibility wrapper with
  `serve --host 127.0.0.1 --port <api_port>`; not the raw upstream CLI.
- Readiness requires `/health` and `/v1/capabilities` with `run_status`,
  `run_events_sse` and `run_stop`.
- Message dispatch uses `POST /v1/runs`, Fleet session ids formatted as
  `fleet:<session_id>:<agent_id>`, and `GET /v1/runs/{run_id}/events` for SSE
  mirror updates.
- Runtime controls use `/steer`, `/stop` and `/approval` endpoints when the
  capability matrix allows them.
- Dashboard is an operator link, not the write channel for messages.
- An unavailable API does not prove an untracked runtime exited. Tracked or
  unconfirmed active/PID/desired-running state remains degraded with ownership
  metadata preserved. Stop/restart cannot replace it without proof. Only an
  already inactive observation can remain stopped; HTTP health is not quiescence.
- Prompt outbox is transactional. Unknown POST acceptance is not automatically
  retried; the agent remains occupied pending reconciliation.
- Free-chat submission now requires fresh durable wire capabilities and the
  private immutable journal described in the
  [data model](DATA_MODEL.md#hermes-dispatch-journal). Concrete run reservation
  and exact request/key/origin/default-profile credential fingerprint commit
  together, then a one-winner submission permit commits before HTTP. The client
  sends saved bytes, not reconstructed prompt/model/options. An unknown response
  keeps pending delivery with an error; neither the original key nor the fixed
  recovery horizon permits another POST. An independent prepared-intent worker
  recovers only an unconsumed original permit, never a submitted or unjournaled
  legacy record.
  Pending delivery with a saved prepared intent keeps its diagnostic and waiting
  state on pre-submission failure. Existing failed messages are never reopened.
  Managed recovery first verifies retained original child/generation custody,
  then compares the normalized fresh native facts with the private Fleet binding.
  A fresh controller cannot adopt a managed PID through this worker.
  It verifies frozen request/context and fresh protocol facts, then claims the
  existing run under drain/capacity/deadline locks. A prepared uncertain outbox
  becomes dispatching only with that permit. Concurrent workers have one winner;
  a crash after permit consumption remains unknown, not permission to resend.
  [ADR 0020](adr/0020-prepared-dispatch-restart-recovery.md) defines the boundary.
  Compatible positive original-key lookup is separate; operator reconciliation
  and managed runtime acceptance still remain release requirements.
- A verified HTTP 202 in a free chat now commits the run ID, prompt delivery and
  outbox acceptance together before any status GET. The run remains `pending`
  until an authenticated, bounded status read identifies its effective Hermes
  session; the requested `fleet:<session>:<agent>` alias is not that proof.
  Readback failure retains this ACK and agent capacity. A bounded keyset worker
  retries only GET for journal-backed pending and already pinned active runs after
  Fleet restart; original origin/fingerprint must match before HTTP. Legacy ACKs
  without that proof retain history/capacity but are not automatically probed.
  It never submits a prompt. The first transactional session pin starts the
  stream; identical concurrent pins do not start another worker. Effective
  session and runtime run IDs are immutable, and late generic updates cannot
  regress a terminal state. EOF status must also match that pinned session.
  Delivery updates serialize under a message row lock: an error with no native
  ID cannot erase a committed ACK, conflicting IDs fail, and terminal deliveries
  do not reopen. Controls reload the current run identity before HTTP; a pending
  session pin rejects stop/steer/approval even when a caller holds an old running
  snapshot. This is an explicit temporary limitation during readback outages,
  not confirmed cancellation. Independent acceptance/pin/control journaling is
  still needed to safely stop a known accepted run during such an outage.
  Task-bound/PM runs retain their separate authority and are not admitted by
  this free-chat recovery path. Source tests and live acceptance are recorded
  separately in the verification ledger. A pin-to-worker crash now recovers by
  status GET without another SSE attachment. Validated terminal run, prompt
  delivery, optional assistant and durable events commit atomically. Exact replay
  changes no timestamps/cursor; empty output creates no synthetic reply. Old
  SSE workers observe persisted terminal state; late delta/tool/approval writes
  are guarded under row locks. A crash before ACK commit or an unknown run ID
  still needs further durable recovery. The journal preserves evidence without implementing unknown-key
  recovery, retention-safe redispatch or process-tree quiescence. Native caps
  have no store epoch and even a new empty SQLite can advertise durable=true;
  a missing record/404 is not permission to recreate a run.
- The managed HTTP client disables implicit retries, redirects and environment
  proxies. Run submission accepts only HTTP 202 with a safe opaque `run_id`, a
  boolean `replayed` and a known `status`; a new run must say `started`. A replay
  preserves the original run identity, not a successful stage result. Rejection
  bodies and transport diagnostics are not exposed as dispatch errors.
  Capability JSON is bounded to 256 KiB, acceptance JSON to 16 KiB and PM status
  readback to 1 MiB, including streamed bodies; encoded responses are rejected.
- Before preparing a task-bound run, Fleet freshly verifies the concrete agent
  binding and the authenticated server-agent capability contract: exact run
  endpoints and durable idempotency with the pinned 86400-second retention.
  Memory-only fallback is refused before run preparation or prompt submission.
  This necessary wire prerequisite does not attest native skills/configuration
  or admit an assignment. The finite retention is not permission to redispatch
  unknown acceptance; lookup by idempotency key and full admission remain gaps.
- Stream EOF is not completion. Fleet requires a terminal event or terminal
  status readback, and deduplicates the final mirror response.
  Only exact `run.completed`, `run.failed`, `run.interrupted`, `run.cancelled`
  or `run.stopped` events are terminal; nested/subagent events, generic `done`
  and cancellation requests are not. Terminal payloads must identify the accepted
  run. Success additionally requires the native `completed=true`, `partial=false`,
  `interrupted=false` flags. After EOF the authenticated status read must be
  HTTP 200, bounded to 1 MiB, unencoded and identify `object=hermes.run` and the
  exact accepted `run_id`; `succeeded` is not an alias. Invalid or non-terminal
  evidence keeps the run waiting and its capacity held, without a fabricated
  assistant reply. This is run-state evidence, not OS/process-tree quiescence,
  Workflow completion or authorization for a Tracker stage transition.
- Configuration is draft/validated/activating/active/failed with desired and
  effective revisions. Activation drains runs and checks files/runtime before
  releasing the agent. Failed rollback keeps the agent drained.
  Workflow rebind and actual role/namespace/workflow identity changes share the
  agent row lock with runtime reservations. Active/pending/stopping runs, queued
  or unknown prompt dispatch and configuration drain reject changes with 409.
  Metadata edits with unchanged identity remain possible outside drain.
  Sensitive activation recovery now uses explicit private controller storage
  outside all agent paths; legacy agent-local documents block before mutations.
  See [environment](ENV.md#private-controller-storage) and
  [operations](OPERATIONS.md#private-activation-recovery-storage). This placement
  does not implement process-tree containment, crash takeover or loaded-generation
  attestation; those gates remain mandatory.
- Effective configuration readiness now reads the actual managed files on each
  request using the persisted effective head, independently of the last-100
  configuration history page. Validate/activate also use exact agent/revision
  lookup; only the current desired validated revision can activate. It compares
  `config.yaml`, `SOUL.md`, `.env`, enabled/disabled skills
  and the revision marker against the persisted effective snapshot, not against
  hashes or paths provided by a marker. Missing/changed managed files,
  re-enabled disabled skills, foreign markers and symlink/junction paths fail closed.
  Pinned Base revisions additionally verify the actual Git package/snapshot with
  bounded async Git IO (5 s/process, 10 s/package, fixed blob/batch size ceilings);
  failures remain sanitized and never fall back to HEAD or network. Closed HOME
  skill inventory includes nested/unlisted skill files and special
  entries (4096 entries / 16 levels maximum). No unexpected files are removed.
  Legacy revisions keep managed-only behavior; `.bundled_manifest` is never
  authority. Plugin/project/external discovery and loaded model/tool settings
  remain outside this observation; the separate
  `runtime_skill_inventory_not_verified` blocker remains until real inventory
  and native provenance are integrated.
  Verification does not create or repair directories/files. Secret values and
  hashes are not returned. This observation is not a fenced admission, proof of
  runtime-loaded configuration, or task-specific deployment workspace receipt.

### SDLC Skill Discovery Policy

Preparing a pinned Base package now seals the native `skills` settings in the
new draft: `project_discovery=false`, empty `trusted_project_dirs`,
`external_dirs`, `disabled` and `platform_disabled`, and `create_dir=null`.
Native project discovery can otherwise shadow HOME skills by name; an enabled
DB allowlist and byte-identical HOME files alone do not exclude those sources.
Unrelated skill/model settings remain unchanged. Package validation/readback
requires every sealed field to match; missing fields and later edits fail closed.
Within HOME/skills, readback permits only the materialized `<name>/SKILL.md` files.
Unlisted legacy Markdown, support documents, scripts, binary/template files and
hidden files are rejected, including those under an allowed skill directory:
native `skill_view(file_path=...)` can read such bytes without another `SKILL.md`.
The pinned package contains no attested support files; accepting future packages
with support requires an explicit versioned inventory contract. Unix readback
also rejects a canonical instruction with more than one hard link. Rejection is
read-only and preserves the unexpected files. Empty directories remain subject
to the existing entry/depth bounds. This does not prevent mutation after readback
or attest files outside the managed HOME tree.

This is a configuration revision, not an in-place change of an active agent.
Previously prepared SDLC package revisions without this policy require explicit
new draft preparation, validation and drain/activation; there is no historical
backfill or automatic repair. Free-chat configurations are unaffected.

### Native Inventory Limits

An opt-in Base `fleet-hermes-request-observer` candidate now captures digest-only
observations from the actual initialized native request builder. Authenticated
GET reads its bounded memory store without invoking configuration/discovery or
another model request. Captures are restricted to the exact original default
credential scope, bound to the native run/agent and factory incarnation, and held
on cancellation/owner loss. Actual pinned Hermes tests cover two isolated
processes and restart; selected system/tools/model HMACs match the deterministic
model's received request. See [verification](CHAT_CLARIFICATION_VERIFICATION.md#native-request-observation-8-october-2026).
Fleet now has an explicit [renderer-3 managed install/remove and bounded Rust
consumer](contracts/MANAGED_REQUEST_OBSERVER_V1.md), including signed rollback
bytes/absence and original run/launch/incarnation binding. The published b965298
source has selected two-Hermes native core evidence for install/remove/rollback,
source-tamper denial and original unknown-ACK recovery. Interrupted observer
activation/Fleet-death and complete inventory acceptance remain pending. The
main-reconciled source with Base875 has separate Linux/component evidence, not
a fresh native certification from those older SDK inputs. See
[current evidence](CHAT_CLARIFICATION_VERIFICATION.md#main-history-reconciliation-8-october-2026).
No accepted runtime was changed or consumed as admission. It reports
`complete=false`, `runtime_ready=false`: effective revision, full source
inventory and post-builder transformations remain unverified. It narrows one
observation gap without closing the full inventory requirement below.

Read-only source review of pinned Hermes `bbaf7af5` confirms that the HOME seal
does not close every native instruction source. Python plugin registrations may
point outside HOME; project plugin activation uses an independent environment
gate. Installed entry points, deferred platform plugins and lazy memory providers
can add skills. Managed configuration can override user settings, and a
context-local profile home can override the launch environment. Native listing
and callable lookup have different aliases/collision rules. Preprocessing, plugin
hooks and frozen session prompts can change the instructions actually delivered.

An eventual owner-side inventory endpoint must use already initialized runtime
handles and a captured configuration generation. It must not call discovery,
reload, lazy provider initialization, config-loader refresh or skill preprocessing
while claiming an observational read. Native `skills_list` and qualified
`skill_view` do not meet that requirement. Installing the endpoint plugin itself
is a separate explicit configuration change, not a read-only operation.

Required evidence includes original runtime incarnation, effective revision and
resolved HOME/profile/cwd; source owners, canonical names/aliases and ambiguity;
bounded exact captured bytes/hashes; active preprocessing, handlers/hooks and
frozen session prompt state. Unknown lazy registrations, unstable generations or
unaccounted transformations must return incomplete rather than grant readiness.
Authentication identifies the responder, not the safety of arbitrary loaded code.
This is a source-audit requirement beyond the implemented partial request
observer, not positive native inventory acceptance. Tracker assignment/Workflow first-step authority
remain separate gates; `runtime_ready=false` and task dispatch stay closed.

The optional native renderer gate consumes the actual Rust policy, checks the
pinned Hermes discovery and lookup helpers, and includes an unsealed negative
control with real project/external skill directories. This loader-only check
does not attest the full plugin inventory, live loaded model/tool state,
assignment lease, first workflow step or SDLC admission. Consequently
`runtime_ready=false` and both existing admission blockers remain authoritative.

## Контракт запуска Hermes

Для pinned Hermes `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3` исходная команда
`hermes serve` запускает dashboard/headless web server, а не gateway API
`/v1/runs`. Текущий Fleet argv совместим только с Base wrapper
`services-base/deploy/fleet-hermes-launch.py`, установленным в packaged image
как `/opt/fleet-hermes/bin/hermes`. Managed setting
`FLEET_CONTROL_FLEET__HERMES_COMMAND` должен указывать на этот wrapper;
имя `hermes` в PATH само по себе не доказывает совместимость.

Wrapper проверяет `--host=127.0.0.1` и порт `1024..65535`, задаёт
`API_SERVER_HOST` / `API_SERVER_PORT` и запускает
`/opt/hermes/.venv/bin/hermes gateway run`. Остальные команды передаются
исходному CLI без преобразования. `HERMES_SERVE_HEADLESS` остаётся частью
текущего Fleet env, но не превращает raw `serve` в gateway API.

Открытый gap: Hermes `hermes_cli/env_loader.py` читает агентский `.env` с
`override=True`, поэтому значения host/port могут заменить env wrapper.
Текущий Fleet renderer не защищает `API_SERVER_HOST` / `API_SERVER_PORT`
от значений `env_json` и не фиксирует их как managed defaults. Проверка
wrapper до запуска не является доказательством фактически загруженной native
конфигурации. Для прямого native gateway запуска нужен versioned renderer
с защищёнными host/port и отдельной проверкой загрузки. Старые immutable
snapshots, revision markers и ожидаемые managed bytes сохраняются: их нельзя
молча переписать или пересчитать по новым defaults. Этот документ не закрывает
native acceptance, `runtime_ready` или SDLC admission.
См. [контракт адаптера](contracts/HERMES_ADAPTER_CONTRACT.md).

Отдельный [native protocol gate](../scripts/hermes_protocol_live/README.md)
проверяет настоящий API adapter/AIAgent/SQLite с локальной моделью, dropped ACK
и process crash. Он не запускает этот wrapper или полный gateway runner и не
закрывает dotenv precedence, managed lifecycle, native config/tool attestation
или production Fleet journal/unknown-acceptance recovery.

## Java Agent

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
PM `completed` readback also requires the native success flags above; a partial
or interrupted payload cannot record successful proof or release PM capacity.

Fresh terminal proof reconciles the matching Fleet run in the same transaction,
releasing runtime capacity even when the event stream was lost. A delayed EOF or
cached running/waiting event cannot reopen the old run; unknown acceptance still
holds the agent slot. A generic terminal cache update without verified PM proof
is rejected. Transcript finalization and Tracker stage completion remain separate.

## Current Approval Snapshot Recovery

For an originally accepted, pinned free-chat run, authenticated status GET can
restore the currently visible `approval.request` after Fleet loses its stream.
Fresh targeted-approval capabilities, exact native run/session/request identities
and the original origin/derived credential are required. Unknown, malformed,
oversized or foreign snapshots keep capacity held without a new prompt or SSE.

One transaction rechecks the current agent, primary session and accepted journal,
inserts the redacted request once and promotes running to waiting. Replays do not
insert transcript messages/events or reopen resolved decisions, stopping or
terminal runs. Task/PM bindings remain denied without fenced admission. This
recovers only the native status document's current request, not missed historical
questions, tool events or an unknown approval decision's outcome. Approval ACK
loss still remains uncertain. Process-tree quiescence is a separate open gate.
