# Persisted PM Credential Preparation

Status: isolated source candidate; Linux/PostgreSQL and exact-head CI pending.

## Scope

Parent is foundation47 `8befcb6ba34c58d2d146403cdd683dbf1dafbce3`, with
unchanged SDK `19a7a381ae6dbea61a643bb96189e483fa64df5c`. This release selects
credential preparation from `9d92f1bf72e9559eb448659731f51dc4a0ba8f72` without
importing the configuration release, later runtime migrations or integration
history. Historical migration files and parent private-chat/namespace guards
remain unchanged. The existing stricter issuance-start TTL bound is retained.

The sole new migration is `m20261004_000011_pm_credentials`, appended to both
canonical and legacy lineages. It extends the existing PM creation JSON journal;
it does not create a second assignment, scheduler, run or secret store.

Counter-review found the copied introspection fixture omitted Base19a7's required
`display_name`. The closed DTO and HTTP fixture now match all four actual Base
fields; a regression keeps missing/invalid display names and unknown fields
rejected. This fixes a real opt-in parent-verification failure, not permission
relaxation. Execution of the regression remains part of the pending Linux gate.

## Behavior And Security

- Opt-in server configuration defaults to disabled. Enabling requires fixed
  Base/Tracker origins, a canonical machine subject and a current parent PAT.
- Save immutable issuance intent before calling Base. Bind canonical payload
  hash, original operation key, parent fingerprint and frozen PM assignment.
- Issue/replay the same Base command, then persist only child token ID, expiry
  and exact scopes. Raw parent/child credentials never enter the database,
  transcript, API response or audit payload.
- Freshly introspect parent and child and read Tracker context on every replay.
  Reject changed subject, assignment, owner, context, scopes or expired receipt.
- Base delegation remains the token authority. A successful credential receipt
  does not claim a lease, authorize a model, bind Workflow or complete a stage.
- Downgrade refuses any persisted credential journal; reconcile explicitly
  instead of discarding recovery material. Legacy journal-free operations retain
  their original JSON and can downgrade/reapply this one migration.

## Acceptance

Run locked Linux Rust1.88 fmt/check/strict Clippy/workspace tests, explicit real
PostgreSQL `sdlc_foundation`, all ignored migration lineage cases, dedicated
credential migration and historical message-order cases, clean up/down/up and
Rust OpenAPI byte parity. Set `FLEET_TEST_DATABASE_URL`,
`FLEET_MIGRATION_TEST_DATABASE_URL`, `FLEET_CREDENTIAL_MIGRATION_TEST_DATABASE_URL`
and `FLEET_MESSAGE_ORDER_TEST_DATABASE_URL` to separate owned disposable DBs.
An absent database variable or skipped case is not acceptance.

Local QA waits for Forge's exclusive heavy slot. Export exact committed Git
sources; use verified Base ComposeHelper journal v2, owned temporary projects,
capacity guards and exact disposable cleanup. Preserve external caches,
runtime volumes, backups, secrets and earlier evidence. No gate or publication
is claimed by source preparation or passing formatting alone.

This unit depends on foundation47 publication/main acceptance. Configuration
foundation is a separate release; neither branch may silently import the other.
Public PM creation response/OpenAPI and runtime images remain unchanged.
Full pre-model admission, heartbeat/fencing, structured tools, checkpoint/resume
and live confirmation-to-Backlog remain required subsequent acceptance.
