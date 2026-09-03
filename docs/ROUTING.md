# Routing

Route groups:

- `/login`, `/register`: authentication.
- `/`, `/dashboard`: operator fleet overview.
- `/leaders`: product view for leader agents.
- `/executors`: product view for executor agents.
- `/agents`: technical runtime inventory.
- `/sessions`: user-owned chats/tasks.
- `/workflows`: namespace/workflow bindings.
- `/deployments`: runtime sources, versions and jobs.
- `/logs`: process logs, events and audit trail.
- `/settings`: runtime roots, ports, integrations, auth and users.
- `/access-denied`: permission evidence state.
- `*`: not found evidence state.

Permission routing:

- Users without `agents:manage` land on `/sessions`.
- Admin/operator-only routes use frontend permission gates and backend RBAC.
- Executor tab routes mirror agent technical tabs and are documented aliases.
