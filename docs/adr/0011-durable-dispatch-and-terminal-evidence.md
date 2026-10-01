# ADR 0011: Durable Dispatch And Terminal Evidence

## Status

Accepted and implemented for the Fleet foundation. Operator recovery and
cross-service fencing remain required before automatic SDLC is enabled.

## Context

A request can time out after Hermes has accepted it. Retrying blindly risks two
runs. An SSE connection can end before completion. Neither an HTTP acknowledgement
nor stream EOF proves a successful task result.

## Decision

- Persist the user message and dispatch outbox entry transactionally.
- Serialize replay/conflict checks and reserve agent capacity under database
  locks. Pass the message ID as Hermes `Idempotency-Key`.
- Claim only runnable, non-drained agents. Unknown acceptance holds capacity;
  it is not automatically redispatched.
- Require a terminal runtime event or terminal status readback after EOF.
  `interrupted` is failure, not an assistant response or success.
- Store terminal assistant messages once, using runtime message identity and
  a session lock to prevent duplicate mirrors.
- Persist Fleet stream events with ordered per-session cursors. Reconnect uses
  `Last-Event-ID`; authorization is rechecked during polling.
- Keep Hermes SessionDB private to Hermes. Fleet only uses the adapter protocol.

The reviewed Hermes source provides durable idempotency, but installed runtime
version/capability and recovery behavior still need live acceptance evidence.
Fleet therefore remains conservative about unknown submissions.

## Consequences

- A duplicate user command does not produce another known dispatch.
- A crash or uncertain request can require operator reconciliation.
- EOF cannot fabricate completion.
- Stream replay survives a Fleet restart, but retention/reset snapshots and
  bounded transcript pagination remain follow-up work.
- A terminal run still does not complete an SDLC stage without workflow receipts.

## Alternatives

- Retry every transport failure: rejected because acceptance may be unknown.
- Treat EOF as success: rejected because transport closure is not evidence.
- Write directly to Hermes SQLite: rejected because it bypasses runtime behavior.
