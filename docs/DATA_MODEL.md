# Data Model

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

`task_chat_bindings` binds session ID, stable Tracker instance, immutable project/task/root
UUIDs, concrete agent ID and verified central owner subject. Unique instance/task/agent
prevents duplicate histories. Explicit binding requires an empty private chat, matching
central owner and concrete assigned PM. Legacy creation system messages/pending run
placeholders are not user history; existing prompts or observed runs reject adoption.
Task binding is not inferred from task_key/title. A database trigger forbids changes to
bound session owner/agent/visibility/leader. Application routes reject handoff/leader
assignment and unverified generic prompt/steer. History uses `(created_at,id)` cursor
ordering and validates the cursor belongs to the session.

Questions, answers, immutable requirements revisions and confirmations live only in
Tracker. Fleet currently reads them through an authorized gateway; no autonomous
projection worker or PM assignment/resume saga is claimed by this migration.

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

## Tracker Event Inbox

The same single pending migration adds `tracker_event_cursors` and
`tracker_event_inbox`. Cursors belong to an immutable task-chat binding, not to an
agent-wide or user-wide feed. Tracker's sequence is global, so task-specific gaps
are valid; Fleet's stream cursor is allocated separately per session.

Inbox receipts are unique by `(session_id,event_id)` and
`(session_id,source_sequence)`, and reference one mirrored system message. They
store a canonical event hash and source metadata, not the raw answer/result.
Update/delete triggers protect receipts. One transaction commits the message,
safe durable invalidation, receipt and source cursor. Exact concurrent replay
adds nothing; changed payload/identity conflicts. A stale page with unseen events
must be fetched again from the persisted cursor, never merged speculatively.

This repository foundation is implemented. The authenticated background poller
and answer-to-PM continuation are not yet wired; a projection receipt is not a
runtime delivery receipt and does not transition Tracker business state.
