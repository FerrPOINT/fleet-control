# Database Standards

Standards:

- PostgreSQL is the source of truth for agents, sessions, users, audit and job
  state.
- Runtime-local databases are not Fleet Control write targets.
- All user-visible identifiers are UUIDs except the operator-friendly `agentN`
  ordinal name.
- Timestamps are UTC in storage and formatted by the UI.
- JSON columns are accepted only for structured metadata, settings and redacted
  previews where schema churn would be high.
- Core relationships use foreign keys.
- Domain enum fields use checked text values.
- Soft archive is the default destructive action for agents.

Repository code should map database rows into domain DTOs at the boundary and
avoid leaking SeaORM models into API handlers.
