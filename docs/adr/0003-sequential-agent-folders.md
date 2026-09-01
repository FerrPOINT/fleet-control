# ADR 0003: Sequential Agent Folders

Decision: allocate agent folders from the database ordinal and materialize them
as `agent1`, `agent2`, and so on.

Reason: operators need predictable folder names, while PostgreSQL remains the
source of truth for identity and ordering.
