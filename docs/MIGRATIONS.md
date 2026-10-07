# Migrations

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

## Accepted Foundation Histories

The registry preserves the canonical ten-step foundation and the historical
thirteen-step split foundation accepted in main `3c6b8ef`. Existing migration
files and applied ledger entries are not rewritten or renamed. On each status,
up or down operation, the stored ledger selects its original foundation;
unknown versions or a mixture of split and combined histories fail closed.

Both registries append the same ordered thirteen runtime/chat migrations
`000010_task_chats` through `000022_controller_stop_delivery`: twenty-three entries for a
canonical installation, twenty-six for a split installation. Full migration
names, not ordinal suffixes alone, identify a step. A matching numeric suffix
does not replace a historical split step with a newer runtime release.

Run the actual PostgreSQL lineage suite, including its opt-in cases:

```sh
FLEET_MIGRATION_TEST_DATABASE_URL=<disposable-postgresql-url> \
  cargo test --locked -p migration --lib lineage_tests -- --include-ignored --test-threads=1
```

The suite covers both populated foundations, partial split history, data and
ledger timestamp preservation, unknown/mixed denial, and empty latest-step
down/reapply. Legacy chats are not inferred as task bindings or backfilled with
runtime launch/dispatch authority. This source integration still requires
ordered release packets with at most one new migration each and exact-head CI.
An empty latest-step downgrade test is not permission to remove retained
launch or dispatch receipts.

## Container Pre-Create Fence

Additive `m20261006_000018_container_preparations` follows000017 without
rewriting it. Empty down/reapply is supported; retained preparation rows prohibit
downgrade, UPDATE, DELETE and TRUNCATE. The table stores identities/intent hash,
not runtime credentials, and does not backfill legacy agents/launches or fabricate
lost preparation provenance. Test its clean upgrade and populated-history
preservation using `FLEET_CONTAINER_PREPARATION_MIGRATION_TEST_DATABASE_URL`.
Release this single migration in an ordered packet after its runtime-launch
dependency, never as part of the accumulated integration tail.

## Endpoint And Controller Recovery Follow-Ups

`000019_runtime_endpoints` adds original-generation endpoint/attachment receipts;
`000020_controller_recovery` adds immutable recovery epochs and DB-clock custody;
`000021_controller_recovery_delivery` adds original native commands, once-only
claims, retained outcomes and audit. These are three separate additive steps,
not replacements for preparation000018 or the accepted foundations.

Each must follow its predecessor in a release packet with at most one new
migration. Recovery/history rows prevent destructive downgrade. A historical
positive receipt after expiry does not renew custody, authorize model/control
effects or permit redispatch. See
[controller recovery contract](contracts/CONTROLLER_RECOVERY_V1.md).
The registry in `backend/migration/src/lib.rs` is authoritative for exact names;
the lineage suite derives counts from its ordered follow-ups and checks both
accepted foundations without changing prior ledger entries.

## Recovered Controller Stop

`000022_controller_stop_delivery` follows000021 as its own additive release.
It retains one immutable original-generation namespace stop intent, one claimed
custody envelope and one validated native exit outcome. Claims require the exact
current acknowledged DB lease; an already claimed unknown command is never
redispatched. A positive historical exit outcome can settle without renewing
the lease. Outcome, original launch exit, runtime status and redacted audit commit
atomically. Session/run/control/approval reconciliation remains separate.
Delete, truncate, identity edits, claim reset and outcome replacement are denied;
empty down/reapply is allowed, populated downgrade is refused. No legacy launch
or session is backfilled. See the [recovery contract](contracts/CONTROLLER_RECOVERY_V1.md).

## Journal Time Ordering

Migration `m20261005_000014_hermes_journal_time_order` is additive and follows
000013. Historical migration 000012 and its guard stay byte-identical. The new
trigger runs after that guard and preserves logical submitted >= created and
accepted >= submitted when the clock regresses. It never renews the deadline,
changes request identity or grants another submission. Down requires an empty
journal and refuses to remove retained receipts. Upgrade/down/reapply tests
compare the original guard and existing user history and verify trigger order.
Release each migration in its own ordered task packet, not an accumulated PR.
See [ADR 0023](adr/0023-logical-journal-progress-time.md).
