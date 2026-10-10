# Data Model

## Original Control Key Lookup

The literal control lookup GET reuses runtime_control_commands' existing unique
actor_user_id/idempotency_key index with exact session_id/session_run_id predicates
and LIMIT 1. Its projection is the existing redacted RuntimeControlReceipt only;
the key, payload hash and native pins stay private. There is no schema, migration,
write, audit/event or outbox change. Missing history is not a submission permit.

## Hermes Recovery And Terminal Atomicity

The recovery slice introduces no new migration or lineage entry. Optional verified recovery facts
are frozen inside the existing journal capabilities before its only submission
permit. Unknown-key acceptance checks the original DB-clock deadline again under
lock and at the mapping update; exact accepted replay does not rewrite history.
Terminal persistence locks agent/session/run/prompt/outbox/journal and commits
the optional redacted assistant/preview, delivery, run state/error/timestamps and
trigger-owned durable events together. Journal/outbox acceptance stays unchanged.
An identical terminal replay changes neither timestamps nor event cursor; a
contradiction rolls back. EOF, lookup absence and expiry never release capacity.
See [ADR 0018](adr/0018-atomic-terminal-pinned-recovery.md) and
[ADR 0019](adr/0019-native-original-key-recovery.md).

## Approval Recovery And Logical Time Candidate

Unit14 adds only `m20261005_000014_hermes_journal_time_order` after13 in both
migration lineages. A later trigger clamps new submission/acceptance timestamps
to prior journal progress without modifying identity, immutable original guard
or the recovery horizon. Nonempty journal downgrade is refused.
Existing approval request uniqueness by native request/run is reused: recovery
atomically inserts redacted exact content and running-to-waiting state, preserving
resolved/stopping states and exact replay. Existing durable uncertain decision
reservation, actor/key uniqueness and atomic delivery remain unchanged.
No new task/PM admission or inferred delivery from unknown native acceptance.

## Steer Transcript Follow-Up

No migration or lineage change is required. Existing `session_messages` supports
human-authored `control` messages and `mirrored` delivery; its primary key is
reused as the immutable control receipt ID. Original actor/creator and session
are retained, the Fleet-local runtime-message link pins run and command, and
`idempotency_payload_hash` stores the original operation/input hash. No caller
idempotency key is copied into the prompt namespace. Only redacted body is stored.
The message, preview, cursor events, ACK and audit commit together. The outbox
trigger queues only pending user prompts, not these mirrors. Replay validates
stored identity/content and neither appends nor updates a present mirror.
The control journal remains hash-only; prior migration bytes are untouched.

## Durable Runtime Controls Unit13

Only `m20261005_000013_runtime_controls` is new in this unit. Both lineages
append it after the unchanged journal12 (current lengths14/17).
`runtime_control_commands` freezes actor/key, payload hash, scope, original
context and native pins. No guidance body or bearer is stored.
Reserved/submitted/uncertain commands hold a run; claims are single-use.
Immutable history cannot be deleted or downgraded while populated.
ACK, audit/event and stopping state commit together. Independent atomic
terminal prompt/run/optional-assistant evidence can settle an unknown hold
as terminal-observed, never as proven command acceptance.
The following unit12 section records that earlier release's own scope.

## Docker Activation Restart Recovery

The [config18 restart successor](CONTAINER_ACTIVATION_RECOVERY_RELEASE.md) adds no
migration. Read-only keyset discovery includes claimed desired Docker revisions
with or without a recorded activation. `last_error` and an atomic deduplicated
audit retain typed recovery action, original phase/command IDs, custody generation
and original-command/readback hashes. They never change `claimed_at`, activation
identity, receipts, lease, effective revision or drain. Stale observations cannot
overwrite newer configuration/progress; existing run and custody fences remain.

The [config18 P2 successor](CONTAINER_ACTIVATION_PREFLIGHT_FIX.md) changes no schema
or historical custody. `claimed_at` is not cleared to retry; only an unsealed
read-only plan attempt may be retried by its live worker. Durable native permits
and effective revision publication remain unchanged.

`m20261009_000018_container_activation` adds controller-private
`runtime_container_activations`, one immutable claim and monotonic CAS record per
agent/target revision, at most one open activation per agent. Non-secret hashes,
original launches/stop receipts and readiness proofs are persisted; exact config
bytes/credentials stay in fsynced0600 private plans. Launch custody and effective
publication commit atomically with their activation steps. Populated custody
blocks downgrade; empty down restores exact17 guard definitions. See
[unit18 transitions and gates](CONTAINER_ACTIVATION_RELEASE.md).

