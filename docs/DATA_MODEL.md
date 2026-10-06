# Data Model

## Internal Container Endpoint Identity

The candidate additive migration `000019` stores `runtime_launch_endpoints`:
`launch_id` (primary key/FK to the original runtime launch), canonical `origin`,
original `pid` and `created_at`. The owning controller records it only after Base
verifies the original running namespace. Exact replay succeeds; changed or
foreign custody and update/delete/truncate fail. Dispatch joins its current
generation and private `fleet_launch` identity. This is not a public endpoint
setting, mutable status projection or authorization to resend an unknown prompt.
Native free chats remain localhost-only. See
[ADR 0030](adr/0030-generation-bound-container-endpoint.md) for limits and downgrade
protection. The real controlled-model chat and fresh workspace/migration gates
pass; ordered release, populated endpoint downgrade refusal and the remaining
recovery paths still require separate evidence.

Chats/PM consumer recovery adds no database table or migration. Original answer
and exact revision/hash confirmation commands stay in session-scoped browser
memory across tab changes; draft text/keys are not persisted in browser storage.
Reload relies on existing authorized Tracker state, not reconstruction of those
private command keys. Durable PM delivery/resume remains a separate journal gate:
[consumer handoff](CHATS_PM_CONSUMER_HANDOFF_20261006.md).

The existing private runtime-launch journal's immutable JSON binding may include
an optional `container` object: original Base registration, protected policy,
Compose/start/stop journal locations, source hashes and explicit Docker context.
Mapped boundary policy3 additionally requires a closed `mount_mapping` and
private `mapping_file`; registration requires `mount_mapping_sha256`. The hash
is canonical sorted JSON, and registration Engine must match the original proof.
Both optional fields are absent from historical host-bind/native records.
Container generation equals the launch UUID; resource equals the concrete agent
UUID. Paths/port/effective configuration are also fixed by the existing launch
binding. No new schema migration or public DTO is introduced for this candidate.
Historical native JSON omits `container` unchanged. Docker PID metadata is never
native process authority, and a stopped generation cannot be reused for restart.
See [Container Control](contracts/CONTAINER_CONTROL_V1.md) for validation and
remaining automatic generation/configuration work.

Automatic generation preparation now adds `runtime_container_preparations`
through additive `000018`. Its primary key is agent UUID + history ordinal;
generation and operation UUIDs are unique. It fixes controller UUID and canonical
intent SHA256 before either the private intent write or physical Docker create.
Only identifiers and hash are stored: no process/environment/credential bytes.
Exact replay requires the same controller and full identity; missing private
files cannot allocate another generation. UPDATE/DELETE/TRUNCATE and nonempty
downgrade are rejected. An outstanding preparation also blocks native fallback
or a mismatching launch. This is not automatic controller takeover or private
storage reconstruction. Legacy unrecorded preparations require inventory and
reconciliation before rollout; the migration does not infer lost historical
intent. Its original
creation intent is an exclusive private controller document, containing the
agent/paths/config revision, generation/operation UUIDs, process and source/context
pins. A mapped intent also fixes original controller snapshot/Engine/local root,
volume name/digest, projected proof paths, local-policy hash and mapping-file path.
The prepared document and DB binding must match this exact original proof;
fresh readback cannot replace it. Resolved env credentials remain outside DB/public DTOs. Base uses a separate
owned preparation SQLite file with application ID0x53444233, original payload hash,
committed create claim and prepared receipt. This is not Hermes SessionDB or a
business assignment queue. Only a matching prepared receipt precedes the existing
Fleet runtime-launch DB claim and sole process start; unknown creation cannot
allocate a replacement intent after private-file loss or change effective
configuration. The same claim is rechecked before a prepared generation may
proceed to the launch claim.

The next preparation ordinal is a single-snapshot count of immutable
runtime_launches rows for the agent, with outstanding claimed/started rows
rejected. It does not order by wall-clock timestamps or mutate old bindings.
An original namespace-exit receipt closes a Docker launch before another ordinal
can prepare a new generation. Unknown preparation retains its ordinal until DB
claim; unknown start retains the outstanding launch. Ordinal-specific private
files preserve prior generations without an overwrite/reset/head-pointer API.

