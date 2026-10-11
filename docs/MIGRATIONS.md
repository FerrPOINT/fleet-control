# Migrations

## Current Integration Candidate

The current source candidate appends026, yielding27 canonical or30 split entries; its
current-source PostgreSQL qualification remains pending. The unit18 paragraph
below retains its original release-unit context, not the current registry count.
Pending Docker migrations15/16/19 use parameterized `to_regprocedure`/
`to_regclass` readback, so cached statements resolve current objects after a QA
drop/recreate roundtrip. Exact source predicates, migration names and ledger
timestamps remain unchanged; these three sources are absent from current main.
This is a candidate source correction, not evidence of successful PostgreSQL
execution or permission to downgrade populated production history. See
[TESTING](TESTING.md) and [current gates](CURRENT_STATE.md).

## PM Stop Under Drain026 Candidate

`m20261011_000026_pm_stop_drain` is additive;010-025 remain unchanged.
Up locks `runtime_control_commands`, renames the025 custody function to
`admit_runtime_control_custody_v25` preserving its OID, and rebinds the existing
trigger to a replacement. Only PM Stop is exempt from the draining predicate;
Steer and all other custody predicates remain guarded. No table, backfill or
receipt rewrite is introduced.

Down locks `runtime_control_commands` and `pm_run_bindings`. Any retained Stop
joined to a PM binding, regardless of control state, refuses downgrade with
`PM stop custody prevents drain prerequisite downgrade`. Otherwise down restores
the025 function with its original OID and rebinds the trigger; re-up reinstalls026.
Do not delete receipts to force downgrade. Both lineages require PostgreSQL
up/down/re-up and refusal/unchanged-ledger qualification; source review is not
execution evidence.

## PM Stop Custody025 Candidate

`m20261011_000025_pm_stop_custody` is additive;010-024 remain unchanged.
Up locks `runtime_control_commands`, preserves the original023 custody function
as `admit_runtime_control_custody_v23`, and rebinds the existing trigger to its
replacement. Only Stop is exempt from the checkpoint/guidance-ACK prerequisite;
Steer and all other original custody predicates remain guarded. No new table,
backfill or rewrite of existing control receipts is introduced.

Down locks the control and original dispatch/binding journals. It refuses retained
PM Stop history lacking both a checkpoint and acknowledged guidance with
`PM stop custody prevents guidance prerequisite downgrade`, before restoring
the original function/trigger. Compatible history permits exact function restore;
re-up reinstalls025. Never delete receipts or edit guidance to force downgrade.
Both lineages require down/up and refusal/unchanged-ledger PostgreSQL rehearsal.
Source-authored tests are not executed migration evidence.

## PM ACK Bounds Repair024

Migration `m20261010_000024_pm_ack_bounds` repairs the invalid PostgreSQL regex
bound in an already-installed022 without changing022's original bytes or ledger.
It locks the journal, asks PostgreSQL to canonicalize two trusted CHECK shapes
in an empty session-local temporary table, and replaces only one exact validated
original ACK constraint, retaining its name. Unknown/ambiguous shapes fail closed;
an already repaired exact shape is a no-op. All journal rows, other constraints,
immutable triggers and submission custody remain intact. The replacement retains
the same ASCII allowlist and length1..512 using a separate length check.

Down refuses any populated journal before changing a constraint. An empty down
restores the original022 CHECK; re-up repairs it again. No destructive reset or
operator ledger rewrite is supported. The temporary comparison table is removed
inside the same atomic SQL block, including rollback on error. Source/rustfmt
checks are not PostgreSQL upgrade acceptance; see the explicit [test](TESTING.md#pm-ack-bounds-additive-upgrade024).

## Historical Unit18 Scope

Unit18 appends only `m20261009_000018_container_activation`, yielding17 canonical
or20 split ledger entries after the frozen15/16/17 releases. Its empty-history
down restores exact17 guard definitions; any activation custody blocks downgrade.
See [unit18 source contract](CONTAINER_ACTIVATION_RELEASE.md).

## Historical Lineages And Task Chats

Accepted main has two supported foundations: canonical ten-step combined SDLC
and historical thirteen-step split SDLC. The registry selects the lineage from
the applied ledger, without rewriting applied names/timestamps. Mixed or unknown
versions fail closed. Both append the same pending
`m20261001_000010_task_chats`, yielding eleven/fourteen logical entries.
Physical source-file count is not the number of applied migrations.

PR47 owns only this new task-chat migration; later credential/runtime releases
remain separate. All fourteen accepted historical migration sources stay
unchanged. Existing free chats are not bound to Tracker tasks automatically.
Transcript backfill adds allocation order in former timestamp/UUID order, not
reconstructed historical insertion order, and does not create mirror events.

The pending task-chat down migration locks transcript and task-chat tables before
checking any populated transcript, binding, creation, projection, PM run or
approval history. It refuses before dropping schema or allocation order. This
includes historical free-chat messages: dropping/recreating append order after
clock rollback would reorder them. Empty-schema down/reapply is supported;
populated recovery requires a forward correction or verified backup/restore,
not destructive downgrade. The up statement and all accepted historical source
bytes are unchanged by this down-only guard.

The lineage suite runs explicitly with `--include-ignored` in CI and disposable
PostgreSQL QA. It covers both populated foundations, replay, empty task-chat
down/reapply, partial split upgrades and unchanged data/ledgers on mixed/unknown
rejection. A default workspace test run that ignores these cases is not upgrade
acceptance.

## Rules

Migration rules:

- Migrations must be reversible unless they intentionally introduce one-way data
  transformations and document the rollback plan.
- New enum-like fields require check constraints.
- Backfills must be explicit and safe for existing rows.
- Concurrent identity allocation must be database-backed.
- Unique indexes for idempotency must allow null keys.
- Down migrations must drop indexes before columns/tables when required by the
  database.

Current critical migrations:

- agent kind/product role/profile/session model
- leader/executor relationships
- session participants/messages/runs
- `SystemRole` backfill from `is_system_admin`
- idempotency keys and payload hashes
- deployment jobs and control settings

Clean DB migration up/status is part of the release gate.
