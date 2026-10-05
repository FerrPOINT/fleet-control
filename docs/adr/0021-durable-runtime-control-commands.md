# ADR 0021: Durable Runtime Control Commands

## Status

Implemented in the source worktree; final packet verification and ordered release
are required. Native per-command recovery and installed acceptance are not implied.
The component and supplementary Rust/PG gates pass370 distinct cases; see the
[verification ledger](../CHAT_CLARIFICATION_VERIFICATION.md#durable-runtime-control-journal-5-october-2026).

## Context

Native steer/stop ACKs can be lost after an effect. A frontend retry, Fleet restart
or new request key must not create duplicate guidance/interrupt or false completion.
Native run submission idempotency does not provide control-command idempotency.

## Decision

Use additive migration 000013 after dispatch journal 000012. Authenticate the actor,
reserve immutable operation/semantic hash and original context, and consume one
durable submitted permit before HTTP. Identical actor/key/payload replays the receipt;
conflicting input returns 409. A partial unique index holds unresolved commands per run.
Revalidate current actor, primary run and context before the claim.

Commit a validated ACK, audit, session event and nonterminal stopping atomically.
Preserve submitted/uncertain after unknown outcome. Read receipts with session/project
authorization. Terminal reconciliation uses the independently committed original
run/prompt packet; terminal_observed preserves unknown command acceptance.
UI freezes uncertain guidance and consumes receipts after reload, not just mutation
memory. Do not blindly retry native POST, mark task success or release process safety.
The mandatory v1 command header is an explicit security migration for legacy
unkeyed clients, checked by a closed compatibility wrapper rather than claimed
as backwards-compatible. See [API versioning](../API_VERSIONING.md).

## Consequences

Duplicate/unknown controls are safe across Fleet restarts but can hold a run until
terminal evidence. Raw guidance is not stored in the ledger: a reserved replay needs
the original client input/key. Safe explicit cancellation of abandoned reservations
and native acceptance lookup remain separate work. No task control is admitted until
assignment/fencing/first-step contracts are implemented. Release 000013 follows
000010/000011/000012 in separate ordered packets; nonempty downgrade is refused.

## Alternatives

- Browser-only keys lose protection on reload and across callers.
- Automatic POST retry has no proven native idempotency contract.
- Run status or EOF cannot identify whether guidance was accepted.
- Writing Hermes SQLite would violate adapter ownership and does not prove execution.
