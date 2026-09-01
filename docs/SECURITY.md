# Security

- Authentication uses JWT access tokens and HttpOnly refresh cookies.
- The first registered user becomes system admin.
- Filesystem access must be derived from database-managed agent paths.
- Reject `..` traversal and never operate outside the configured agents root.
- Secret-like env/log values are redacted before persistence and API return.
- Physical folder purge is not part of default delete; agents are archived and
  stopped first.
