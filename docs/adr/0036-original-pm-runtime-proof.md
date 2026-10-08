# ADR0036: Original PM Runtime Proof

## Status

Accepted for defensive source hardening. Component verification is recorded
separately; PM dispatch/admission/resume and installed acceptance remain open.

## Context

An immutable run/session pair is insufficient when readback selects today's
listener and credentials. A replacement runtime may return those same IDs.
Lifecycle exclusion also cannot prevent physical child exit while a transaction
waits for database locks. Replayed terminal writes can append duplicate durable
events even when audit insertion is once-only.

## Decision

Seal the original launch/controller IDs, origin and credential fingerprint in
each new private PM reservation. Capture it from the owning supervisor. Require
matching persisted provenance at reservation and observation. Never backfill
legacy missing bindings; keep history readable but refuse new proof.

Use the sealed origin for authenticated status GET. Verify physical custody,
current origin and credential fingerprint before and after HTTP. Keep lifecycle
exclusion through persistence. After the observation transaction has acquired
agent, PM binding, launch/runtime and visible-run locks, ask the supervisor for a
fresh custody verification. Only then persist the observation and visible state.
This is an observation boundary, not proof that a process can never exit later.

An identical terminal replay validates the binding and visible mapping, then
returns without timestamp/event/audit mutations. Contradictory terminal results
remain conflicts. Private binding fields do not extend the public flat callback.
No runtime SQLite writes, schema migration or business-stage completion is added.

## Consequences

Legacy and foreign-controller records retain unresolved capacity instead of
adopting a current listener. A controller restart requires the separate verified
recovery protocol, not automatic PM proof takeover. The transaction may hold
locks during a bounded custody check; it must not call lifecycle mutations.
The in-transaction verifier must not resolve/register a container endpoint in a
second transaction: that would wait for its own agent lock. Endpoint provenance
is already verified under the outer transaction; physical container observation
and credential/controller verification are read-only with respect to Fleet DB.
The repository passes the locked launch and PID to this verifier. It must not
read through the pool either: one connection or concurrent pool exhaustion would
otherwise make the transaction wait for itself. Agent kind/role, paths/config,
origin and desired state are validated by the owning transaction; the verifier
checks retained local launch, process/container custody and credential identity.
Local custody map locks are nonblocking inside this transaction. A busy global
launch/child map returns unavailable and retains capacity; it does not wait while
holding a pooled connection that another lifecycle writer may need. A subsequent
fresh callback GET can retry this observation, never a prompt dispatch.
Tests distinguish synthetic repository witnesses from a physical retained child
and from actual Hermes/PM acceptance. Predispatch authority, loaded inventory,
safe descendants and workflow continuation are still independent gates.

## Alternatives

- Current listener plus pinned IDs: rejected because incarnation is not bound.
- PID-only recovery: rejected because PID reuse is not original custody.
- Physical check before the transaction: rejected because lock waiting can make
  that check stale before proof commits.
- Rewriting terminal state on replay: rejected because it changes durable history.
