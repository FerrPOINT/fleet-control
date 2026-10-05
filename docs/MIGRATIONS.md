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
