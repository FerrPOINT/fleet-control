# ADR 0028: Durable Container Pre-Create Fence

## Status

Implemented in the integration candidate; live Docker/storage-loss acceptance
and ordered release remain required. No installed rollout is implied.

## Context

The original runtime-launch claim precedes Docker start, but Base prepares the
container earlier. The local exclusive creation intent and Base SQLite journal
hold its original operation. Losing the intent before the launch claim must not
allocate another generation, even when Fleet has no open runtime-launch row.

## Decision

Add one append-only `runtime_container_preparations` table in additive000018.
Under the existing agent/runtime row locks, bind history ordinal, controller,
generation, operation and canonical private intent SHA256 before writing the
intent and before Base prepare. Exact replay requires all fields unchanged.
Generation/operation UUIDs are globally unique; agent/ordinal is the primary key.
Outstanding preparation denies a native or mismatching launch. Recheck the
same claim when consuming an existing automatic prepared receipt. Store no raw
credential, process environment or secret-bearing document in PostgreSQL.

## Consequences

- Lost private intent or controller directory holds instead of creating again.
- Competing controllers cannot acquire a different identity for the same ordinal.
- Exact private backup restore still requires original-controller/Base readback.
- Confirmed namespace exit may advance immutable launch-history ordinal normally.
- Controller takeover, complete Base journal reconstruction and actual loaded
  configuration acceptance are not granted by this fence.
- Existing unrecorded preparation cannot be recovered by inference; inventory and
  reconcile it before rollout. Empty downgrade works, retained rows prevent it.
- Release000018 only after its launch-history dependency in a one-migration packet.

## Alternatives

Filesystem-only replay loses its authority when the file disappears. Resetting
the ordinal or adopting a discovered container cannot prove the original command
or acceptance. Storing the complete intent in DB would persist resolved secrets.
None of these alternatives supplies the required pre-create durable identity.