Original-controller liveness verification does not add a migration or mutate
launch identity: an exited retained child leaves reconciliation holds intact.
Foreign health observation preserves agent/runtime/launch rows; its separate
request audit records observed and persisted statuses without claiming custody.

## Runtime Launches

Additive `000017` introduces internal `runtime_launches`: immutable UUID/agent/
controller binding, configuration phase/revision/snapshot and command hashes,
`claimed | gateway_started | gateway_exited | spawn_failed`, original positive
PID and observation timestamps. Raw commands, secrets and snapshots are excluded.
The partial unique index enforces one outstanding launch per agent across
controllers. Database guards prevent binding/PID/history mutation or deletion.
Any launch history blocks downgrade; legacy runtime rows are not backfilled.
The [contract](contracts/RUNTIME_LAUNCH_JOURNAL_V1.md) specifies replay and limits.
`gateway_exited` is deliberately not a boundary-empty or SDLC-completion receipt.

The [control outcome contract](contracts/HERMES_CONTROL_OUTCOME_V1.md) now has
an internal stop/steer journal in additive migration `000015`. The opt-in
supervisor persists it before POST and consumes saved-context GET outcomes;
the separate approval decision sender/GET worker uses its additive000016 private
journal without rewriting applied000013/000014/000015. The existing default-off
flag selects original mode for new API decisions; legacy history is never backfilled.
Existing historical receipts are not backfilled with a producer epoch.

## Runtime Approval Outcomes

`runtime_approval_decisions.outcome_required` is immutable from reservation and
defaults false for legacy decisions. `submission_claimed` changes false -> true
only once, while the original-mode decision is uncertain, atomically with its
`runtime_approval_outcomes` row. Context contains the exact closed once/deny
request bytes/hash, decision UUID, native run, original origin/credential
fingerprint and source-pinned store capabilities. It is private, not a DTO.

Fresh claim rechecks active actor/ownership, concrete primary Hermes agent,
pending request, active run/native session and accepted dispatch journal.
Task/PM bindings are refused without their separate admission. A replay never
grants another permit; legacy uncertainty cannot acquire context later.

Outcome state submitted -> acknowledged is one-way. Completion atomically
records delivered decision, audit and existing durable session events. A pending
request becomes approved/denied, but an already cancelled/settled request and
terminal run history remain unchanged. This method records a separately verified
prior ACK, not fresh authority; it is not connected to the HTTP sender yet.
Deferred constraints reject split claim/context and split ACK/receipt/audit.
Claimed uncertainty cannot become failed. UUID-keyset recovery pages hold at most
100 entries, including historical cancelled requests with unresolved decisions.
Empty downgrade restores the predecessor guard; any original decision history,
even an undispatched failed decision, blocks downgrade/deletion.

## Runtime Control Outcomes

`runtime_control_outcomes.command_id` is a one-to-one FK to the immutable
command. Its private, bounded JSON context preserves exact serialized request
bytes/hash, original command/run, origin, credential fingerprint and producer
capabilities/store epoch. It contains raw guidance, not a runtime bearer token;
it is never a public DTO or logged Debug value.

`runtime_control_commands.outcome_required` defaults false for legacy commands.
Only reserved -> submitted may activate it, in the same transaction as the
original context. A deferred guard prohibits committing a required context
without its row; it cannot be enabled or disabled after submission. Context
identity is immutable; no delete, backfill, expiry, negative-lookup reset or
new dispatch permit exists. The pending scan is UUID keyset-paginated at 100
rows with a partial pending index.

An exact positive witness atomically commits outcome ACK, command receipt,
nonterminal stopping state if applicable, audit and durable event. Submitted
or uncertain commands can become acknowledged only with that original context.
Deferred guards reject an ACK without its matching receipt. If independently
observed terminal history already exists, it stays `terminal_observed` with its
original observed state/timestamp: the public receipt projects the independent
ACK from the outcome row. Neither run nor transcript is reopened or rewritten.
Duplicate identical ACK commits return the same receipt without another event;
changed epoch/context/ACK conflicts. Nonempty downgrade refuses history loss.

