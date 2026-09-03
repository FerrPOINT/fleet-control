# ADR 0007: Leader Delegation Sessions

## Status

Accepted.

## Context

A user may chat with an IT lead and ask that leader to coordinate development
and testing. The leader needs to create executor work without turning the parent
chat into a shared transcript for every executor.

## Decision

Leader coordination uses parent and child Fleet sessions.

A leader chat is the parent coordination thread. When the leader creates a task
for an executor, Fleet creates a child session with `parent_session_id` and
inherits `leader_agent_id`.

## Consequences

- Child sessions keep delegated executor work explicit and auditable.
- Private executor chats remain isolated.
- The UI must show parent/child relationships and delegation actions.

## Alternatives

- Add all executors as participants in the leader chat. Rejected because it
  exposes too much by default and makes executor run ownership unclear.
- Use handoff only. Rejected because delegation creates parallel team tasks,
  while handoff transfers a single session's primary agent.
