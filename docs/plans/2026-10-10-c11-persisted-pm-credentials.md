# C11 Persisted PM Credential Candidate

Status: extracted source candidate, not executed Linux/PG or installed acceptance.

## Inputs And Scope

Baseline is PR64 `820a1afe7bc830e2491dff7fffad2bb51f47faf5`, which normally
contains PR47 `11f97aa96337d6d324c832a7c9748ceafa00aeea` and main
`b750e7b69cb359fbe7c7fd13647882c7ae8472bb`. Donor is committed integration
`afc5bb4d9c376f4326c94af2465c51bf1f6d5dbc`. Only C11 modules/shared hunks,
tests, operator docs and CI are selected; donor history and runtime tail are not
merged. SDK19a and PR64 package4b9, Workflow/readback/managed-config guards,
generated artifacts, locks and all historical migrations remain unchanged.

Exactly one new migration, `m20261004_000011_pm_credentials`, extends the existing
PM creation journal from migration10; canonical12/split15 retain both histories.
No migrations12..20, new planner, scheduler, container controller or runtime
admission are included. There is no second schema dependency.

## Implementation

- Save immutable original command, request hash, parent fingerprint and frozen
  assignment before Base delegation. Persist child ID/expiry/scopes, never bearer.
- Reuse the exact operation key/command after uncertainty. Retain receipt across
  Tracker failure; reject payload/parent/origin/TTL conflicts and expired receipt.
- Recheck current parent/child introspection and exact Tracker context on replay.
  Redacted intent/ACK audit is atomic with journal; existing owner/project checks
  precede API continuation. No automatic background human-token reuse.
- Enable only with trusted operator config, default disabled. Preparation does
  not claim a lease, bind Workflow, change effective config or dispatch Hermes.
- Retained-journal down targets the named credential migration and requires its
  specific refusal, unchanged version/applied_at ledger and saved journal.

## Required Evidence

Fresh candidate locked Rust1.88 fmt/check/Clippy/workspace, explicit7 credential
unit/1 config/10 PG recovery cases, all10 lineage cases, dedicated11 migration,
message-order, approval SSE and PR64 package/Workflow regressions remain required.
CI selectors reject missing/zero/ignored receipts for the focused C11 groups.
OpenAPI regeneration/parity must preserve PR64; no manual generated changes.

The separate real Auth consumer uses source01388 and binary hash qualification,
not SDK19a as delegation authority. Its synthetic issuance/replay/conflict/revoke
gate must run again on this candidate. Installed delegation policy, current
Auth/Tracker identity/revocation/expiry and runtime credential handoff remain
external acceptance requirements. No prior combined/partial gate is relabelled
as C11 acceptance. Live PM admission and full SDLC remain outside this unit.
