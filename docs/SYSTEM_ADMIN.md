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
- Inspect effective runtime, port, integration and legacy auth startup values
  in `/settings`.
- Change runtime roots, sources, ports and Project Workflow connection through
  `FLEET_CONTROL_*` deployment/environment configuration, then restart or
  redeploy Fleet Control.
- Manage human access and platform authentication in Central Auth/Admin Panel.
- Review all sessions through the user filter.

Compatibility:

- `is_system_admin` remains stored for older code paths.
- Effective admin status is `system_role = admin`.
- Access tokens now include fleet-compatible issuer/audience claims. Keep
  `mode=hmac` until `sdlc-auth-core` OIDC/JWKS validation is implemented.
