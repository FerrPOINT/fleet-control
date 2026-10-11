# Heartbeat Alert Verification

## Current Integration Evidence

The sections below retain the original main-based PR58 packet, including its
baseline and SDK pin. Exact PR58 head
`ef6f275aaef84c8df77e2d9733c2b6ea83e326f3` has five successful GitHub checks
(run37680405418); that is not CI evidence for the entire integration branch.
The scoped fix is now imported as `7dd19f1` into the runtime/Chats candidate
with unchanged SDK `cbb4e99230420dc2659431b1c9fb5090e5c940f0`.
Its separately executed [combined gates](CHAT_CLARIFICATION_VERIFICATION.md#combined-heartbeat-closure-8-october-2026)
and [current fixture screenshots](assets/screens/heartbeat-alert-integration-20261008/manifest.json)
prove current-source behavior. Neither packet promotes installed runtime or
opens task admission.

## Scope

Main-based correction of persisted heartbeat incidents, not a rollout of the
runtime/PM integration branch. Baseline is
`c8093aace07e54436893c5f7e35df1f968690266`; unchanged Base pin is
`875cac2edf1a18c3a8a59e2f67256d02a8fc04e4`.

The service previously inserted `agent_heartbeat_stale`, which the historical
database constraint rejects. It now inserts canonical `heartbeat_stale`,
deduplicates open/acknowledged incidents across concurrent creators with an
agent-row lock, and resolves only a positively fresh running-agent heartbeat.
Resolution and redacted audit are atomic. Unknown/future timestamps and
nonrunning statuses retain incidents; unrelated down alerts are not cleared by
heartbeat freshness. Explicit health recovery uses the same canonical kind.

No migration, SDK/lockfile update, public route/model change, installed runtime
promotion, or SDLC admission is part of this patch. Legacy UI kind labels remain
compatible. Healthy monitoring does not imply readiness for SDLC.

## Source Gates

Disposable Compose project `sdlc-qa-fleet-heartbeat-6eb38d375290` uses pinned Rust
1.88.0, PostgreSQL 17.11, an internal unpublished network and disposable tmpfs
database. The source input manifest contains 165 backend/SDK files.

- Rust formatting, all-target check and strict Clippy pass.
- Rust-generated OpenAPI is byte-identical to the committed public document.
- Workspace suite: 99 passed, 10 explicitly ignored, no failures.
- The same compiled profile/lineage binaries then execute all 10 ignored cases
  against that disposable PostgreSQL: 3 profile and 7 migration cases pass.
  These results establish 109 distinct passing cases, not 114: the additional
  focused heartbeat run repeats the same 5 cases and passes again.
- The 5 new PostgreSQL cases prove canonical persistence, acknowledgement
  deduplication, concurrent identity, fresh recovery without status transition,
  new incidents, unknown/nonrunning retention, explicit health recovery, audit
  failure rollback and redacted retry.
- Frontend frozen offline install uses Node 22.20.0 and pnpm 10.28.1 with the
  exact unchanged SDK. Codegen/typecheck/lint/format, 160 unit tests, build,
  OpenAPI compatibility, Markdown links and the existing 135-screen manifest
  pass. Build retains the existing advisory bundle-size warning.
- Chromium, Firefox and WebKit each pass the canonical heartbeat browser case
  at all three required viewports; 9 screenshots are captured.
- README validation and its 3 tests pass. Docker grouping audit reports complete
  endpoint coverage and no violations.
- Owned containers/network are removed by the exact Compose project cleanup;
  all 165 source hashes remain unchanged after execution.

Private packet checksums (SHA-256):

- Frozen source manifest:
  `224483accc1d58ba7283883ffc336e437a7d841c7dc6664b291cf4e1da90c734`.
- Completed Rust/PostgreSQL log:
  `7733e3161f3e8d07c000357604fa92c74ec4720ac63f04abd3c5923ecd87ccb9`.
- Explicit opt-in PostgreSQL log:
  `e78743929e498d4a6858b5e740f4f72782d50f153482c2fcd41a5b40e055f792`.
- Frontend/browser log:
  `823f4286c57c565f2a0a96215d9ccc503be2b0cefa9230d2b3639e3a8f53e91f`.

The original packet `sdlc-qa-fleet-heartbeat-848e66f2f71c` failed all-target
compilation because the new fixture attempted to clone a mock-enabled SeaORM
connection. The fixture now opens separate connections. That historical failure
is not acceptance evidence; the replacement source packet above is authoritative.

## UI Evidence

The generated [manifest](assets/screens/heartbeat-alert-20261007/manifest.json)
binds `/alerts`, browser, CSS viewport, actual PNG dimensions/device scale,
normalized UTF-8 source hashes and unchanged PNG hashes. Chromium captures at
375x812, 1920x1080 and 2560x1440 were visually inspected for readable labels,
overflow and overlap. Source/PNG hashes are independently verified.

These are production-component HTTP fixtures. They do not prove installed
Hermes health collection, native process recovery, PM clarification or an
autonomous SDLC transition.

## Release Gate

The original local source gates, Rust OpenAPI export/comparison, owned cleanup
and all five exact-head GitHub checks now pass for PR58. A
The local source gates, Rust OpenAPI export/comparison and owned cleanup pass;
exact-head GitHub checks must still finish before this task is marked merge-ready. A
successful component test is not proof of the entire product goal. Permanent
runtime images, secrets, volumes and settings have not been promoted by this
task.
