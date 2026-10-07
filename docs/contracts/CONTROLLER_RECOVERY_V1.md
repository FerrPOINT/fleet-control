# Controller Recovery V1

Status: internal Fleet storage candidate. Native handover and actual restart
acceptance are not implemented by this storage contract. No public endpoint,
automatic recovery worker, installed opt-in or new model dispatch is enabled.

## Original Identity And Epochs

`runtime_launches` remains the immutable original launch/controller/configuration
binding. Additive migration `000020` stores separate `runtime_controller_recoveries`
epochs, without backfilling historical agents or rewriting original registrations.
The request fixes UUIDs for command, launch, agent, original/proposed controller
and nullable predecessor; original launch/mapping/registration SHA256; current
physical controller snapshot; and original acknowledged agent PID. It contains
no raw prompt, dotenv, native secret or transcript. Unknown JSON fields are denied.

Only an original mapped policy3 Hermes launch in `gateway_started` can reserve.
The database checks the exact original hashes, controller, agent PID, immutable
physical container/inventory and a distinct controller `StartedAt`. This is a
storage CAS check against a caller-supplied private witness, not an Engine probe.
The production caller must obtain and revalidate Base's original restart witness;
an HTTP health response, caller assertion or PID alone is not that witness.

The first epoch is1. Subsequent epochs require the exact current predecessor,
an acknowledged prior record, expired prior lease, different logical controller
and distinct physical controller start. The old acknowledged epoch becomes
`superseded` atomically with insertion of the new reservation. An unresolved
`reserved` record cannot be replaced merely because its lease expired.

## Lease, Replay And Acknowledgement

The same agent-row lock serializes claims with launch, queue, configuration and
runtime mutations. Unique launch/epoch and one-current-epoch indexes provide
cross-replica fences. A lease lasts at most30 seconds on PostgreSQL's clock.
Heartbeat requires the exact controller and current lease version; it increments
the version and cannot revive an expired lease. Intended worker cadence is10
seconds, but the worker is not yet connected. No caller-supplied clock is trusted.

Identical command/request replay returns the original epoch and expiry without
renewal or native action. Changed payload for that command conflicts. Readback
remains historical after expiry or supersession; `lease_valid` is only a snapshot,
not permission to perform an effect. Unknown commit outcomes require original
command readback rather than a new command ID.

The internal acknowledgement operation stores one original native receipt hash
under exact controller/lease-version CAS before expiry. Identical receipt replay
is read-only; a changed receipt conflicts. Storage acknowledgement does not
validate a raw Base response or restore runtime custody by itself. The future
caller must verify a closed, original-generation Base handover receipt and its
durable private epoch journal before invoking this operation.

## Current Effect Fence

Any recovery record fences the old runtime owner. Outstanding launch readback
exposes a private `controller_recovery` marker, not a public DTO. Original
supervisor generation checks, endpoint writes, lifecycle observations, runtime
metadata updates, controller-scoped/unscoped queue claims and dispatch permit
preparation/consumption reject that outstanding generation while recovery exists.
Even a stored acknowledgement does not open a new-owner path in this candidate.
Original messages stay queued, launch/PID/configuration history stays unchanged
and no replacement generation or model request is created.

Database triggers reject identity/receipt relabelling, illegal transitions,
unbounded/stale heartbeat, delete and truncate. Retained history prevents
downgrade; an empty clean-schema down/up is allowed. There is no force-unlock,
clear-history or reset-owner operation.

## Required Native Integration

Complete the same flow, not a second ownership scheme:

1. Retain the original command identity privately before reserving; revalidate
   original source/configuration, stopped old controller and Base restart witness.
2. Issue a closed Base handover command bound to this exact DB epoch/lease and
   original generation. Base must preserve its original mapping/start journals
   and record a separate immutable private owner epoch before granting effects.
3. Reconcile an unknown Base acknowledgement by original read-only GET. Expiry
   or EOF must never allocate another epoch or resend an uncertain effect.
4. Verify the exact native receipt, commit its hash, then require fresh live DB
   lease and Base epoch checks at every queue/permit/lifecycle/control effect.
   Do not authorize from `lease_valid`, cached ACK or an in-memory owner map alone.
5. Recover original dispatch/control/approval and activation checkpoints before
   admitting new work. Confirm safe namespace exit before replacement or rollback.
6. Prove actual Fleet/Base/Docker restart with an ongoing Hermes run, competing
   controller denial, unknown acknowledgement, expired owner, interrupted
   activation and retained journals. Lost storage/source upgrade/recreated
   controller need explicit recovery rules; no fallback is provided here.

See [implementation plan](../IMPLEMENTATION_PLAN.md),
[launch contract](RUNTIME_LAUNCH_JOURNAL_V1.md),
[Base witness](CONTAINER_CONTROL_V1.md#original-controller-restart-observation)
and [verification ledger](../CHAT_CLARIFICATION_VERIFICATION.md).
