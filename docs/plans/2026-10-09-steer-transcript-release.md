# Acknowledged Steer Transcript Follow-Up

Status: isolated source candidate, light checks only; not runtime acceptance.

## Source And Scope

Based exactly on final controls13
`fc6ef12df757a0858f0931cfd88a66fc7f133ed4`, retaining its normal main,
Foundation47, credential11 and journal12 ancestry. Frozen13 and approval14
are unchanged. No config64 dependency, migration, lockfile, SDK/pin, frontend,
generated OpenAPI or later runtime module is introduced. SDK pin remains
`19a7a381ae6dbea61a643bb96189e483fa64df5c`.

The previous control journal retained only a payload hash; a successful steer
could therefore disappear from Fleet conversation history. This follow-up
stores redacted guidance only after validated native acceptance, not at reservation.

## Schema And Transaction Proof

- Migration3 already permits human-authored `control` session messages.
- Migration4 already provides creator/hash metadata; migration5 permits
  `mirrored` delivery. No new column, constraint or migration is necessary.
- Foundation9 queues only pending user prompts. A mirrored control neither
  enters the outbox nor requests a new run; task10 assigns append order and
  existing triggers publish history/cursor events.
- The command receipt UUID is the message primary key, so there is one mirror
  per command, not one per native run. Actor/creator come from the original
  command, never the replaying model. Session is exact, and the local
  `fleet-control:<session_run_id>:<command_id>:steer` marker carries run linkage.
  It is deliberately not the native run ID used by terminal prompt proof.
- ACK finalization verifies the original operation/input hash before writing.
  Existing agent/session/run/command lock order and native pin checks remain.
  ACK, redacted mirror, preview, cursor events and audit share one transaction.
  A conflicting existing message is rejected, never silently overwritten.
- Acknowledged exact POST replay validates the same payload and creates only a
  missing local mirror. A present mirror is checked without update, duplicate
  event or preview bump. This repairs pre-fix ACK rows only when the caller
  supplies the exact original guidance; hash-only history cannot reconstruct it.
  Scoped GETs remain read-only and cannot perform this repair.
- Reserved/rejected/submitted/uncertain/terminal-observed receipts never create
  delivered guidance. Lost ACK or failed ACK DB commit remains held and cannot
  authorize another native POST. Independent terminal proof is not steer ACK.
  Stop, task/PM denial, owner guards, targeted approvals and capacity semantics
  are unchanged. A mirror is not run completion or SDLC/model admission.

## Required Gates

Seven new PG/HTTP regressions are explicitly opt-in and require an isolated
`FLEET_TEST_DATABASE_URL`. They cover redacted original-operator attribution and
scope, two distinct acknowledged steers on one run, concurrent exact replay and
supervisor reconstruction, old ACK repair, unknown/rejected/terminal-observed
non-delivery, hash proof, audit rollback, actual native ACK plus message-insert
failure with no second POST, and existing-message collision. Default ignored
results are not coverage. The existing stop regression now also denies a mirror.

```bash
cd backend
cargo test --locked -p infra --test sdlc_foundation runtime_run_control::steer_transcript:: -- --ignored --test-threads=1
cargo test --locked -p infra --test sdlc_foundation runtime_run_control:: -- --ignored --test-threads=1
cargo test --locked -p infra --test sdlc_foundation runtime_terminal:: -- --ignored --test-threads=1
cargo test --locked -p migration --test runtime_controls -- --ignored --test-threads=1
cargo test --locked -p migration --lib lineage_tests -- --include-ignored --test-threads=1
cargo test --locked -p infra --test runtime_approval_events -- --ignored --test-threads=1
cargo check --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace -- --test-threads=1
```

Expected focused successes: transcript7, all controls25 (old18 + new7),
atomic terminal9, control migration1. CI requires all 25 exact control names.
Parent API/native ACK/owner/task binding gates, real Auth and runtime compatibility,
both-lineage data preservation/SSE, generated contract parity in the coordinator's
combined source and the reviewed client remain required. No heavy gate was run
for this follow-up. Formatting, diff/static checks and documentation tests are
not Linux compilation or PostgreSQL acceptance. `runtime_ready=false`.

Preparation light checks: Rust1.88 formatter/parser and diff-check PASS;
README validator and its three pure unit tests PASS; YAML parsing and all 55
workflow run-step Bash syntax checks PASS. Static inventory confirms seven
new plus eighteen retained opt-in control cases, all 25 exact CI names/count,
and unchanged authorize/current/reserve/claim/retire/reconcile implementations.
No Docker, Cargo compilation/test, browser run, push or PR was performed.
