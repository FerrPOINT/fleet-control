# ADR 0003: Sequential Agent Folders

## Status

Accepted.

## Context

Operators need predictable on-disk folders for runtime, config, workspace and
logs. At the same time, allocation must remain race-safe and auditable.

## Decision

Allocate agent folders from the database ordinal and materialize them as
`agent1`, `agent2`, and so on.

## Consequences

- PostgreSQL remains the source of truth for identity and ordering.
- Folder names are stable enough for runbooks and support.
- Provisioning must verify `.fleet-agent.json` markers before reusing folders.

## Alternatives

- Use random UUID folder names. Rejected because support and manual inspection
  become harder.
- Let the filesystem allocate names. Rejected because concurrency and audit
  behavior become fragile.
