# System Admin

Roles:

- `admin`: manage users, roles, settings, RBAC, runtime infrastructure and all
  sessions.
- `operator`: manage agents, leaders, executors, runtime, config, skills,
  deployments, logs and all sessions.
- `user`: create and use own sessions; read safe agent directory.

Admin tasks:

- Open Admin Panel from `/settings?tab=users` to manage users and platform roles.
- Review role changes in `/logs?tab=audit`.
- Preview managed runtime, agent-port, integration, auth and retention changes
  in `/settings`, confirm the diff and let the process restart under its
  supervisor. Every apply and rollback creates an immutable audited version.
- Roll back a bad managed configuration by selecting a previous version; the
  rollback itself becomes a new active version.
- Change database credentials, service secrets, backend/frontend bind ports and
  host/container port mappings through `FLEET_CONTROL_*` deployment
  configuration. Those infrastructure-owned values are never stored in the
  managed snapshot.
- Manage human access and platform authentication in Central Auth/Admin Panel.
- Review all sessions through the user filter.

Compatibility:

- `is_system_admin` remains stored for older code paths.
- Effective admin status is `system_role = admin`.
- Access tokens include fleet-compatible issuer/audience claims. OIDC mode
  still requires its issuer/JWKS baseline configuration before it can be
  selected in a managed version.
