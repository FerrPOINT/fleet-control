# ADR 0015: Accepted Run Session Readback

## Status

Accepted source design for free chats. Task-bound/PM admission and native live
acceptance remain separate gates; see the verification ledger.
Original-context recovery now additionally requires the private journal in
[ADR 0016](0016-hermes-original-request-journal.md). Legacy history compatibility
does not authorize automatic probing with current credentials.

## Context

Hermes HTTP 202 identifies an accepted run, not its effective persistent session.
It can resolve a Fleet alias to another ID. A status failure after acceptance
must not lose the run or cause another submission, and concurrent recoverers
must not both start the same stream worker.

## Decision

Commit ACK run ID, prompt delivery and dispatch outbox together in PostgreSQL.
Keep the prepared run pending and agent capacity occupied until authenticated,
bounded status GET validates object/run/session/status. Pin the effective session
once under row locks. The first pending-to-running transition is the only
stream-start winner; identical pins replay without writes or state regression.

A bounded 20-row keyset worker recovers accepted-but-unpinned free-chat runs,
advancing past failed reads and wrapping after the final page. It never POSTs,
creates a run or repairs an unknown acceptance. Generic updates cannot replace
an accepted runtime ID or regress terminal state. EOF terminal readback must
identify the pinned effective session as well as the run.

Delivery CAS preserves an ACK after a post-commit result-read failure. Missing
native IDs cannot regress that delivery; conflicting native IDs fail before any
update. Terminal replay is read-only. Controls reload current run identity and
refuse pending pin state before HTTP, independently of the caller's cached state.

Task-chat and PM bindings are excluded in both repository and runtime paths.
Even valid durable capabilities cannot authorize a task-bound prompt before
the separate authoritative admission/first-step implementation.

## Consequences

Readback outage/restart retains the ACK and capacity, and later recovery need
not send another prompt. Unknown acceptance before ACK commit, exact request/
credential/profile/horizon journaling, stream recovery after the pin-to-worker
gap and native process-tree quiescence still require implementation. A run result
is not Workflow completion, a Tracker transition or SDLC readiness.

Stop/steer/approval are temporarily unavailable while an accepted run awaits
session readback. This prevents a control update from dropping it out of the
pending recovery queue; it is not a final offline-stop solution. Independent
pin/control state and durable command reconciliation remain required.

No migration or public API field is added. Internal semantics distinguish
pending without acceptance from pending with a durable run ID; legacy running
records are not retrospectively attested by this worker.

## Alternatives

- Assume the requested alias is effective: rejected; Hermes resolves sessions.
- Persist ACK after GET: rejected; an outage would discard known acceptance.
- Recover through POST: rejected without the exact durable replay proof.
- Start a worker for every identical pin: rejected; concurrency duplicates work.
- Treat wire capabilities as admission: rejected; they do not own assignments.
