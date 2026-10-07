# ADR0033: Original Controller Delivery And Historical Outcome

## Status

Accepted for the internal candidate; installed rollout and automatic recovery are
not accepted. Builds/tests and native acceptance have separate evidence scopes.

## Context

Migration000020 stores fenced owner epochs, but a stored hash alone does not prove
native custody. A native ACK can be lost or arrive after the DB lease expired.
Retrying the handover can duplicate a side effect; discarding the reservation can
allow a competing owner while native state is unknown.

## Decision

Add one migration000021 with a private immutable initial command, canonical hash,
once-only dispatch claim and native receipt. Commit the claim before the sole native
call. Unknown acceptance becomes original-key readback, never automatic resend.
Validate the closed receipt against the original launch and witness, then commit
receipt, historical acknowledgement and redacted audit atomically.

Allow an exact claimed historical ACK after lease expiry without changing the
lease version/deadline or granting runtime effects. Keep000020 original-owner
fences. A successor still requires settled original history, expiry and distinct
physical restart. Do not automatically invoke recovery or enable a new-owner run.

## Consequences

Crash before the sole native call can leave a claimed reservation held indefinitely;
this is intentional until a safe explicit reconciliation policy exists. Private
history cannot be reset/deleted; populated downgrade is refused. Source pinning,
both live leases and actual ongoing Hermes restart tests remain release gates.

## Alternatives

- Retry an unknown call: rejected because absence of ACK is not non-acceptance.
- Renew an expired lease while saving ACK: rejected because history is not authority.
- Rewrite the original launch owner: rejected because it erases the custody anchor.
- Treat cached native receipt as a live permit: rejected because it cannot fence
  a subsequent physical restart or expired owner.

See [contract](../contracts/CONTROLLER_RECOVERY_V1.md) and
[verification](../CHAT_CLARIFICATION_VERIFICATION.md).