Delivery updates serialize on their session with `FOR NO KEY UPDATE` before
locking the message. This matches dispatch/terminal session-before-child ordering
and avoids a message/event-trigger FK cycle with a session-owning journal writer.
Session event/cursor generation stays transactional; unknown acceptance still
keeps delivery pending and does not release dispatch capacity. No migration is
needed for this repository-level locking correction.

Generic run progress now likewise reads its immutable scope, locks the session
`FOR NO KEY UPDATE`, then locks PM proof/run and rechecks the scope. This avoids
the run/event-FK cycle with the original dispatch journal's exclusive session
lock. A real PG deadlock and deterministic fail-before/pass-after regression
verify this correction; no applied migration or timeout policy is changed.

Native free-chat stop/steer read the existing accepted dispatch journal by concrete
Fleet run ID; the receipt/live run are observed together. Control ACK is not a new terminal proof or
capacity release. Steer preserves current state; stopping remains nonterminal.
Legacy run-wide approval has no adapter bypass. See the
[run-control profile](contracts/HERMES_RUN_CONTROL_V1.md).

## Runtime Control Commands

Additive `m20261005_000013_runtime_controls` follows dispatch journal 000012;
historical migration bytes and legacy transcripts are unchanged. A command stores
immutable Fleet session/run/agent/actor, actor-scoped key, semantic payload hash,
native run/session pins and original request hash/origin/credential fingerprint.
It does not store raw guidance, token or arbitrary upstream response. The public
receipt omits the key and private context. Foreign keys bind the concrete run to
the dispatch journal; backend rechecks accepted state, primary agent and permissions.

States: reserved -> submitted -> acknowledged or uncertain. Known no-dispatch
preflight rejection is reserved -> rejected. Independently committed terminal
mirror permits submitted/uncertain -> terminal_observed or reserved -> rejected.
Database triggers protect identity and prohibit rewinding/overwriting final states.
Unique actor/key supports replay; a partial unique run index holds one unresolved
command. History and bounded reconciliation have dedicated indexes. Nonempty
downgrade refuses removal; empty downgrade/reapply is tested on a disposable DB.

ACK, stopping state, audit and durable session cursor/event share one transaction.
Reconciliation requires accepted original journal, pinned native message ID,
terminal run with last-event observation and completed/failed original prompt
delivery. A raw terminal state update is insufficient. Terminal observation
never invents the unknown command's acknowledgement or resends its native POST.

Hermes stream resource accounting is per worker and adds no schema or migration.
The [consumer profile](contracts/HERMES_EVENT_STREAM_V1.md) bounds received
frames, text and emitted full-text delta snapshots before the next write.
It is not a persistent storage quota or native history-replay receipt. A failed
consumer leaves the original run/session/dispatch journal and capacity held;
existing GET-only recovery may commit an independently verified terminal result.

Prepared restart recovery uses the existing immutable dispatch journal and outbox,
not a new queue/schema. Its bounded keyset excludes consumed permits, legacy and
task/PM records. Claiming a prepared uncertain record changes its outbox to
dispatching in the same transaction as the one-way journal permit; request/key/
run/horizon are not renewed. See [ADR 0020](adr/0020-prepared-dispatch-restart-recovery.md).

Free-chat run acceptance continues to use existing columns:
`runtime_run_id`/message `runtime_message_id` and outbox dispatched commit
atomically after a verified 202. Pending plus a run ID means accepted but awaiting
effective-session GET. The first session pin replaces the requested alias and
sets running under row locks; replay preserves terminal state and timestamps.
These application guards do not retrospectively attest legacy rows or protect
against direct privileged SQL. Task-bound/PM records use their separate model.

## Hermes Dispatch Journal

Additive `m20261005_000014_hermes_journal_time_order` follows 000013. Its
BEFORE UPDATE trigger runs after the unchanged original identity/ACK/expiry
guard and floors only new submitted/accepted timestamps to prior progress.
Creation/deadline, identity/key/hash and consumed send permit are unchanged.
These timestamps represent logical progress, not reliable elapsed wall time.
Nonempty downgrade is refused. Clock integrity and horizon authority remain
separate requirements; see [ADR 0023](adr/0023-logical-journal-progress-time.md).

