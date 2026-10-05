# ADR 0025: Original Approval Outcome Journal

## Status

Implemented internal journal and opt-in approval HTTP sender/GET worker. Actual
native once/deny/lost-ACK GET recovery verified through the control plugin.
Separate native approval recovery after two SIGKILLs/three Fleet PIDs is verified.
Combined extensions and installed release remain pending.

## Context

An approval request can become cancelled after a run ends while its decision ACK
is lost. Request lifecycle is not proof of the decision's acceptance. A terminal
run or fresh producer epoch cannot establish the original action retrospectively.

## Decision

Use additive000016 without editing applied predecessors. Fix original mode at
reservation, then persist exact closed action bytes/hash and original producer
context with one transactional claim. Recheck active actor and accepted concrete
free-chat scope before claiming. Never backfill legacy unknown decisions.

Commit witnessed ACK, delivered decision and audit together under session-first
locking. An already cancelled/settled request and terminal run stay unchanged.
Deferred DB guards prevent partial claim/context or ACK/receipt/audit commits;
claimed uncertainty cannot be released as failed. Recovery pages include those
historical cancelled requests. Original decision/context history is immutable;
nonempty downgrade refuses deletion.

## Consequences

The connected consumer verifies the exact pending native request before claim,
sends saved bytes/UUID/epoch once and recovers only original-context GET witnesses.
The flag is default-off; opted-in preflight never falls back to legacy POST. Internal
completion must be called only after closed native witness verification; it
grants no new runtime authority after revocation. Unknown effect without durable
native ACK remains held. Legacy behavior and installed flags stay unchanged.

## Alternatives

- Mark decision accepted from terminal/absence of pending request: rejected,
  neither proves the exact original choice.
- Give old uncertain decisions a fresh epoch: rejected, not original evidence.
- Resolve or recreate a cancelled request on late ACK: rejected, rewrites history.
- Direct Hermes SessionDB edits or duplicate approval POST: rejected, breaks
  runtime ownership and single-send semantics.

See [outcome contract](../contracts/HERMES_CONTROL_OUTCOME_V1.md) and
[data model](../DATA_MODEL.md#runtime-approval-outcomes).
