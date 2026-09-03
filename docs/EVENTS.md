# Events

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
