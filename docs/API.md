# API

Base path: `/api/v1`.

Auth:

- `POST /auth/register`
- `POST /auth/login`
- `POST /auth/refresh`
- `POST /auth/logout`
- `GET /users/me`
- `GET /users/me/permissions`
- `GET /users`
- `PATCH /users/{user_id}/role`

Access tokens are local HMAC JWTs in MVP. New tokens include `aud`, `iss`,
`role`, `scopes` and `sid` claims compatible with the future `sdlc-auth-core`
validator. Legacy compact tokens without `aud`/`iss` remain accepted during the
transition window.

RBAC:

- `admin`: all users, settings, RBAC, sessions and runtime actions.
- `operator`: agents, leaders, executors, runtime, config, skills, deployments,
  logs and all sessions.
- `user`: own sessions/messages and read-only agent directory.

Fleet:

- `GET /dashboard`
- `GET /agent-directory`
- `GET /agents`
- `POST /agents`
- `GET/PATCH/DELETE /agents/{agent_id}`
- `GET /agents/{agent_id}/storage` returns the managed folder storage report,
  marker status and purge eligibility.
- `POST /agents/{agent_id}/purge-files` physically removes the managed
  `agents_root/agentN` folder after archive, exact name confirmation and marker
  validation.
- `POST /agents/{agent_id}/provision`
- `POST /agents/{agent_id}/start`
- `POST /agents/{agent_id}/stop`
- `POST /agents/{agent_id}/restart`
- `POST /agents/{agent_id}/health`
- `GET/PUT /agents/{agent_id}/config`
- `GET /agents/{agent_id}/skills`
- `PUT /agents/{agent_id}/skills/{skill_name}`
- `GET /leaders`
- `GET/PUT /leaders/{leader_agent_id}/executors`
- `GET /executors`

Sessions and workflow:

- `GET /sessions?agent_id={agent_id}&leader_agent_id={leader_id}&user_id={id1,id2}`
  lists sessions by primary agent, selected leader and user filter.
- Omitting `user_id` returns only the current user's sessions.
- `user_id=all` returns all users only for admin/operator; normal users are
  forbidden from expanding beyond themselves.
- `POST /sessions` creates a session owned by the authenticated user. Use
  `primary_agent_id`; legacy `agent_id` is still accepted.
- `POST /sessions` is idempotent by `idempotency_key`; replay returns the
  original session, while the same key with a different payload returns `409`.
- `GET /sessions/{session_id}`
- `GET/POST /sessions/{session_id}/messages`
- `POST /sessions/{session_id}/messages` is idempotent by request key and avoids
  duplicate runtime dispatch on replay.
- `GET /sessions/{session_id}/stream`
- `GET /sessions/{session_id}/participants`
- `PUT /sessions/{session_id}/leader`
- `POST /sessions/{session_id}/handoff`
- `POST /sessions/{session_id}/delegations`
- `GET /sessions/{session_id}/runs`
- `POST /sessions/{session_id}/runs/{run_id}/steer`
- `POST /sessions/{session_id}/runs/{run_id}/stop`
- `POST /sessions/{session_id}/runs/{run_id}/approval` forwards the decision to
  Hermes and resolves pending Fleet approval mirror records for that run.
- `GET /workflow-bindings`

Session defaults:

- Direct executor chat: `leader_agent_id = null`, `visibility = private`.
- Direct leader chat: primary agent and selected leader are the same leader.
- Child executor session from a leader chat inherits `leader_agent_id` and gets
  `parent_session_id`.
- Backend validates that selected leaders manage the target executor through
  `leader_executors`.

Runtime:

- `GET /runtime-templates`
- Hermes chat dispatch uses the runtime adapter and `/v1/runs`; Fleet sends
  `session_id=fleet:{session_id}:{agent_id}` and stores the mirror transcript.
- `GET/POST /deployments/jobs`
- `GET /deployments/jobs/{job_id}`
- `POST /deployments/jobs/{job_id}/cancel`
- `GET /logs`
- `GET /events` as SSE
- `GET /events/recent`
- `GET /audit-log`

Settings:

- `GET/PUT /settings/runtime`
- `GET/PUT /settings/ports`
- `GET/PUT /settings/integrations`
- `GET/PUT /settings/auth`

Auth settings expose `mode`, `jwt_issuer`, `jwt_audience`, token TTLs and
refresh-cookie policy. `mode=hmac` is the only active mode until
`sdlc-auth-core` is adopted.

The frontend build regenerates TypeScript types from `openapi/openapi.json`.
The OpenAPI JSON is regenerated from Rust source before release. Native Windows
regeneration requires MSVC `link.exe`; WSL/Linux generation is supported.
