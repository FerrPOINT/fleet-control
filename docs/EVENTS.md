# Events

## Durable Runtime Control Events

Migration 000013 controls persist `runtime_control.changed` in the existing
per-session cursor transaction. SSE payload type is `runtime_control_changed`
with `command_id`, `run_id` and action only. Reservation, submitted permit,
ACK/rejection/uncertainty and terminal reconciliation are audited in that same
transaction. It contains no guidance/key/token. Clients invalidate scoped command
history on the event; the event itself does not assert acceptance or completion.
ACK replay and completed reconciliation create no duplicate cursor/event.

Fleet Control emits events for:

- agent create/update/archive
- provisioning and runtime lifecycle
- config and skill changes
- leader team binding changes
- session create/update/handoff/delegation/message
- deployment job create/cancel/state transition
- settings changes

Persistence:

- `agent_events` stores operator-visible event stream entries.
- `audit_log` stores durable security/audit records for mutating actions.
- `agent_logs` stores runtime process output.

Delivery:

- `/api/v1/events` streams SSE for operator/admin UI.
- `/api/v1/events/recent` returns a bounded JSON list for logs screens and
  screenshots.

Payloads must be redacted before persistence and before API return.

## Task Event Projections

The working PM feature provides a transactional Tracker inbox repository. Its
input is a bounded, typed v1 outbox page from a trusted gateway, never a browser
POST containing arbitrary events. It verifies the configured instance, project,
task/root and owner against the immutable chat binding before storing a receipt.

Each new source event produces a compact `system_event` transcript entry and a
durable `tracker_event` session invalidation. Result bodies and credentials are
not copied into either payload. In particular, `clarification.answered` means
the answer was saved in Tracker, not that PM received it. Source event IDs and
payload hashes prevent duplicate projection after concurrency or reconnect.

The source cursor advances atomically with the entire page. Global sequence gaps
are allowed; reordered events, changed replay payloads and unsupported event
types fail closed. Background polling, outbox-to-PM delivery and continuation
remain open integration work. No public ingestion route has been introduced.
