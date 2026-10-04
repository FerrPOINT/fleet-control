# Hermes Run Control Consumer Profile v1

This is Fleet's consumer policy for the pinned native API, not a new Hermes
endpoint or permission to run SDLC. See [runtime](../RUNTIME.md) and the
[verification ledger](../CHAT_CLARIFICATION_VERIFICATION.md).

## Before A Side Effect

- Resolve the current Fleet run, not caller-supplied state. Require exact agent,
  session, native run and pinned native session IDs. Pending/terminal runs cannot
  receive controls; steer requires running, stop allows running/waiting/stopping.
- Require the original accepted dispatch journal, exact original request hash,
  origin and derived-credential fingerprint. Legacy/unknown context fails closed.
  The private by-run journal lookup observes the receipt and run in one query;
  it does not renew a lease, horizon, idempotency key or dispatch permit.
- Reject task-bound controls until assignment/control admission is integrated.
  Public HTTP ownership remains authoritative; this adapter is not human auth.
- Fresh authenticated health/capabilities must advertise the exact POST endpoint
  and feature. Authenticated bounded GET verifies the original run/session and
  eligible native status before POST. No redirect, proxy-env fallback or retry.

## Acknowledgement And State

POST uses identity encoding and a ten-second request/body deadline. Require exact
HTTP200, JSON MIME, unencoded JSON and at most64KiB. Reject malformed, oversized,
foreign or negative acknowledgements; do not expose arbitrary upstream bodies.

Steer requires `object=hermes.run.steer`, original `run_id`, `accepted=true`.
It acknowledges queued guidance, not its use in a later model turn. Fleet reads
its current state without writing running: a concurrent waiting/stopping/terminal
transition cannot be reset by that acknowledgement.

Stop requires the original `run_id` and `status=stopping`; an optional session ID
must match the pin. Only stopping is persisted. If native completion races POST,
the returned full `hermes.run` status must match both identities and native
terminal/completion flags. Fleet does not fabricate a terminal mirror or replace
its local state with stopping from that response; the terminal worker still
provides the independent durable commit. Neither ACK releases run capacity or
proves descendant/process-tree quiescence, task success or Workflow completion.

Unknown acceptance, stale session/status or invalid response keeps the original
run/journal/capacity. Automatic retry is forbidden. Durable per-command receipts
and independent control-outcome reconciliation remain release requirements.

## Approvals And Phase 2

Run-wide approval is retired at both public API and adapter boundary. Never use
`always`, session grants or `resolve_all` as a substitute for an exact human
decision. [Targeted approval](../API.md) keeps its separate durable request and
decision flow; native approval/replay acceptance is not established by stop/steer.
Java chat/control remains phase2. Native run controls do not terminate the gateway
or attest operating-system isolation and safe stop.
