# Security

- Authentication uses JWT access tokens and HttpOnly refresh cookies.
- The first registered user becomes `admin`.
- `SystemRole = admin | operator | user`; `is_system_admin` remains a derived
  compatibility alias for `admin`.
- Filesystem access must be derived from database-managed agent paths.
- Reject `..` traversal, reject absolute paths outside the configured agents
  root, and verify existing `.fleet-agent.json` markers before provisioning.
- Secret-like env/log values are redacted before persistence and API return.
- `FLEET_CONTROL_FLEET__RUNTIME_TOKEN_SECRET` is required. Fleet derives a
  deterministic per-agent `API_SERVER_KEY` from that secret and the agent id;
  the raw token is written only to the managed agent env/config surface.
- Physical folder purge is not part of default delete; agents are archived and
  stopped first.
- Session lists default to the authenticated user on the backend.
- Only admin/operator users can expand session/user filters to other users.
- Backend RBAC is authoritative. The UI hides sections using
  `/api/v1/users/me/permissions`, but every protected route still checks the
  current role.
- `/agents/**`, runtime actions, config, skills, leader team binding, settings,
  deployments, logs and audit log require admin/operator.
- User management and role updates require admin, except that operators can list
  users for session filtering.
- A session without `leader_agent_id` is private and is not readable as a
  leader-scoped task.
- Selecting a leader for an executor session requires an existing
  `leader_executors` binding.
- Fleet Control stores transcript mirrors and dispatches through runtime
  adapters; it must not write directly into Hermes SessionDB.
- Mutating control-plane actions write `audit_log` entries with redacted
  payloads; message audits store author/type/length metadata instead of a
  second full prompt copy.
- Idempotency keys protect session and message creation from duplicate browser
  submits or retry storms. Reusing a key with a different payload returns
  conflict.
