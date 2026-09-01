# API

Base path: `/api/v1`.

Auth:

- `POST /auth/register`
- `POST /auth/login`
- `POST /auth/refresh`
- `POST /auth/logout`
- `GET /users/me`
- `GET /users`

Fleet:

- `GET /dashboard`
- `GET /agents`
- `POST /agents`
- `GET/PATCH/DELETE /agents/{agent_id}`
- `POST /agents/{agent_id}/provision`
- `POST /agents/{agent_id}/start`
- `POST /agents/{agent_id}/stop`
- `POST /agents/{agent_id}/restart`
- `POST /agents/{agent_id}/health`
- `GET/PUT /agents/{agent_id}/config`
- `GET /agents/{agent_id}/skills`
- `PUT /agents/{agent_id}/skills/{skill_name}`

Sessions and workflow:

- `GET /sessions?agent_id={agent_id}&user_id={id1,id2}` lists sessions by
  optional agent and optional comma-separated user filter. Omit `user_id` for
  all users.
- `POST /sessions` creates a session owned by the authenticated user.
- `GET /sessions/{session_id}`
- `POST /sessions/{session_id}/handoff`
- `GET /workflow-bindings`

Runtime:

- `GET /runtime-templates`
- `GET /logs`
- `GET /events` as SSE

The frontend build regenerates TypeScript types from `openapi/openapi.json`.
