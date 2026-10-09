# ADR 0019: Native Original-Key Recovery

## Status

Implemented candidate; explicit opt-in and installed/live acceptance required.

## Context

An original `POST /v1/runs` can commit while Fleet loses its acknowledgement.
Native durable idempotency alone does not prove continuity after pruning, reset,
credential rotation or relocation. Retrying POST is not a readback operation.
Fleet must not open Hermes SQLite, modify SessionDB or create a second scheduler.

## Decision

Use the supported native platform-handler plugin hook, owned by Base and pinned
to verified native module hashes. An AFTER INSERT witness commits in the original
native reservation transaction. Immutable scope/key/fingerprint/run mapping and
store epoch survive native pruning; historical rows are never backfilled.
The plugin exposes bounded authenticated capability and non-dispatch lookup on
the same listener. Original POST requires the verified epoch header.

Fleet opt-in defaults false. When true, verified closed capability facts are
frozen in the existing original request journal before its one-time permit.
Unknown recovery checks original bytes/hash/key/origin/credential/scope/epoch and
DB-clock horizon, reads the original key only, and atomically commits the recovered
ID/delivery/outbox/journal under the existing lock order. Horizon is checked again
at the mapping update. Native GET still supplies effective-session and terminal
evidence; lookup never supplies completion or capacity release.

See [the wire contract](../contracts/HERMES_RECOVERY_V1.md). Task/PM admission,
prepared-intent restart dispatch, native configuration attestation and OS
isolation remain separate gates. Java chat/control receives no new capability.

## Consequences

Missing/conflicting witness, changed scope/epoch, unavailable native status or
expired unknown-key horizon retains the pending capacity. No new key, POST run
replay, fabricated message or business receipt is produced. Witness capacity is
bounded to 100000 without automatic purge; saturation blocks admission, not
existing readback. Path/inode/schema checks intentionally refuse relocation or
restore without operator reconciliation. They do not defeat a privileged OS/DB
writer. Installed config and images are not silently enabled or repinned.

## Alternatives

- Retry native POST with the saved key: rejected; pruned/reset history can admit
  a replacement and execute the model again.
- Open SQLite from Fleet: rejected; crosses producer ownership and filesystem
  authority, and cannot safely establish the native admission transaction.
- Monkeypatch native handlers or add a proxy listener: rejected; use the
  supported pre-freeze platform handler hook and existing authentication.
- Invent synthetic witnesses for legacy rows: rejected; no original atomic
  witness proof exists for those admissions.
