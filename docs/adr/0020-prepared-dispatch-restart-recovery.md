# ADR 0020: Prepared Dispatch Restart Recovery

## Status

Implemented candidate; component gates and managed runtime acceptance are distinct.

## Context

Fleet can stop after an immutable request journal commits but before its durable
submission permit. The outbox is already dispatching, so ordinary pending dequeue
does not recover it. Resetting every dispatching/uncertain record would risk
duplicating a run whose HTTP acceptance is unknown. Historic records without a
journal cannot prove that no side effect happened.

## Decision

Use a separate bounded UUID-keyset worker for exact free-chat journals that remain
prepared, have no submitted timestamp/native ID and a live original DB horizon.
It does not create runs, reconstruct prompts, reset keys or renew horizons.
Before claiming, verify original origin/credential/request proof and fresh native
health/protocol facts, including the original opt-in store epoch when present.
The existing transactional claim rechecks agent/session/run/message identity,
drain, capacity and current deadline under the same locks. Only its winner can
perform the one original POST, through the same submission/ACK/session readback
path as the normal dispatcher. A prepared uncertain outbox may become dispatching
only in that permit transaction. Submitted/accepted receipts never reset.

The readback worker remains separate and never submits runs. Unknown HTTP
acceptance after the permit remains held and may only use compatible original-key
readback. No-journal, failed delivery, expired, archived, drained, task-bound or
PM records are excluded. This is technical dispatch recovery, not an assignment
queue, a second scheduler or task admission. Java chat/control remains unchanged.

## Consequences

Two original/recovery workers may observe one prepared intent, but only one wins
the DB permit. A crash after permit commit but before HTTP is intentionally
unknown: a negative lookup cannot authorize another send. Changed credentials,
origin/capabilities or unavailable prerequisites do not consume a permit. The
original request, model/options, run/key and recovery deadline remain immutable.
Failed or stale records stay available for explicit reconciliation; no automatic
capacity release or destructive repair is added. Managed runtime/config/OS proof
and task/PM admission remain independent release gates.

## Alternatives

- Reset all dispatching/uncertain outboxes to pending: rejected; may repeat an
  unknown POST or adopt legacy records without original proof.
- Recreate a run/key or serialize a current prompt: rejected; loses the immutable
  original request and permits divergent side effects after restart.
- Let the acceptance worker POST when native ID is missing: rejected; prepared
  initial submission and submitted unknown acceptance require different authority.
- Wait forever for operator handling of every prepared record: rejected; the
  original durable unconsumed permit allows safe automatic initial delivery.
