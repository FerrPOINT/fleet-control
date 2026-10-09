# Hermes Recovery Release Candidate: 9 October 2026

Status: source prepared; exact-tree Rust/Linux/PostgreSQL/HTTP/concurrency and
native compatibility gates are pending. No previous QA packet accepts this tree.

## Scope

Baseline: `cb720d7258294ca5d71c7f586a86f7407e9201b1` (journal source stays frozen).
Historical donor patches only:

- `ecbf09b95e2046af38e2107779392c085f11a926`: atomic terminal and pinned recovery.
- `ba973efc1631c135d8eaaa2114a019f5b54e303b`: original-key witness consumer.
- `c2c6b0f0c815e1670a1cd5fbeda59bb19ddf5ef5`: bounded SSE framing, identity,
  transcript budgets and their fixtures only. No prepared-dispatch/tail import.

Baseline fail-closed repository defaults and explicit ignored-DB requirements
are retained. The recovery-wire denial case additionally checks changed original
origin/body/hash/key/credential/state before any HTTP. No new migration (including
approval14), config64/module/control13/lifecycle15..22, UI, lockfile, SDK/runtime
pin or installed configuration change is included. SDK remains
`19a7a381ae6dbea61a643bb96189e483fa64df5c`. Parent owns later integration;
this checkout does not publish, merge PRs, mark ready or deploy.

## Authority

The sole run submission permit precedes network IO. Unknown acceptance never
issues a second `POST /v1/runs`, even with the saved idempotency key. Recovery uses
only an authenticated non-dispatch original-key witness lookup on the original
listener, checking exact request bytes/hash/key, credential/scope and durable store
incarnation. Facts must have been frozen before the original submission; legacy
journals cannot acquire them retroactively. DB-clock horizon is rechecked under
lock and at the recovered mapping update. Missing/negative/conflicting/expired
proof holds capacity and cannot produce a replacement key or run.

Known-ID recovery verifies status and the effective session, including already
pinned running/waiting/stopping records; it does not attach a replacement SSE
consumer. Terminal proof atomically commits redacted mirrors/delivery/run state
and durable events. Exact replay is read-only; contradictory proof rolls back.
An incomplete EOF frame is never dispatched. Independent terminal status is
required, and framing/transport/budget failure holds the original identity/capacity.
No task/PM model authority, Workflow receipt or Tracker stage completion is granted.

## Mandatory Selectors

Backend CI retains locked workspace check, strict Clippy, workspace tests,
lineages, existing journal15/ACK11/readback5/migration1, OpenAPI and approval SSE.
The existing `runtime_http_` selector becomes8. Each of the five new PG families
gets its own disposable CI database. Shell uses `set -euo pipefail`; every family
requires exact nonzero PASS counts and zero ignores. Without
`FLEET_TEST_DATABASE_URL`, explicitly selected PG cases fail instead of skipping.

| Selector | Target | Count | Default |
| --- | --- | --- | --- |
| `runtime_terminal::` | `sdlc_foundation` | 14 | ignored |
| `runtime_pinned_recovery::` | `sdlc_foundation` | 5 | ignored |
| `runtime_unknown_recovery::` | `sdlc_foundation` | 2 | ignored |
| `runtime_recovery_races::` | `sdlc_foundation` | 2 | ignored |
| `runtime_stream_bounds::` | `sdlc_foundation` | 7 | ignored |
| `runtime_http_` | `sdlc_foundation` | 8 | ignored |
| `runtime::recovery_wire::tests::` | `infra --lib` | 3 | unit |
| `runtime::sse_wire::tests::` | `infra --lib` | 10 | unit |

These are source inventory counts, not executed test results: 30 new-family PG
cases, 8 runtime HTTP cases (one new) and13 explicit recovery/framing unit cases.
The tests use synthetic credentials and owned HTTP listeners/PG fixtures, never
installed runtime credentials. The framing cases include real30s/60s deadlines;
a light source/format check does not execute those cases.

## Native Acceptance Boundary

The historical producer reference is Base PR140 at
`177edb889e3429b18f12affa35f7034623f11523`, targeting native
`bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3`. This consumer's source-string check
is not byte provenance, witness durability or live compatibility evidence.
Actual source-qualified producer/native SQLite rollback/concurrency/prune/reset,
authenticated lost-ACK/process-restart tests with unchanged inference count and
managed supervisor acceptance remain mandatory before enabling the default-off
`FLEET_CONTROL_FLEET__HERMES_RECOVERY_EXTENSION_ENABLED` option. No plugin is
installed or enabled by this patch. Missing capability does not fall back to
unguarded dispatch. No native acceptance is claimed from HTTP fixtures.

See [recovery wire](../contracts/HERMES_RECOVERY_V1.md),
[stream profile](../contracts/HERMES_EVENT_STREAM_V1.md),
[terminal ADR](../adr/0018-atomic-terminal-pinned-recovery.md) and
[recovery ADR](../adr/0019-native-original-key-recovery.md).
