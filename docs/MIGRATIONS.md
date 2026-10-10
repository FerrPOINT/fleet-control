# Migrations

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
