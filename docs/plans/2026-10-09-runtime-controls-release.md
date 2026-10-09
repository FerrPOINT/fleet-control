# Durable Runtime Controls Release Unit13

Status: isolated source freeze candidate; compilation and runtime acceptance pending.

## Source And Prerequisites

- Exact parent: journal12 `72c05080f5fcb51c674dc174cc4d7754b2a5b211`.
  Only committed Git objects are used, never dirty donor/parent files.
- Own sibling build SDK and unchanged `.base-revision`:
  `19a7a381ae6dbea61a643bb96189e483fa64df5c`.
- Introduction: `d976a82881315188a875dafe66ec973d18bd1691`.
  Select its control domain/repository/native adapter, migration and tests.
  Its native preflight derives from `76b0447`; the required atomic terminal
  packet derives from `ecbf09b`, selected at the introduction snapshot.
- Narrow follow-ups: verified human route proof from `db9c03f`, fresh native
  origin/session/credential checks and session-before-delivery lock ordering
  from `78d7b0b`. Duplicate Idempotency-Key rejection occurs before route I/O.
  Migration13 also denies deletion of durable control history.
- Foundation47, credential11 and journal12 are publication prerequisites.
  Config64 is not a code dependency: required foundation config/drain tables
  already exist. No config64/package/workflow module or migration14..22 is copied.
- Coordinator's clean journal integration is `cb720d7` (normal merge through
  `e0e7f1c63ded345e4cde229996775efb0a873082`, credentials80f,
  Foundation8fa and accepted main `34aaec0f6a3aa776e5725a2173547342c1173178`).
  This source freeze stays on72. A later normal merge and exact integrated
  validation are required; no automatic merge, rebase, push or PR is done here.
- Real Hermes capability/ACK compatibility, centralized human auth acceptance
  and deployed cross-service compatibility remain independent gates. Loopback
  fixtures test this consumer, not an installed Hermes server or PM admission.

## Scope And Invariants

The sole new schema file is `m20261005_000013_runtime_controls`, appended after
journal12 in canonical and legacy lineages (14/17 entries). Earlier migrations
are unchanged. Downgrade refuses populated control history. Empty rollback and
reapply preserve older data; task-chat rollback tests step through exactly three
empty successors. SSE checks compare the whole sorted migration ledger and
assert journal12/control13 registration without assuming either is last.

Stop/steer derive the actor from authenticated middleware and require a verified
human session plus the existing owner/write guard. Central private-owner rules,
standalone legacy role semantics and fresh project authorization are preserved.
Task-bound and PM-bound commands are denied; roles, model content, credentials
and an accepted run do not grant task admission. Legacy run-wide approval is
refused even through the internal adapter; targeted approval code is unchanged.

The private journal freezes actor/key, operation/input hash, session/run/agent,
original request hash, API origin, credential fingerprint and native run/session.
It stores neither guidance text nor bearer secrets. Unique actor/key and active
run hold indexes enforce exact replay and at most one unresolved command.
Fresh agent/run/session/original journal checks precede capability/status GETs;
claim rechecks actor, scope, native pins and origin under locks. A submitted
permit commits before the one native POST. Transport, timeout, content type,
encoding, size, false/missing/foreign ACK errors never trigger retry or fallback.
Submitted/uncertain keys survive supervisor reconstruction without another POST.

ACK, command audit/event and stopping state commit atomically. A stop ACK is
only interrupt acknowledgement: it does not prove completion, safe OS stop or
capacity release. An already-terminal native ACK does not fabricate a mirror.
Uncertain controls release their hold only after an independently committed
exact terminal mirror; acceptance remains unknown. Control GETs only read
scoped receipts/history (at most100 rows), never mutate/reconnect/retry.

The required terminal packet atomically commits prompt delivery, optional
assistant body, run state and DB event cursor. Exact replay is read-only;
changed message/native pins/body/error/state, task/PM binding and missing journal
are denied. Empty completion does not invent assistant text. Late DB faults
roll back the entire packet, not just the final run flag. Session lock ordering
allows existing progress/event FK writers to finish without a lock inversion.

