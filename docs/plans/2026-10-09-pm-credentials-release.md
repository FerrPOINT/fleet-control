# Persisted PM Credential Preparation

Status: partial Linux/PostgreSQL evidence; fixture correction and current-main
integration pending. Not merge-ready.

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
relaxation. The regression passed in the Linux gate described below.

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

Real delegation also depends on [Base126](https://github.com/FerrPOINT/services-base/pull/126).
Fresh source inspection confirms neither build SDK19a7 nor accepted Auth
main15cae15 exposes `/auth/tokens/delegate`. The fake HTTP endpoint in the
credential tests proves recovery logic only, not installed Auth compatibility.
Base126 head2754a6d is Draft/conflicting; isolated merge8a34598 and
test-contract correction30f355a (format follow-up01388df)
preserves that history and main15cae15. Base packetfe8c01e6259f passed14/14 gates,
13+9 actual Auth/PG cases,10 parity checks and exact cleanup. Publication and
installed acceptance remain pending. Build SDK and deployed Auth are separate pins: do not silently repin
the SDK, promote Auth, or enable credentials from fixture success. Before opt-in,
verify the real deployed/candidate Auth endpoint, policy, strict introspection,
replay/revoke/expiry and migration compatibility. This dependency is not model
admission and does not waive Tracker/Workflow prerequisites.

`infra/tests/pm_credentials_real_auth.rs` adds the separate opt-in real Auth
consumer gate: exact source01388df, owned process and loopback-only origin,
synthetic HTTP bootstrap, actual issuance/replay, changed-payload conflict, four-field
introspection and child revoke. Missing inputs panic when explicitly selected;
the default ignored result is not coverage. A source/binary-qualified launcher
must build the Auth binary, record its digest and supply only its path/digest
and an owned disposable database. The test creates its own user and parent PAT,
restarts Auth with exact-subject delegation policy and kills/waits its child
even after assertion failure. It never accepts installed runtime credentials.
Environment assertions alone are not provenance.
This case does not contact Tracker, claim a lease or test model dispatch. It
passed explicitly in packet4ed36ce828ea. The twenty-stage gate includes both the real
Auth build and this consumer case alongside fixture/DB recovery checks.

The new `real-base-auth` hosted CI job makes the same consumer mandatory. It
uses isolated GitHub-managed job services (not a claimed local Compose group),
exports committed sources and records source/binary hashes without PAT inputs.
It cannot pass until Base126 publishes the pinned Auth commit. Its configuration
is added here; no hosted execution is claimed before this candidate is published.

Run locked Linux Rust1.88 fmt/check/strict Clippy/workspace tests, explicit real
PostgreSQL `sdlc_foundation`, all ignored migration lineage cases, dedicated
credential migration and historical message-order cases, clean up/down/up and
Rust OpenAPI byte parity. Set `FLEET_TEST_DATABASE_URL`,
`FLEET_MIGRATION_TEST_DATABASE_URL`, `FLEET_CREDENTIAL_MIGRATION_TEST_DATABASE_URL`
and `FLEET_MESSAGE_ORDER_TEST_DATABASE_URL` to separate owned disposable DBs.
An absent database variable or skipped case is not acceptance.

Packet4ed36ce828ea passed15 stages, including actual Auth consumer,7 credential
unit cases,10 PG recovery cases,47 foundation cases, workspace and10 lineage
cases. The inherited approval SSE fixture failed on its stale migration count
(11 versus12), before exercising SSE. The corrected fixture compares exact
canonical ledger versions and requires the credentials migration. Its Linux/PG
verification and four later gates remain pending. All10 independent input/cleanup
checks passed; permanent runtime was unchanged. Preserve this failed packet.

Reconcile the current main34aaec0 before the next full gate rather than rebuild
an already divergent candidate twice. Local QA requires the exclusive heavy slot.
Export exact committed Git
sources; use verified Base ComposeHelper journal v2, owned temporary projects,
capacity guards and exact disposable cleanup. Preserve external caches,
runtime volumes, backups, secrets and earlier evidence. No gate or publication
is claimed by source preparation or passing formatting alone.

This unit depends on foundation47 publication/main acceptance. Configuration
foundation is a separate release; neither branch may silently import the other.
Public PM creation response/OpenAPI and runtime images remain unchanged.
Full pre-model admission, heartbeat/fencing, structured tools, checkpoint/resume
and live confirmation-to-Backlog remain required subsequent acceptance.