Optional closed `capabilities.fleet_recovery` freezes the source-pinned native
store epoch/default-profile scope and non-dispatch endpoint before the original
submission. It is private metadata, not an arbitrary upstream object or secret.
Existing immutable JSON/bytes guards apply; no new migration or legacy backfill.
`recovery_allowed` is an internal DB-clock query projection, not stored authority.
Recovered acceptance rechecks original facts and deadline under the journal lock
and at its atomic mapping update. The producer's witness tables live only in the
native run-idempotency store, not Fleet's DB or Hermes SessionDB. See
[recovery v1](contracts/HERMES_RECOVERY_V1.md).

Additive `m20261004_000012_hermes_dispatch_journal` adds the private
`hermes_dispatch_journal` ledger. A transaction reserves the concrete primary
free-chat run and records the original message/run/session/agent, requested
alias, exact serialized request bytes and SHA256, UUID idempotency key,
loopback origin, default-profile credential fingerprint and bounded verified
protocol facts. Raw runtime tokens, arbitrary capabilities metadata and journal
contents are not public API/log/audit DTOs. Prompt bytes are sensitive, as are
the original messages; database access and backup protection still apply.

The DB clock sets the immutable creation time and conservative recovery
deadline (86400-second advertised retention minus 60 seconds). State is
`prepared -> submitted -> accepted`. A row-locked single-send permit commits
`submitted` before HTTP; concurrent callers cannot acquire a second permit.
Verified ACK commits the run/message/outbox and `accepted` together. Database
guards reject rewritten bytes/hash/key/scope/timestamps, backwards progress,
deletion and downgrade with any journal rows. Migration does not invent intent
or credentials for historical records.

`prepared` proves no submission permit was consumed, not that an automatic
recovery worker exists. `submitted` without a known native ID remains unresolved
and holds capacity; pending prompt plus delivery error is not definitive rejection.
Known accepted-but-unpinned recovery requires its original journal context.
Legacy ACKs lacking that proof remain readable and held, without automatic HTTP.
The deadline never renews and is a rejection guard, not SQLite continuity proof.
Unknown-key lookup and operator reconciliation remain open. Known pinned active
runs now use GET-only recovery with the same accepted journal/origin/credential.

## Atomic Hermes Terminal Packet

The private repository command uses existing run/message/outbox/journal columns;
there is no new migration. Agent/session/run/prompt/outbox/journal row locking
validates exact acceptance and effective native pin before writing. One transaction
stores the redacted assistant when nonempty, preview, delivery and terminal run
timestamps/state. Existing triggers allocate durable invalidations in that same
transaction. Fault rollback leaves transcript/cursor and capacity unchanged.
Journal/outbox acceptance remains immutable; exact replay updates no rows.
Session locking permits one assistant per accepted native run, reusing only an
identical historical partial mirror. Empty completed, failed and cancelled
packets create no synthetic assistant. Late delta/tool/approval writes serialize
with terminal state. See [ADR 0018](adr/0018-atomic-terminal-pinned-recovery.md).

## Local Configuration Recovery Material

Configuration snapshots now have optional `renderer_version`: absent/1 uses the
historical renderer and retains its previous serialized snapshot/file/hash shape.
New server-created Hermes revisions store 2, while Java revisions retain 1.
Existing rows are not backfilled. Unknown versions fail validation/rendering;
the config-edit request has no renderer selector. Version 2 adds its version to
the file marker and derives native listener fields/protected env from agent
identity. It does not change desired configuration JSON or grant runtime admission.

Hermes activation writes an exclusive, size-bounded, sensitive controller-side
`<controller_root>/<agent-uuid>.activation.json` before stop/file mutation. The
operator-provisioned Linux root must be private and outside every agent path;
it is not an agent-config field or a runtime mount. Version2 records original
agents/config locations, agent/revision, prior runtime state, relative managed paths,
previous bytes (hex) and expected hashes/absence. It is not a DB migration,
new source of configuration authority or runtime/Workflow completion receipt.
The existing revision/head transaction remains authoritative; only after its
verified result commits can the byte-identical journal be removed. Interrupted
operations keep recovery material without automatic claim takeover.
Existing `config/.fleet-activation-journal.json` v1 remains an explicit blocker:
it is not moved, rewritten, deleted or upgraded automatically. An empty/missing/
unsafe controller root holds new activation before file/runtime effects. No DB
migration or automatic Windows ACL fallback is introduced.
No new schema/renderer version is needed for Linux directory durability. Managed
file rename/unlink and new ancestor directory entries must be synchronized before
the existing head transaction may acknowledge application or rollback. Failed
barriers retain an unconfirmed activation and drain, not an active revision.

