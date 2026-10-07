# Controller Recovery V1

Status: internal Fleet recovery candidate. Migration000021 adds durable native
delivery and a trusted supervisor entry point on top of000020. A subsequent
default-off startup worker maintains custody; no public endpoint, installed
opt-in or new model dispatch is enabled.
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
the version and cannot revive an expired lease. The opt-in worker cadence is10
seconds, with one in-flight cycle per agent. Initial recovery is bounded to8
seconds; dual heartbeat has a20-second budget, below the unchanged
30-second lease duration. Ticks during an active cycle are skipped, not queued.
Heartbeat lock admission has a separate20-second bound; native work receives
its full20-second bound only after acquiring the lifecycle lock. The worker
envelope is41 seconds for both bounds and metadata overhead. Waiting does not
extend either lease: after admission, expired custody still fails before native
work. The envelope is a cancellation limit, never a lease or effects permit.
No caller-supplied clock is trusted.
Graceful shutdown stops scheduling and drains existing bounded cycles, rather
than aborting a native write at test/process teardown. The shared drain waits
at most 42 seconds and does not extend a lease or issue an effects permit.
An unresolved outcome remains held; forced OS exit still requires original-key
reconciliation. Exact native live/version readback remains mandatory after drain.

The latest candidate replaces the four separate heartbeat/observe traversals
with two additive Base protocol3 `heartbeat_controller_live` calls, before and
after the DB CAS. Each synchronizes only the exact current/next version, then
obtains fresh original namespace evidence and rechecks the exact live native
owner/version/deadline. The closed live receipt must match the immutable original
recovery receipt. Historical heartbeat ACKs, altered snapshots, expired or
unknown live state cannot substitute for this proof. An older Base executable
is incompatible with this candidate and fails closed; upgrade is a new verified
launch, never an alteration of retained source hashes for an active controller.

Initial recovery/readback and heartbeat are separate worker cycles. Once this
logical controller has an acknowledged record, the worker invokes heartbeat
directly instead of repeating the initial restart witness and historical ACK
readback. Heartbeat still validates the original receipt, physical identity and
both live leases before renewal; a stored ACK alone never permits effects.
For an already claimed command of this logical owner, reconciliation reads the
retained original command directly; Base readback supplies its own fresh stable
physical witness. New reservations still require the full restart witness.
An acknowledged predecessor may reuse its validated immutable stored receipt
as historical evidence; new epoch acceptance still requires Base's fresh native
checks, expired prior custody and a distinct physical controller start.
Cancellation diagnostics contain only agent ID, static stage and elapsed time,
not native command bodies, receipts, credentials or transcript.

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

The trusted `recover_container_controller` entry obtains the Base restart witness
before a new reservation, reserves/retains/claims once, then reconciles by original
readback. Previously claimed commands skip pre-reservation work, not Base's
native readback validation. The entry is not exposed
through HTTP. The explicit default-off startup policy calls it for existing
Hermes container launches owned by a previous logical controller. It does not
create or relabel a launch. Fresh effect admission, interrupted activation
settlement and actual ongoing Hermes acceptance remain required before enabling
restored execution.

Migration000021 is additive. Delete/truncate, identity relabel, claim reset and
receipt replacement fail. Downgrade refuses if either ownership or delivery history
is nonempty. Only when both are empty does it restore the exact000020 schema/guard.

## Dual-Lease Heartbeat

The mutable current lease is separate from the immutable initial delivery body
and native receipt. A cycle first sends the exact stored current version to the
native heartbeat action, then performs a live native `observe`. Only then may
PostgreSQL CAS extend the lease. Fleet sends the exact new DB version/deadline to
native storage and observes again before returning a successful cycle. A final
DB read verifies the same acknowledged owner, version, deadline and receipt hash.

A crash after DB extension but before native delivery is reconciled by delivering
that stored version before another extension. A native commit with a lost reply
is reconciled by exact read-only replay. Equal-version historical ACKs are not
live authority: native expiry must fail observation before DB renewal. DB expiry
prevents native calls and cannot be revived. Changed ACKs, foreign owner or
physical/source/journal drift hold the cycle; no fallback or fresh recovery ID
is created. Timeouts preserve the original unknown outcome and effect fences.

The worker starts only with `fleet.controller_recovery_enabled=true` and a trusted
bridge controller configuration. The default and accepted deployment flags remain
false. It does not skip configuration-draining agents: custody maintenance is
separate from permitting new work. Tests of helper fixtures are not actual OS
crash or running-Hermes acceptance.

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

1. Validate the implemented default-off startup policy against actual original
   source and process restarts. A missing original journal or changed physical
   container must stay held; source upgrade is not an implicit recovery action.
2. Prove the implemented dual heartbeat and unknown-version catch-up against
   actual native custody at10-second cadence. Preserve historical command
   identity across every heartbeat.
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
