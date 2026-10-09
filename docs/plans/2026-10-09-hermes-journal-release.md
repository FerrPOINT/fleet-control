# Hermes Immutable Request Journal Release

Status: isolated source candidate, not build or runtime acceptance.

## Source And Dependencies

- Exact parent: credential candidate `a85a4359cc089b685c97bc5d0062bd14ce064839`.
  Parent working files and later CI/launcher changes are not imported.
- Own sibling build SDK and unchanged `.base-revision`:
  `19a7a381ae6dbea61a643bb96189e483fa64df5c`.
- Historical journal source: `0a580be8a9b4a45179b5b8e46066de65666992e4`.
  Its required atomic acceptance/session readback layer originated in
  `09685b98061ed6764dceb4e11372bcc46d18aab9`; use the version frozen at
  the journal introduction, not latest integration files.
- Foundation47 and credential11 are publication prerequisites. This local branch
  preserves normal parentage from the immutable credential candidate; no remote
  PR or base change is prepared here.
- Coordinator reports current remote Fleet main
  `34aaec0f6a3aa776e5725a2173547342c1173178` (PR55 logs and PR58 heartbeat),
  which is not an ancestor of the parent's later frozen credential candidate
  `d59`. This candidate deliberately stays on `a85`: no automatic merge,
  UI audit or import of dirty parent files. Publication requires coordinator-owned
  normal-history integration with current main and the final credential CI/
  source-qualified-disposable ownership changes, then exact integrated gates.
- Config64 is not a code prerequisite: required config-head/drain tables already
  exist in accepted foundation9. No package/workflow readback, config64 module,
  SDK repin, UI, lockfile, lifecycle refactor or migration13..22 is imported.
- A real Hermes server must expose the exact durable authenticated protocol
  checked by this unit. Fake sockets establish consumer behavior only, not
  installed Hermes compatibility or end-to-end PM admission. Base delegation/
  Tracker/Workflow acceptance remains independently required by the parent and
  does not become journal submission authority.

## Scope And Invariants

The sole new migration is `m20261004_000012_hermes_dispatch_journal`, appended
after credential11 in both canonical and legacy lineages. Existing migrations
and historical ledgers are preserved. Canonical/legacy lengths become 13/16.
The credential migration test targets credential11 explicitly, not whichever
migration happens to be last. Task-chat downgrade tests step through the two
empty successors before checking their original protected history.

The private journal freezes request bytes/hash, model/provider/options, requested
session, message/run/agent identity, origin, credential fingerprint and sanitized
verified capabilities. The database enforces immutable identity, insert scope,
request hash, monotonic timestamps and prepared/submitted/accepted transitions.
It forbids deletion and refuses nonempty downgrade.

Prepare and claim serialize on current agent/session/run/message/outbox. Reject
task/PM binding, foreign primary, draining config, held capacity, changed request
or original credential/origin and expired recovery horizon. A committed claim is
a single permit, not permission to retry an unknown POST. There is no automatic
renewal or fallback submission.

Atomic ACK persists run ID, message mapping, dispatched outbox and accepted
journal together. The run holds capacity in pending until authenticated GET
readback pins its actual effective session. Recovery checks the original journal
and never POSTs. Missing legacy journal or changed original context remains held.
Fresh persisted pending state denies steer/stop/legacy approval-control even if
the caller supplies a stale running DTO. Terminal run/session evidence must
match exactly; partial/nested/foreign events cannot fabricate completion.

The journal is free-chat only. Task-bound dispatch remains denied; roles,
credentials, accepted ACKs, model output and SDLC completion are separate facts.
`runtime_ready=false`: no new admission, running-image promotion or full business
goal completion is claimed.

## Required Exact-Source Gates

No Cargo build/test, Docker, cache preparation, browser or heavy gate ran during
source preparation. Formatting and static scope checks are not compilation.
Light checks passed: targeted Rust1.88 formatting, diff whitespace, workflow
YAML parsing/Bash syntax, explicit focused test-count/DB requirements and static
one-migration/both-lineage/unchanged-parent-path checks. CI configuration only
prepares execution; it is not a CI PASS.

Use a committed Git-SHA export, Rust1.88, locked dependencies and SDK19a7.
Future local QA needs coordinator approval, capacity preflight, verified
maintenance Base ComposeHelper journal v2, an owned disposable Compose project,
separate synthetic databases and finally exact cleanup. Preserve permanent
images/volumes, accepted credentials, shared caches and prior evidence.

Mandatory focused commands, with each expected test name and count checked:

```sh
cargo test --locked -p infra --lib runtime::hermes_wire::tests::
cargo test --locked -p infra --test sdlc_foundation hermes_dispatch_journal:: -- --ignored --test-threads=1
cargo test --locked -p infra --test sdlc_foundation runtime_acceptance:: -- --ignored --test-threads=1
cargo test --locked -p infra --test sdlc_foundation runtime_acceptance_readback_http:: -- --ignored --test-threads=1
cargo test --locked -p infra --test sdlc_foundation runtime_http_ -- --ignored --test-threads=1
cargo test --locked -p infra --test runtime_approval_events -- --ignored --test-threads=1
cargo test --locked -p migration --lib lineage_tests::
cargo test --locked -p migration --lib lineage_tests:: -- --ignored --test-threads=1
cargo test --locked -p migration --test hermes_dispatch_journal -- --ignored --test-threads=1
cargo test --locked -p migration --test pm_credentials -- --ignored --test-threads=1
cargo test --locked -p migration --test message_order -- --ignored --test-threads=1
```

The three new support modules contain 15 journal, 11 atomic ACK/pin and 5 recovery
cases. Wire checks contain 10 cases. Runtime HTTP terminal regressions contain
7 cases; authenticated SSE and journal migration each contain 1 case.
New database cases are explicitly ignored by default and require their DB
variable when selected. A skip, zero matched tests or absent variable is not PASS.

Set `FLEET_TEST_DATABASE_URL`, `FLEET_MIGRATION_TEST_DATABASE_URL`,
`FLEET_DISPATCH_JOURNAL_MIGRATION_TEST_DATABASE_URL`,
`FLEET_CREDENTIAL_MIGRATION_TEST_DATABASE_URL`,
`FLEET_MESSAGE_ORDER_TEST_DATABASE_URL` and
`FLEET_RUNTIME_APPROVAL_EVENTS_TEST_DATABASE_URL` to separate owned disposable
DBs; the SSE fixture requires its documented exact synthetic DB/user names.
Run the broader foundation/credential regressions, migration fresh/legacy/mixed/
unknown/down-reup checks, fmt/check/strict Clippy, workspace tests and Rust
OpenAPI byte parity too. Check unchanged parent guards, source/SDK parity and
independently empty cleanup. Exact Linux execution remains pending.