## Targeted Approval Commands

`runtime_approval_decisions` stores one immutable human decision per runtime approval request. It pins the session, run, actor, choice and command key; the actor/key pair is unique across requests. The initial state is `uncertain`, committed before any HTTP side effect. A verified exact acknowledgement permits transition to `delivered`. If final authorization fails before HTTP, the decision becomes terminal `failed` without resolving the request. Both terminal states are immutable; a failed command cannot later be delivered. Request resolution, audit and durable stream invalidation commit together. Replays never dispatch and never settle other pending requests. Raw runtime credentials and approval response bodies are not stored in this ledger.

Approval preflight reuses the original accepted dispatch journal by concrete run;
it does not backfill legacy requests, renew keys or create another permit. Current
native GET verifies the waiting exact request and pinned session. If preflight
fails after decision reservation, that existing uncertain receipt stays held;
the API does not infer a delivered/failed native effect or resend after recovery.
No new column/migration or config-generation attestation is introduced here.

## October Foundation Schema

Additive migration 000009 adds `agents.sdlc_role`, `session_event_cursors`,
`session_events`, `message_dispatch_outbox`, `agent_config_revisions` and
`agent_config_heads`. Cursor allocation is transactional per session, not a
global sequence; uncommitted late events cannot be skipped by committed cursors.
Message insertion, its durable event and dispatch row commit together.
Desired/effective config heads are separate; activation failure cannot promote
the desired revision. Migration 000010 adds explicit task bindings; assignment leases
are not implemented here yet. See [scope and blockers](SDLC_IMPLEMENTATION.md).

Pinned Base preparation reuses these config revisions without a new migration.
Human readiness and machine configuration observation read `agent_config_heads.effective_revision`
directly; the chronological last-100 revision list is not head authority. Exact
pinned revision verification and validate/activate use direct agent/revision lookup.
Only the desired validated revision can activate; history paging cannot override
this rule. The observation UUID/time
is not persisted as an assignment lease, approval or execution receipt.
The snapshot's `config_json.fleet_sdlc_package` stores metadata only: exact Base
commit, normalized manifest/instruction hashes, role namespace/profile/modes and
skill hashes. Its SOUL and skill content remain in the protected snapshot, not
in audit. Proof metadata is not trusted without pinned Git verification.
Preparing a draft compares the desired revision under the agent row lock;
activation verifies the current role/namespace/workflow IDs, and agent updates check drain
under the same lock. Effective configuration and installed skill rows do not
change during preparation. This is not an assignment or runtime receipt.

`config_json.fleet_sdlc_workflow_binding` freezes Workflow-owned persisted
namespace ID/name, workflow ID/key, role, declared profile, catalog version/hash
and skills revision. The ID strings are canonical positive signed-64-bit decimal
IDs from Workflow, not namespace symbols. `namespace_id` and `workflow_id` in
the snapshot match those frozen IDs. Preparation and activation check the agent
IDs under its row lock; public validation/activation and supervisor apply also
compare a fresh authenticated owner readback. A source/DB declared profile is
not the effective Hermes runtime profile. This adds no table or migration.

## PM Chat Bindings

Additive migration `m20261004_000011_pm_credentials` extends the creation guard;
historical 000010 is unchanged. An optional private `credentials` journal inside
the existing operation JSON stores the exact Base command, canonical payload
hash, original parent fingerprint, fixed integration origins and machine subject.
The absent field preserves legacy serialization; it is not populated by migration.
The only progress is absent -> intent -> acknowledged child metadata. Intent and
receipt are immutable, including after expiry/revocation; secrets are never stored.
The child receipt contains token UUID, scopes and expiry, not proof of admission.
ACK persists before fresh child/Tracker readback, so Tracker failure does not erase
successful issuance. Row locking and Base's original-parent/key replay serialize
concurrent duplicates without a second child. First intent/ACK audit rows commit
with their operation update; replay produces no new audit rows. Downgrade refuses
any journal and never discards recovery material. No new table/index/scheduler is
needed; owner/operation and owner/key lookups retain their existing indexes.

