# Controller Recovery V1

Status: internal Fleet recovery candidate. Migration000021 adds durable native
delivery and a trusted supervisor entry point on top of000020. No public endpoint,
automatic startup worker, installed opt-in or new model dispatch is enabled.
Base native protocol3 is published separately at
[3facb28](https://github.com/FerrPOINT/services-base/commit/3facb289d449c6a9a2a3863e31a661a661235299).
The SDK/source pins are unchanged; this commit is not automatically installed.

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

The legacy internal acknowledgement operation stores one original native receipt hash
under exact controller/lease-version CAS before expiry. Identical receipt replay
is read-only; a changed receipt conflicts. Storage acknowledgement does not
validate a raw Base response or restore runtime custody by itself. The future
caller must verify a closed, original-generation Base handover receipt and its
durable private epoch journal before invoking this operation. The new supervisor
flow uses the atomic delivery/outcome operation below, not this legacy hash-only
operation as proof of native custody.

## Durable Native Delivery And Historical Outcome

`runtime_controller_recovery_deliveries` retains the exact initial closed command:
`request`, `epoch`, `lease_version=1`, and original `lease_expires_at`. Its canonical
SHA256 is immutable. This body is distinct from subsequent DB heartbeat versions;
it is the original-key identity used for readback, never a renewed live permit.

Before the native command, Fleet commits a once-only `dispatch_claimed` bit under
the same agent-row lock. Only the first claim may call Base `recover_controller`.
All errors and later invocations use `read_controller_recovery` with the exact
retained body. A crash after claim but before the native call stays held if no ACK
exists. Absence, timeout, EOF or lease expiry never resets the bit or redispatches.
These Base actions use the existing trusted stdin/stdout helper protocol3,
not a new HTTP endpoint. References to original-key GET describe read-only
reconciliation, not an HTTP route implemented by this helper.

Fleet validates the closed native receipt, command/hash, original registration
and launch, physical controller snapshot, original agent PID and restart witness.
Receipt bytes/hash, acknowledged owner state and one redacted audit row commit in
one transaction. An exact replay is read-only; a changed outcome conflicts.
The command is bounded to16KiB and native receipt to64KiB. Neither body is public.

A positive historical ACK may be saved after DB lease expiry, but only for its
previously claimed original command. This operation leaves the lease version and
deadline unchanged. It does not revive authority, release capacity, change the
original launch, bypass old-owner fences or enable a model/run. A successor still
requires the acknowledged expired predecessor and distinct physical restart.

The trusted `recover_container_controller` entry obtains the Base restart witness,
reserves/retains/claims once, then reconciles by original readback. It is not exposed
through HTTP or called automatically by the reconciler. Startup policy, dual
DB/native heartbeat and fresh effect admission, interrupted activation settlement
and actual ongoing Hermes acceptance remain required before automatic enablement.

Migration000021 is additive. Delete/truncate, identity relabel, claim reset and
receipt replacement fail. Downgrade refuses if either ownership or delivery history
is nonempty. Only when both are empty does it restore the exact000020 schema/guard.

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

1. Connect an explicit startup recovery policy and current-source compatibility
   to the trusted entry point. A missing original journal or changed physical
   container must stay held; source upgrade is not an implicit recovery action.
2. Maintain both DB and native leases at10-second cadence with fresh exact-owner
   version checks. Preserve historical command identity across every heartbeat.
3. Reconcile interruption between DB claim/native acceptance/receipt commit using
   original readback only, including actual OS crashes rather than lost-reply mocks.
4. Require fresh live DB lease and Base epoch checks at every
   queue/permit/lifecycle/control effect after verified receipt commit.
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
