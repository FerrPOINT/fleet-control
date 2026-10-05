# ADR 0022: Current Native Approval Snapshot

## Status

Implemented; actual two-Fleet-process native evidence is recorded in the
[verification ledger](../CHAT_CLARIFICATION_VERIFICATION.md#current-approval-snapshot-recovery).
Ordered release and exact-head validation remain prerequisites.

## Context

Fleet can exit after a native run reaches its terminal-tool approval guard but
before receiving the approval event. Opening a second SSE consumer would not
prove missed-history completeness and can race with an existing stream.

## Decision

Recover only the current pending approval from authenticated run-status GET.
Require the original accepted free-chat journal, current concrete agent/origin/
credential, pinned native run/session, fresh approval capabilities and a bounded
exact request. Atomically lock agent, primary session and run; insert one redacted
request and move running to waiting. Preserve resolved, stopping and terminal
states. Existing triggers generate the durable event; GET creates no transcript
message or historical tool event. Identical replay changes neither history nor
event cursor. Task/PM runs remain outside this path until admission is proven.

## Consequences

The owner can decide on the original real action after Fleet restarts, without
a second run, stream or decision POST. The status document is not an approval
queue: completed/historical requests and unknown decision outcomes still require
their own protocol. Native GET is not configuration-generation, central identity,
assignment fencing or safe descendant-stop evidence. No schema/API DTO change is
needed for this recovery; migration 000014 fixes an independent journal defect.

## Alternatives

- Re-dispatching the prompt can execute the task twice.
- Treating stream EOF as completion loses pending actions.
- Reading or writing Hermes SQLite breaks runtime ownership.
- Fabricating an approval from model prose does not identify a native action.