`pm_draft_creation_operations` is the owner/key-unique creation ledger in pending
migration 000010. It stores immutable owner/project/agent/input and monotonically
added Tracker Draft, original-input, reservation and atomic chat receipts.
Operation UUIDs derive distinct stable Tracker creation/reservation and Fleet chat
keys. Foreign owners cannot read the row; disabled or remapped local central users
cannot advance it. Row locks serialize receipt recording, not network requests.
Database guards prohibit changing/removing acknowledged receipts. Unknown remote
outcomes leave earlier stages intact and are reconciled with authenticated Tracker
readback before any same-key write. No bearer/PAT/runtime token is stored. Original
input is private task content and must be protected in DB/backups like transcripts.
The ledger is not a second scheduler, business stage authority or admission proof.
Recovery by key uses the existing unique `(owner_user_id, idempotency_key)` index;
it does not add a migration or store another copy of the prompt. Continuation
reads the immutable row by owner/operation ID and reuses its remote command keys.
The project directory is a request-scoped Tracker projection, not a local project
registry or saved authorization snapshot.

Internal `create_pm_draft_chat` creates the private session, immutable Tracker
binding, owner/primary participants, audit and durable `task.bound` event in one
transaction. It is not a public creation endpoint or a Tracker authority proof;
the coordinator must validate fresh human/project access and authoritative Draft
and PM reservation receipts before calling it. It checks the actual local central
owner and concrete non-archived Hermes PM, serializes actor/key replay with legacy
session creation, and rejects changed payload or duplicate task/agent bindings.
No unbound session is committed, and no prompt, pending run or dispatch is created.
Runtime health and admission remain separate prerequisites.

`task_chat_bindings` binds session ID, stable Tracker instance, immutable project/task/root
UUIDs, concrete agent ID and verified central owner subject. Unique instance/task/agent
prevents duplicate histories. Explicit binding requires an empty private chat, matching
central owner and concrete assigned PM. Legacy creation system messages/pending run
placeholders are not user history; existing prompts or observed runs reject adoption.
Task binding is not inferred from task_key/title. A database trigger forbids changes to
bound session owner/agent/visibility/leader. Application routes reject handoff/leader
assignment and unverified generic prompt/steer. Reading task context preserves that
immutable binding after PM reassignment and requires fresh Tracker project access;
its permission flags are attenuated for an absent or different assigned agent.
No binding, assignment or run record is rewritten by this read. History uses an internal immutable
`session_messages.append_sequence` identity allocation order and validates that the
public UUID cursor belongs to the session. The bigint is not added to message DTOs.
Gaps are allowed: allocation order is neither transaction commit order nor an SSE
replay cursor. Migration 000010 backfills existing records in timestamp/UUID order;
it cannot recover historical insertion order lost before the migration. New records
retain allocation order even when the host clock moves backwards. Identity inserts
and a database update guard prevent reassignment of an allocated position.
The backfill temporarily disables only the message-change trigger inside the
transactional migration, avoiding synthetic transcript events for old records.

Questions, answers, immutable requirements revisions and confirmations live only in
Tracker. Fleet reads them through an authorized gateway and an opt-in authenticated
metadata projection worker. Neither projection nor this migration performs a PM
resume saga or dispatches prompts. The owner-issued initial Draft reservation is
coordinated separately by the creation ledger; runtime admission remains unwired.

Tables:

- `users`: локальные профили для авторов/FK, immutable `central_sub`, legacy
  `system_role`/`is_system_admin`, refresh token hash and timestamps.
- `runtime_templates`: runtime kind metadata and capabilities.
- `agents`: sequential agent identity, runtime kind, product role, profile,
  status, ports and paths.
- `agent_runtime`: desired state, pid, health, command and env preview.
- `AgentStorageReport` and `AgentStorageReview` are computed from the managed
  filesystem on demand and are not stored in PostgreSQL.
