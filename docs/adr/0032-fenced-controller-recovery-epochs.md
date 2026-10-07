# ADR 0032: Fenced Controller Recovery Epochs

## Status

Accepted storage design; native integration and live takeover acceptance remain
open. This decision does not enable installed recovery or SDLC execution.

## Context

An original Docker agent can survive a Fleet container restart, but its immutable
launch is bound to the old logical controller. Base's restart witness proves
same-container cessation and retained volume/registration; it does not grant the
new process ownership. Overwriting the original controller UUID or adopting a
daemon PID would erase provenance and permit competing effects. Process-local
locks cannot coordinate replicas or recover an unknown database commit.

## Decision

Use a separate PostgreSQL recovery epoch journal under the existing agent-row
lock. Each original command fixes proposed owner, predecessor and witness hashes.
Thirty-second, versioned DB-clock leases fence native handover/heartbeat; replay
does not renew them. Unknown reservations remain held after expiry. A successor
requires an acknowledged predecessor, expired lease and proven distinct physical
controller start. Original launch/configuration/registration never changes.

Store the verified original native handover receipt hash once. Do not treat that
storage ACK as an Engine proof or runtime permit: Base must retain its private
native epoch and every effect must verify both fences. Until that full consumer
exists, any recovery epoch holds the old queue, permit and lifecycle paths.
Contract: [Controller Recovery V1](../contracts/CONTROLLER_RECOVERY_V1.md).

## Consequences

Cross-replica claims and original-key commit readback become durable and testable.
Expired/unknown reservations intentionally require native reconciliation; lease
expiry alone does not create a replay or replacement permit. The additive
migration owns one release packet and retains history on downgrade attempts.
Native handover, worker heartbeat, checkpoint recovery and actual restart
acceptance are required before the feature can be advertised as complete.

## Alternatives

- Persisting/reusing the old logical controller UUID erases process cessation
  and permits surviving/competing processes to claim the same identity.
- A mutable owner column in `runtime_launches` destroys original launch history.
- Automatic retry after lease expiry duplicates an unknown native command.
- An in-memory owner map or healthy listener cannot fence a second Fleet replica.