Additive `m20261009_000019_recovered_activation` adds immutable
`runtime_container_activation_authorities`: original activation/claim/plan hash,
original recovery ID and current logical controller. Every native continuation
and phase CAS checks the exact current native lease/ACK and original readback;
history alone is not authority. It narrowly permits exited-anchor renewal only
for a fenced original activation (or its exact published generation), preserving
all other16 predicates/OID and empty-down restoration. Canonical/split inventories
are20/23. See [Base4 consumer](RECOVERED_ACTIVATION_CONSUMER.md).
Sequential claims optionally seal lineage to the original anchor/family and exact
terminal predecessor ID/hash. Old claim JSON/hashes remain byte-compatible.
The same unreleased migration19 adds closed predecessor/effective/drain checks
and latest-root-lease fences on sequential activation insert/update; no new table,
migration20, child anchor or rewritten original claim is introduced. This is a
source successor, not an upgrade for a database already migrated with f32619.
The standalone integration introduces no further migration. Configuration,
intent and mapping JSON hashes use the same Base ASCII-escaped canonical recipe,
including Unicode/SMP keys and values; existing mismatched hashes are never
rewritten. See [integration inventory](CONTAINER_ACTIVATION_INTEGRATION.md).

Private `runtime_container_preparations` (unit17) stores immutable initial-generation
intent hashes, one create-delivery permit and the original prepared receipt.
See [custody and downgrade fences](AUTOMATIC_CONTAINER_PREPARATION_RELEASE.md).

Unit16 adds `runtime_container_recoveries` with frozen per-generation epoch
commands/ACKs and durable bounded heartbeat delivery, while retaining original
000015 launch identity. See [the private unit contract](MAPPED_CONTROLLER_RECOVERY_RELEASE.md).
The unit16 correctness follow-up needs no schema change and never rewrites
historical bindings/receipts; foreign unknown heartbeat delivery stays held.

## Original Docker Controller

`m20261009_000015_container_controller` adds `runtime_container_launches` after
journal12 in both histories (14 canonical / 17 legacy entries). Its immutable
prepared registration/controller/generation identity progresses through
claimed/running/stopping/exited with one original physical snapshot, endpoint and
stable stop UUID. One non-exited generation per agent is enforced. Deletion and
nonempty downgrade are rejected. Journal12's existing origin gate gains a narrow
check against the running original generation; historical journal rows remain
unchanged. See [bounded Docker source release](DOCKER_LIFECYCLE_RELEASE.md).

## Hermes Journal Release Unit12

`m20261004_000012_hermes_dispatch_journal` is appended after PM credentials in
both canonical and legacy lineages (13 and 16 entries respectively). It records
immutable exact request bytes/hash, original origin/credential fingerprint,
capabilities, scope, deadline and the prepared/submitted/accepted progression.
The single submission permit is consumed before POST; run/message/outbox ACK
and journal acceptance commit atomically. Existing rows are not backfilled.
Nonempty journal history refuses downgrade. No migration13..22 is included.

## PM Credential Journal

Migration `m20261004_000011_pm_credentials` adds guarded optional `credentials`
to `pm_draft_creation_operations.operation`. Immutable intent pins the exact
delegation command/hash, parent fingerprint, origins and assignment machine
subject; immutable acknowledgement retains token ID, expiry and exact scopes.
Intent is saved before Base mutation; acknowledgement and metadata-only audit
commit together. Child/parent secrets are not stored. Existing journal-free JSON
is not rewritten. Both supported migration lineages append this same migration;
downgrade refuses a persisted credential journal. See
[credential preparation](plans/2026-10-09-pm-credentials-release.md) for gates and
remaining admission boundaries.

## Targeted Approval Commands

`runtime_approval_decisions` stores one immutable human decision per runtime approval request. It pins the session, run, actor, choice and command key; the actor/key pair is unique across requests. The initial state is `uncertain`, committed before any HTTP side effect. A verified exact acknowledgement permits transition to `delivered`. If final authorization fails before HTTP, the decision becomes terminal `failed` without resolving the request. Both terminal states are immutable; a failed command cannot later be delivered. Request resolution, audit and durable stream invalidation commit together. Replays never dispatch and never settle other pending requests. Raw runtime credentials and approval response bodies are not stored in this ledger.

## October Foundation Schema

Additive migration 000009 adds `agents.sdlc_role`, `session_event_cursors`,
`session_events`, `message_dispatch_outbox`, `agent_config_revisions` and
`agent_config_heads`. Cursor allocation is transactional per session, not a
global sequence; uncommitted late events cannot be skipped by committed cursors.
Message insertion, its durable event and dispatch row commit together.
Desired/effective config heads are separate; activation failure cannot promote
the desired revision. Migration 000010 adds explicit task bindings; assignment leases
are not implemented here yet. See [scope and blockers](SDLC_IMPLEMENTATION.md).

