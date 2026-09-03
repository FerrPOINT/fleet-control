# ADR 0005: Private Session Default

## Status

Accepted.

## Context

Users need to work directly with executors without automatically exposing every
task transcript to a leader. At the same time, leader-managed work must be
visible and writable by the selected leader.

## Decision

Human-created executor chats are private by default.

Private sessions store `leader_agent_id = NULL` and `visibility = private`.
Selecting a leader changes visibility to `leader_scoped` only after backend
validation proves that the leader manages the primary executor.

Direct chats with a leader are leader-scoped by default because the primary
agent and selected leader are the same agent.

## Consequences

- One session has at most one selected leader.
- Private chats are not readable or writable through leader permissions.
- The UI must show leader/private badges and a leader selector per session.

## Alternatives

- Auto-share all executor chats with assigned leaders. Rejected because it
  violates private-by-default behavior.
- Allow multiple leaders per session. Rejected for MVP because write authority,
  audit and handoff semantics become ambiguous.
