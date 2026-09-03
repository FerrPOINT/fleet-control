# Data Model

Tables:

- `users`: accounts, `system_role`, legacy `is_system_admin`, refresh token
  hash and timestamps.
- `runtime_templates`: runtime kind metadata and capabilities.
- `agents`: sequential agent identity, runtime kind, product role, profile,
  status, ports and paths.
- `agent_runtime`: desired state, pid, health, command and env preview.
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
  session run; details are redacted and successful approvals close pending
  records for that run.
- `deployment_jobs`: provision/runtime update job queue and operator-visible
  lifecycle state.
- `control_settings`: typed JSON settings for runtime roots, ports,
  integrations and auth.
- `workflow_bindings`: per-agent namespace/workflow link.
- `agent_events`: audit-friendly event stream for UI invalidation.
- `agent_logs`: bounded process/runtime log records.
- `audit_log`: immutable operator action audit for agent changes, runtime
  actions, config/skill edits, leader assignments, handoff and message writes.

Important constraints:

- `users.system_role` is `admin`, `operator` or `user`; `is_system_admin` is a
  derived legacy alias for `admin`.
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
- runtime kind, role, status, desired state, skill state and session state are
  checked text values.

Indexes cover agent status filters, product-role filters, per-agent/per-user
session lists, leader-scoped session lists, participants, message ordering,
runtime runs, task-key lookup, workflow namespace lookup, deployment job state,
audit-log filters and recent events/logs.
