# System Admin

Roles:

- `admin`: manage users, roles, settings, RBAC, runtime infrastructure and all
  sessions.
- `operator`: manage agents, leaders, executors, runtime, config, skills,
  deployments, logs and all sessions.
- `user`: create and use own sessions; read safe agent directory.

Admin tasks:

- Promote/demote users in `/settings?tab=users`.
- Review role changes in `/logs?tab=audit`.
- Configure runtime roots and sources in `/settings`.
- Configure ports and auth in `/settings`.
- Review all sessions through the user filter.

Compatibility:

- `is_system_admin` remains stored for older code paths.
- Effective admin status is `system_role = admin`.
- Access tokens now include fleet-compatible issuer/audience claims. Keep
  `mode=hmac` until `sdlc-auth-core` OIDC/JWKS validation is implemented.