- `agent_configs`: config JSON, SOUL.md text and redacted env JSON.
- `agent_skills`: per-agent skill selection and optional edited content.
- `leader_executors`: many-to-many team binding from leader agents to executor
  agents.
- `agent_sessions`: user-owned task chats with primary agent, optional selected
  leader, optional parent session, idempotency key/hash and
  private/leader-scoped visibility.
- `session_participants`: owner, primary agent, selected leader and observer
  participants for each session.
- `session_messages`: Fleet Control mirror of transcript/control events with
  idempotency key/hash, delivery state, runtime message id and author user for
  replay protection.
- `session_agent_runs`: per-agent runtime session/run links for one Fleet
  session, including runtime run id, state, model/provider/options and last
  event/error timestamps.
- `runtime_approval_requests`: Hermes approval mirror records tied to a Fleet
  session run; details are redacted and successful targeted decisions close only
  their exact request, never every pending request in a run.
- `deployment_jobs`: provision/runtime update and Service Pulse product deploy/rollback
  jobs with operator-visible lifecycle state. Product jobs keep `demo`, exact SHA
  or previous Forge release ID in `detail`, a unique nullable UUID
  `idempotency_key`, Forge deployment/pipeline IDs and post-release health
  result. Only product jobs require the key; agent runtime jobs remain separate.
- `control_settings`: legacy typed JSON rows for runtime roots, ports,
  integrations and auth. They are retained for migration compatibility but
  are not an active configuration source; `GET /settings/*` reads startup
  configuration.
- `workflow_bindings`: per-agent namespace/workflow link. `binding_status` is computed from the live Project Workflow catalog: `connected` for an exact ID/name match, `stale` for a removed or renamed persisted selection, and `unbound` when no selection exists.
- `agent_events`: audit-friendly event stream for UI invalidation.
- `agent_logs`: bounded process/runtime log records.
  Insert acknowledgements return the stored UUID, agent, stream, redacted message
  and timestamp from the same PostgreSQL statement. No latest-row lookup, schema
  change, generation cursor or Docker ingestion table is introduced by this fix.
- `audit_log`: immutable operator action audit for agent changes, runtime
  actions, config/skill edits, leader assignments, handoff and message writes.

Important constraints:

- `users.central_sub` уникален для центральных профилей; email уникален только
  среди legacy rows с `central_sub IS NULL`, поэтому исторический и новый
  профиль могут безопасно иметь одинаковый email.
- `users.system_role` is `admin`, `operator` or `user`; `is_system_admin` is a
  derived legacy alias for `admin`. Verified Central Auth establishes identity;
  the stored active user and system role still determine Fleet permissions.
- `agents.ordinal` and `agents.name` are unique.
- `agent_skills` is unique by `(agent_id, name)`.
- `agents.product_role` is `leader` or `executor`.
- `agents.role` is a profile value: `developer`, `tester`, `it_lead` or
  `custom`.
- `leader_executors` is unique by `(leader_agent_id, executor_agent_id)` and
  cannot point a leader at itself.
- `agent_sessions.user_id` references `users.id`; new sessions are always
  created for the authenticated user.
- `(agent_sessions.user_id, agent_sessions.idempotency_key)` is unique when an
  idempotency key is supplied.
- `agent_sessions.agent_id` is retained for compatibility and is treated by the
  public API as `primary_agent_id`.
- `agent_sessions.leader_agent_id` is nullable; `NULL` means private chat.
- `agent_sessions.visibility` is `private` or `leader_scoped`.
- `session_messages` requires exactly one author shape: user, agent or system.
- `(session_messages.session_id, session_messages.created_by_user_id,
  session_messages.idempotency_key)` is unique when a user idempotency key is
  supplied.
- `session_agent_runs` tracks each runtime participant independently.
- `workflow_bindings` is unique by `agent_id`.
- Historical `control_settings.auth` rows may contain `mode`, `jwt_issuer`,
  `jwt_audience`, token TTLs and refresh-cookie policy, but are ignored by the
  effective settings API and runtime.
- runtime kind, role, status, desired state, skill state and session state are
  checked text values.

