# Clarification Answer Command Custody

## Source Boundary

Isolated normal child of Fleet
`b249bc895e5160fe13383c49d42a24c9308852b3`. Tracker reference is immutable PR114
`357caa7a60a717eb7b0ac72f286b793326992931`, read-only. Its
`backend/infra/src/sdlc.rs::execute` authorizes before exact actor/task/key/hash
replay; selection ordering is a set in `backend/app/src/sdlc.rs::command_hash`.
The existing answer POST returns the stored answer on exact replay before
applying a new answer. Saved-answer GET does not expose the original command
key. This unit does not invent a Tracker command lookup API.

[Fleet PR60](https://github.com/FerrPOINT/fleet-control/pull/60) was independently
read at head `99d1c7f698d5d6069697427b84773be8c03f3813`, OPEN/nonDraft, base
`feat/hermes-runtime-integration-20261004`. Its task-owned changes are consumer
recovery: tab-scoped metadata retains a hold, private payload stays in memory.
Its `docs/CHATS_PM_CONSUMER_20261008.md` CF-03 explicitly leaves reload replay
unavailable. It adds no server journal. No PR60 branch/history/UI implementation
is imported or modified here; later consumer integration must reconcile its
metadata hold only from an exact authoritative Fleet receipt, never an empty
pending list or saved-answer observation.

## Implemented Unit

- Owner/project-authorized Fleet command storage and exact/pending readback.
- Original body/key/hash persisted before HTTP; no credential persistence.
- One-at-a-time leased delivery with fenced completion and sticky uncertainty.
- Explicit same-command recovery via the existing Tracker answer POST; no new
  business key on unknown outcome, no runtime start or model call.
- One additive migration20 in both lineages, now canonical21/split24 entries;
  older migration blobs and runtime/hosted controls are untouched.
- Minimal b249 consumer hookup: reload shows the server-held answer and offers
  explicit recovery by command UUID. New answers fail closed until journal
  readback is known. No private answer is written to browser persistent storage.

See [API](../API.md), [schema](../DATA_MODEL.md) and [focused gates](../TESTING.md).
Delivery acknowledgement closes an answer only; it is not requirements
confirmation/publication, PM consumption or SDLC completion.

## Evidence and Remaining Gates

Eight actual pure Node tests exercise the source response validator with
synthetic data; they do not prove live Tracker behaviour. Rust/domain/API/PG/
HTTP/migration and React/browser regressions are authored but unexecuted under
the no-heavy constraint. Formatting, links and diff checks are lightweight only.

Mandatory next gate is a coordinated Linux Rust1.88 exact-source compile,
strict Clippy, focused and complete PostgreSQL/lineage suites, genuine Rust
OpenAPI generation and strict compatibility, generated TypeScript, frontend
typecheck/unit/browser checks and inspected UI evidence. Generated artifacts
are deliberately not handwritten in this candidate; the temporary frontend
receipt type must be reconciled with that output before release. Migration20
ownership/order must be reconciled normally with parent integration, not by
rewriting historical migrations. No push/PR or runtime acceptance is claimed.

Existing producer pre-model veto/run identity/custody and Workflow dependencies
remain external. Requirements confirmation recovery is a separate command
contract; this unit changes clarification answers only. No background replay
worker stores a human token or attempts delivery without fresh authorization.