## PM Chat Bindings

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
assignment and unverified generic prompt/steer. History uses an internal immutable
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
- `fleet_alerts`: persisted health incidents. The historical migration 000006
  permits `heartbeat_stale`, not `agent_heartbeat_stale`. Heartbeat insertion
  locks the owning agent row, reuses an open/acknowledged incident, and permits
  a new identity only after resolution. Resolution and its redacted audit are
  atomic; an audit failure leaves the incident active. No migration is needed
  for this correction.
- `audit_log`: immutable operator action audit for agent changes, runtime
  actions, config/skill edits, leader assignments, handoff and message writes.

Important constraints:

- `users.central_sub` уникален для центральных профилей; email уникален только
  среди legacy rows с `central_sub IS NULL`, поэтому исторический и новый
  профиль могут безопасно иметь одинаковый email.
- `users.system_role` is `admin`, `operator` or `user`; `is_system_admin` is a
  derived legacy alias for `admin`. В центральном режиме эти поля не
  ограничивают людей и сохраняются только для совместимости.
- Central authentication never promotes these fields. Central role mutations
  are disabled; private session ownership is independent of historical role.
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
`pm_run_bindings` (eleven canonical or fourteen historical split ledger entries;
see [migration lineage](MIGRATIONS.md#historical-lineages-and-task-chats)).
Its pending down migration refuses populated transcript or task-chat history,
including unmessaged bindings/creation operations, before any schema effect.
Empty-schema down/reapply is supported; the up statement is unchanged.
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

This repository foundation is implemented. The authenticated background poller
and answer-to-PM continuation are not yet wired; a projection receipt is not a
runtime delivery receipt and does not transition Tracker business state.

## Clarification Answer Command Custody

One additive `m20261010_000020_clarification_commands` is appended to both
canonical and legacy split lineages. Historical migration SQL is unchanged.
`clarification_answer_commands` references the immutable task-chat binding and
local human user. Its identity, owner subject, full binding, question, original
key, canonical request bytes/hash and creation time are write-once. Option IDs
are a sorted set; text/comment, versions, nulls and key are retained unchanged.
PostgreSQL verifies SHA-256 of the UTF-8 body without an added extension.

`(actor_user_id,idempotency_key)` is unique. Partial unique indexes fence
unresolved commands both by session/question and by Tracker instance/task/
question/actor across chats and reassignments. Terminal history is retained.
Every repository operation rechecks active local owner/central subject/exact
binding under shared locks before querying the journal. The API supplies fresh
Tracker project proof before repository access; the repository does not treat a
client-supplied binding as project authorization.

Transitions are `stored -> delivering -> delivered|rejected|uncertain` and
`uncertain -> delivering`. Claim commits before HTTP. A database-clock 30-second
lease and new attempt UUID fence concurrent claims and expired-attempt recovery;
completion compare-and-swaps the exact attempt. Request-local Tracker HTTP is
bounded to 10 seconds and has no automatic retries. `ever_uncertain` is sticky:
unknown outcomes and recovery of an expired attempt prohibit a later rejection
from freeing the question. Only exact original-answer acknowledgement can then
settle it. No runtime dispatch or transcript completion is inferred.

Triggers reject identity changes, deletion, invalid transitions and uncertainty
erasure. Down migration refuses any nonempty journal; empty down/reapply and
ledger/history preservation require the focused PostgreSQL gate. Private answer
body is required for replay, not an audit/log payload. The new unit provides no
retention purge or unattended credential custody.

## Configuration Foundation Candidate (No New Migration)

This section describes the original configuration-only unit. Its normal source
integration with the separately owned runtime migrations preserves both schemas
without adding a migration; see the
[combined source boundary](plans/2026-10-10-runtime-config-integration.md).

This packet uses the existing `m20261001_000009_sdlc_foundation` tables and JSON
snapshot boundary. It neither edits that migration nor the foundation47-owned
`000010_task_chats`, and does not import migrations `000011` through `000022`.

`agent_config_revisions.snapshot.config.config_json.fleet_sdlc_package` holds
public exact-Git package metadata;
`fleet_sdlc_workflow_binding` freezes the strict Workflow v3 DTO. Numeric IDs are
distinct from namespace names, workflow keys and declared profiles. No new column,
credential table, native receipt or admission state is introduced. Package
preparation remains a draft; `agent_config_heads.effective_revision` is the
authoritative direct lookup, independently of the latest-100 history list.

Creation compares the desired head and concrete identity while holding the agent
row lock. Activation rechecks package identity under the same lock. Rebind and
identity mutations use existing drain, session-run and dispatch-outbox state to
refuse unresolved work. Fresh owner observations are not distributed leases.