Indexes cover agent status filters, product-role filters, per-agent/per-user
session lists, leader-scoped session lists, participants, message ordering,
runtime runs, task-key lookup, workflow namespace lookup, deployment job state,
audit-log filters and recent events/logs.

Chat directory counts, cursor validation and the page share one scoped SQL
snapshot. The owner IDs arrive as JSON and are converted once into a UUID array
for `user_id = ANY(...)`, so PostgreSQL can use the existing user/session index
for the normal owner scope. A JSON set subquery in `IN (...)` substantially
overestimates matching sessions and can make both scope selection and page
hydration scan all owners' history. The explicit all-users scope still counts
its full visible history; project ACL predicates apply in either mode.

## PM Run Proof

The feature's single pending migration `m20261001_000010_task_chats` adds
`pm_run_bindings` (eleven migration files after the accepted main refresh).
Each Fleet run UUID has one immutable
reservation containing its task chat, concrete agent, Tracker identity,
assignment/execution, workflow binding, dispatch operation key, checkpoint and
fence. Reservation and technical capacity allocation commit together, before any
runtime HTTP request. Concurrent identical reservations replay the same record;
altered payloads conflict. An unresolved reservation keeps the agent occupied.

Runtime acknowledgement pins both the agent-local Hermes run reference and its
effective session ID (Hermes can resolve a Fleet alias). Both are write-once;
uniqueness is scoped to the agent, not the fleet. Authenticated readback records
terminal proof once and rejects regression or a different terminal result. DB
triggers also prevent reservation/mapping/terminal mutation through direct SQL.
Verified terminal observation atomically updates the matching visible run state
and frees that runtime capacity. Generic cached/SSE-error updates cannot replace
the accepted mapping, introduce terminal state without proof, or regress verified
terminal state. Both paths lock the PM binding before the runtime run. A mapping
mismatch rolls back proof and run state together. This never advances Tracker;
business completion still requires its own workflow/requirements receipts.
Success observation requires the accepted run/session identity and native
`completed=true`, `partial=false`, `interrupted=false` flags, not just a status
string. A contradictory partial result leaves the reservation unresolved.

## Tracker Event Inbox

The same single pending migration adds `tracker_event_cursors` and
`tracker_event_inbox`. Cursors belong to an immutable task-chat binding, not to an
agent-wide or user-wide feed. Tracker's sequence is global, so task-specific gaps
are valid; Fleet's stream cursor is allocated separately per session.

The cursor also pins `projection` (`legacy_full_v1` or `metadata_v1`) and
`contract_version=1`. The first successful page, including an empty page,
creates the pin in the same transaction as its receipts. Its trigger rejects
deletion, identity/format changes and cursor regression. Existing legacy receipts
cannot be interpreted as metadata digests; switching requires a separately
specified explicit migration, not a poller option or an implicit reset.

Inbox receipts are unique by `(session_id,event_id)` and
`(session_id,source_sequence)`, and reference one mirrored system message. They
store a canonical event hash and source metadata, not the raw answer/result.
For metadata this is the verified Tracker `metadata_sha256`; legacy hashing is
unchanged. Metadata source cursors are canonical decimal strings on the wire,
converted losslessly to PostgreSQL bigint only after validation. Their safe Fleet
stream invalidation also uses a string; legacy numeric invalidations are unchanged.
Update/delete triggers protect receipts. One transaction commits the message,
safe durable invalidation, receipt and source cursor. Exact concurrent replay
adds nothing; changed payload/identity conflicts. A stale page with unseen events
must be fetched again from the persisted cursor, never merged speculatively.

This repository foundation and disabled-by-default authenticated background poller
are implemented. Live producer acceptance and answer-to-PM continuation remain;
a projection receipt is not a
runtime delivery receipt and does not transition Tracker business state.

## Recovered Current Approval

No schema or migration is added. `runtime_approval_requests` retains its unique
`(session_run_id,runtime_approval_id)` identity. Recovery locks agent, primary
session and run in that order, verifies the original accepted free-chat journal,
then commits the redacted pending request and running-to-waiting transition
together. Existing content must match on replay; prior decisions are retained.
Existing database triggers create durable approval/run events. No transcript
message is fabricated from the snapshot and repeat reads do not advance cursors.
The native GET is evidence for its current request, not a historical event inbox.
