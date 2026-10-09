# Exact-Target Approval Recovery Release Unit14

Status: source candidate only; Linux/Rust/PostgreSQL and native acceptance pending.

## Source And Prerequisites

- Exact parent: controls13 `fc6ef12df757a0858f0931cfd88a66fc7f133ed4`.
  It normally merges controls `3a07a331` and integrated journal
  `cb720d7258294ca5d71c7f586a86f7407e9201b1`, retaining accepted
  main `34aaec0`, credentials and Foundation history.
- Historical donor: `d7487ff72409a16836d4fd595c7672971b33a997`.
  Select pending-approval snapshot parsing/repository recovery, targeted
  preflight, migration14 and focused fixtures. Do not import its full tree.
- Own sibling SDK and unchanged build pin:
  `19a7a381ae6dbea61a643bb96189e483fa64df5c`.
- Publication prerequisites: Foundation47, credential11, journal12 and controls13.
  Config64 is not needed: the inherited accepted foundation already owns the
  required config/drain schema. No package/workflow readback or config pin is added.
- Parent owns the separate controls UI consumer and Rust OpenAPI generation.
  This source unit changes neither UI nor generated contracts, locks or SDK.
  Main/parent drift after this exact parent needs normal-history reconciliation
  and an exact combined gate; earlier packets do not accept this candidate.

## Scope And Safety

Only `m20261005_000014_hermes_journal_time_order` is added, after13 in both
lineages (canonical15/legacy18). Earlier migration bytes remain unchanged.
The additional UPDATE trigger runs after journal12's original immutable guard.
It clamps only a new submitted/accepted timestamp to its previous logical
progress time when the wall clock regresses. It never changes request bytes,
identity, the original recovery deadline, accepted ACK requirements or state
transitions. Populated journal history blocks downgrade. Empty down/up preserves
older data and the exact original guard definition.

A bounded20-row keyset queue reads only originally accepted free-chat runs,
including pinned running/waiting/stopping runs after Fleet restart. No unknown
run-ID lookup extension, run submission, native process launch or SSE reconnect
is added. Current primary/agent/archive/run/prompt/outbox, native origin,
credential fingerprint and exact original journal pins are checked before GET.
Legacy runs without an original accepted journal remain held.

A current authenticated GET status of waiting_for_approval must contain the
exact native run/session/request, approval.request event, bounded prompt/detail
and once/deny choices. The verified targeted capability is required. Recovery
atomically inserts the redacted exact request and changes running to waiting;
replay preserves the request ID/content, timestamps, event cursor and transcript.
Changed content conflicts; stopping and settled decisions are not reopened.
Terminal GET recovery uses the inherited atomic terminal mirror, never a
synthetic success or a second native POST.

A human decision still uses the inherited durable reservation before HTTP.
Targeted dispatch now additionally proves the current original free-chat
context, authenticated capability, exact pending action and native session.
It repeats fresh context checks after GET preflight. The native body remains
`{choice, request_id, resolve_all:false}`; exact bounded JSON ACK is mandatory.
Transport/ACK uncertainty retains the existing uncertain decision. Reconstructed
supervisors, GETs and repeated HTTP commands cannot send it again. No inferred
conversion of an uncertain decision to delivered is implemented.

The existing owner/human/task permission guards and assignment revalidation
stay in place. New free-chat recovery/dispatch does not authorize task/PM
execution, model decisions, broad session/always grants or run-wide approvals.
Approval reads remain scoped database-only reads; no API wire schema changes.

The known controls13 steer-history gap is NOT fixed here: guidance is still
hash-only in its journal and needs a separate reviewed mirror follow-up.
No migration15..22, outcome/witness journal, lifecycle/attach, checkpoint/resume,
installed runtime mutation or PM admission is included. `runtime_ready=false`.

## Focused Gates

New cases: snapshot unit3, exact-pending unit1, PG/HTTP recovery15,
logical clock PG1 and additive migration1:21 new cases. The targeted unit group
also retains its prior ACK case (group2). The updated historical positive HTTP
case now seeds a real original journal rather than a legacy acceptance shortcut.
New PG cases are explicitly ignored by default and require named disposable DB
envs when selected; no silent return or zero-test result can satisfy CI.

The mandatory CI steps check every new exact test name and success count.
The journal12 PG group is now16, including the named clock-regression test.
The controls13 counts stay unchanged (API1/native3/controls18/terminal9/migration1).
Whole sorted migration ledger checks include11/12/13/14 without assuming last;
lineage10 and exact four-successor task-chat rollback remain mandatory.

```sh
cargo test --locked -p infra --lib runtime::approval_snapshot::tests::
cargo test --locked -p infra --lib runtime::targeted_approval::tests::
cargo test --locked -p infra --test sdlc_foundation runtime_approval_recovery:: -- --ignored --test-threads=1
cargo test --locked -p infra --test sdlc_foundation hermes_dispatch_journal:: -- --ignored --test-threads=1
cargo test --locked -p infra --test sdlc_foundation targeted_approval_http_requires_human_and_unknown_ack_is_not_repeated -- --exact --test-threads=1
cargo test --locked -p migration --test hermes_journal_time_order -- --ignored --test-threads=1
cargo test --locked -p migration --lib lineage_tests:: -- --include-ignored --test-threads=1
cargo test --locked -p migration --test message_order -- --ignored --test-threads=1
cargo test --locked -p infra --test runtime_approval_events -- --ignored --test-threads=1
```

Use owned disposable PostgreSQL databases for `FLEET_TEST_DATABASE_URL`,
`FLEET_HERMES_TIME_MIGRATION_TEST_DATABASE_URL`,
`FLEET_MIGRATION_TEST_DATABASE_URL`, `FLEET_MESSAGE_ORDER_TEST_DATABASE_URL`
and the existing exact isolated SSE fixture. Retain all inherited foundation,
credential, journal, controls, privacy and real Auth gates, strict Clippy,
locked all-targets/workspace tests and Rust-generated contract byte parity.

This preparation permits targeted Rust1.88 formatting/parser, YAML/Bash syntax,
diff/static checks and README pure checks only. Rust compilation, PostgreSQL,
real Hermes/native-process behavior and CI have not run for this candidate.
Two-process real Hermes compatibility remains a separate acceptance gap, not
a claim supplied by loopback reconstruction tests. Future QA needs an explicit
grant, Git-SHA source export, verified maintenance ComposeHelper v2, capacity/
ownership preflight and finally exact independently empty cleanup. No Docker,
cache preparation, heavy tests, push or PR action is authorized here.

Completed light checks on the candidate: targeted Rust1.88 format/parser17 files,
workflow YAML/Bash56 blocks, README validator and3 pure tests, diff whitespace,
one-new-migration/protected-path/both-lineage/whole-ledger checks, all15 ignored
recovery names,16 journal names and GET-only recovery/one targeted POST structure.
These checks are not compilation, executed PostgreSQL/Rust tests or CI acceptance.
