# Hermes Recovery Extension v1

Status: prepared Fleet consumer source, not native compatibility acceptance.
The historical producer reference is Base [PR #140](https://github.com/FerrPOINT/services-base/pull/140),
head `177edb889e3429b18f12affa35f7034623f11523`, targeting native source
`bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3`. This candidate has not verified
that producer, installed plugin, native byte hashes or a live managed runtime.
The capability revision string is a compatibility requirement, not provenance
attestation. See the [release boundary](../plans/2026-10-09-hermes-recovery-release.md).

## Boundary

This is runtime protocol compatibility, not a scheduler, assignment admission,
Workflow receipt or PM tool. It uses the native platform-handler plugin hook,
not route monkeypatching or another listener. Fleet never opens runtime SQLite.
The producer owns metadata in the native run-idempotency database, not SessionDB.
An after-insert witness commits in the same native reservation transaction,
before model execution. It retains the first scope/key/fingerprint/run mapping
after native pruning; a tombstoned key cannot admit a replacement run.

Historical records are not backfilled. Installation never creates a missing
native database or adopts partial/foreign extension objects. Store incarnation,
path/schema/trigger integrity and bounded witness capacity are checked. This
does not defend against privileged host/database modification or prove OS
isolation; those remain independent rollout gates.

## Capability

Authenticated `GET /fleet/v1/recovery/capabilities` returns the closed object:

```json
{
  "object": "fleet.hermes.recovery.capabilities",
  "contract_version": 1,
  "store_id": "canonical-store-uuid",
  "profile": "default",
  "scope_fingerprint": "sha256-of-default-NUL-original-api-key",
  "native_source_revision": "bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3",
  "lookup": {"method": "POST", "path": "/fleet/v1/recovery/lookup"},
  "non_dispatch": true,
  "durable_witness": true
}
```

Fleet persists these verified facts before the original submission, alongside
the exact request bytes/key/hash/origin/credential fingerprint and DB-clock
horizon. No current capability may retroactively attest an old intent.
Default-profile bearer only: multiplex, room grants and session-key headers are
unsupported. Source hashes are verified by the pinned producer, not caller flags.
When explicitly enabled, the plugin requires `X-Fleet-Recovery-Store-Id` on the
original native `POST /v1/runs`. Auth, store integrity and this exact epoch are
checked before native admission. Missing/wrong epoch returns 409, never a run.
Installing the files alone does not enable the plugin. Enable name:
`fleet-hermes-recovery` in `plugins.enabled` for an explicitly accepted config
revision; Fleet's separate opt-in is
`FLEET_CONTROL_FLEET__HERMES_RECOVERY_EXTENSION_ENABLED=true` (default false).
Fleet never falls back to an unguarded POST when opted in and capability is absent.

## Lookup

Authenticated `POST /fleet/v1/recovery/lookup` is read-only with respect to run
admission: it never reserves a key or starts a model. Its closed request is:

```json
{
  "contract_version": 1,
  "store_id": "original-store-uuid",
  "idempotency_key": "original-message-uuid",
  "request_json": "exact-original-request-bytes-as-utf8-string",
  "request_sha256": "original-byte-hash"
}
```

The producer verifies the raw byte hash and native canonical request fingerprint,
under the original authenticated default-profile scope. A witnessed match returns
HTTP 200 with exactly `object=fleet.hermes.recovery.lookup`, `contract_version`,
`store_id`, `idempotency_key`, `request_sha256`, `scope_fingerprint`,
`profile=default`, `found=true` and `run_id`.

Missing witness returns 404, conflict 409, unavailable/integrity failure 503.
None authorize redispatch, new keys or capacity release. A positive witness can
restore the original accepted ID, but status/session pin/terminal evidence still
come from native GET. Pruned/unavailable status retains capacity.
Original request is bounded to 1 MiB, nested lookup wire to 2 MiB + 4096 bytes,
responses to 16 KiB. Lookup body deadline is five seconds; Fleet requests use
ten-second absolute HTTP deadlines with no redirects, proxy-env or automatic retries.
Fleet checks its original DB-clock horizon before IO and at the atomic recovered
mapping update. Exact already-accepted replay is read-only. Known-ID native GET
does not redispatch and is separate from the bounded unknown-key recovery horizon.

## Verification And Rollout

Required: real native SQLite rollback/concurrency/prune/reset tests, authenticated
actual API lost-ACK/process-crash scenarios with unchanged inference count,
consumer journal/recovery tests and source/hash/cleanup evidence. Fixtures alone
do not enable this extension on installed agents. Existing images/configuration
and legacy intents remain unchanged; absent capability preserves fail-closed
unknown acceptance. No new task/PM dispatch authority is introduced.
