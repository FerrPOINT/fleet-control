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