This unit deliberately does not add the later native outcome journal,
original-command outcome recovery, witness-based acceptance recovery, active-run
worker recovery across Fleet process restart, checkpoint/resume, controller
lifecycle/attach or PM execution admission. A historical submitted/uncertain
receipt is not permission to resend; absent independent terminal mirror keeps
the hold, including after restart.

## API And Client Boundary

- POST `/api/v1/sessions/{session_id}/runs/{run_id}/steer` and `.../stop`
  require exactly one bounded `Idempotency-Key`. Body/key/actor/scope replay is
  exact; changed identity/payload conflicts.
- Responses add nullable `command` with the scoped receipt.
  `accepted=true` means a validated ACK, not SDLC/runtime completion.
- GET `.../controls` and `.../controls/{command_id}` enforce the existing
  session read/project guards and exact session/run identity.
- Rust OpenAPI registrations are changed, but generated JSON/TypeScript are
  intentionally not hand-edited. Generation and byte parity need the exact
  Linux source gate. Current generated artifacts remain the previous snapshot.
- UI is not connected in this freeze: existing clients still need a reviewed
  stable-key/uncertain-state interaction using their existing Chats/session
  primitives after API validation. No screenshots or browser acceptance are
  fabricated. This candidate is not publish-ready while that boundary is open.

`runtime_ready=false`. Runtime control receipts are not model authority,
SDLC stage completion, safe OS-stop evidence or full business-goal closure.

## Required Gates

Preparation permits only targeted Rust1.88 formatting, diff/static checks and
workflow YAML/Bash syntax. None of the commands below has run for this freeze.
No Docker, Cargo build/test, cache preparation, browser, push or PR action runs
during source preparation.

Completed light checks: targeted Rust1.88 formatting/parser check, diff
whitespace, static one-migration/both-lineage/protected-path and focused-name/
count checks, workflow YAML/Bash syntax, README validator and its three pure
tests. These are not Rust compilation, PostgreSQL execution or CI acceptance.

New focused cases: API header/actor1, native ACK3, control PG/HTTP18,
atomic terminal PG9 and additive migration1 (32 cases). PG cases are explicitly
ignored by default and fail for missing required env when selected. CI names
every new PG case and checks exact success counts; no broad-suite fallback,
zero-test or default ignored result can satisfy these gates.

```sh
cargo test --locked -p api --lib routes::sessions::tests::runtime_controls_require_one_bounded_key_and_derive_the_actor -- --exact
cargo test --locked -p infra --lib runtime::run_control::tests::
cargo test --locked -p infra --test sdlc_foundation runtime_run_control:: -- --ignored --test-threads=1
cargo test --locked -p infra --test sdlc_foundation runtime_terminal:: -- --ignored --test-threads=1
cargo test --locked -p migration --test runtime_controls -- --ignored --test-threads=1
cargo test --locked -p migration --lib lineage_tests:: -- --include-ignored --test-threads=1
cargo test --locked -p migration --test message_order -- --ignored --test-threads=1
cargo test --locked -p infra --test runtime_approval_events -- --ignored --test-threads=1
```

Use separate owned disposable databases for `FLEET_TEST_DATABASE_URL`,
`FLEET_RUNTIME_CONTROL_MIGRATION_TEST_DATABASE_URL`,
`FLEET_MIGRATION_TEST_DATABASE_URL`, `FLEET_MESSAGE_ORDER_TEST_DATABASE_URL`
and the existing exact synthetic SSE DB/user fixture. Run all parent journal12,
credential11, targeted approval, privacy, real Auth and foundation regressions;
fmt, locked all-targets check, strict Clippy, workspace tests, migration fresh/
legacy/mixed/unknown/history and Rust OpenAPI generation/parity remain mandatory.

Future local QA requires an explicit heavy grant, Git-SHA exports, verified
maintenance Base ComposeHelper journal v2, source/SDK parity, capacity/ownership
preflight and finally exact independently empty cleanup. Preserve permanent
images, volumes, secrets, shared caches and prior immutable evidence.
