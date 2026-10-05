# ADR 0023: Logical Journal Progress Time

## Status

Additive migration 000014 implemented; see the
[verification ledger](../CHAT_CLARIFICATION_VERIFICATION.md#journal-clock-order-repair).
This is not a trusted-clock or automatic retry guarantee.

## Context

PostgreSQL clock_timestamp can move backwards. The original guard sets submission
and acceptance timestamps using that clock, then check constraints reject progress
before creation/submission. An actual broad gate observed check4 on verified ACK
commit; a deterministic forward-created fixture reproduces check3 on submission.

## Decision

Keep historical migration 000012 and its identity/ACK/expiry guard byte-identical.
Install a second BEFORE UPDATE trigger ordered after that guard. Floor a newly
observed submitted_at to immutable created_at and accepted_at to submitted_at.
These are logical progress times, not precise observed wall-clock timestamps.
Do not change creation, deadline, request/key/hash, run identity or send permits.
Require an empty journal for downgrade; leave retained receipts untouched.

## Consequences

Valid progress no longer rolls back solely because the observed clock regresses.
Original expiry checks and immutable recovery horizon remain. The wall-clock
recovery window still depends on environment clock integrity; this migration does
not prove native SQLite continuity, renew TTL or authorize an unknown redispatch.
Release 000014 after 000010/000011/000012/000013, with at most one new migration
per task PR. Test fresh/upgrade/down/reapply, original guard/history equality,
trigger order and refusal of nonempty downgrade. No applied bytes are rewritten.

## Alternatives

- Rewriting applied migration 000012 would invalidate migration provenance.
- Dropping time constraints would lose logical ordering for journal consumers.
- Retrying with a new key after failed ACK commit risks duplicate execution.
- Repairing host clocks is also needed, but cannot replace durable record guards.
