# Tracker Metadata11 Compatibility

This slice extends Fleet's strict `metadata_v1` read-only projection from nine
to eleven event types. It does not reserve, renew, schedule, admit or dispatch
work. Existing PM events, Base authorization, bound project/task/owner checks,
redacted fixed summaries, digest verification and transactional cursor handling
remain unchanged. Unknown or invalid events still reject the page, never skip it.
The legacy full-result projection is not extended.

Producer references verified on 2026-10-10 (both PRs open, not merged):

- Tracker PR114: `357caa7a60a717eb7b0ac72f286b793326992931`.
  [Projection](https://github.com/FerrPOINT/task-tracker/blob/357caa7a60a717eb7b0ac72f286b793326992931/backend/app/src/sdlc_metadata.rs#L267)
  emits `analysis.intent_created` as the closed AnalysisIntent, and
  `analysis.assignment_reserved` as exactly eight reference/fence/hash fields.
  [Reservation receipt](https://github.com/FerrPOINT/task-tracker/blob/357caa7a60a717eb7b0ac72f286b793326992931/backend/infra/src/sdlc_reservation.rs#L250)
  explicitly returns `AwaitingAdmission`, `dispatch_allowed=false`, capacity held.
  This is prepared work, not runtime acceptance.
- Workflow PR90: `66e5d6db9fc2ae9129c9162688bacb1a98c7a4a3`.
  [require_owner_execution_evidence](https://github.com/FerrPOINT/project-workflow/blob/66e5d6db9fc2ae9129c9162688bacb1a98c7a4a3/project_workflow/application/base_admission.py#L17)
  unconditionally rejects missing trusted owner assignment/config/evidence
  readback; `assert_base_step` calls it at line 106. Catalog/legacy endpoints do
  not establish generic Base execution authority.

Generic execution for the six non-PM roles (Analyst, Architect, Developer,
Reviewer, Tester, DevOps) remains unavailable from these producer contracts.
Fleet's existing generic dispatch/readiness holds remain source guards, not an
implemented assignment consumer. This limitation is not a missing Hermes hook;
this change adds neither a Hermes patch nor a new producer handshake.

`backend/domain/tests/fixtures/tracker-metadata-analysis.http.json` copies the
saved synthetic Tracker PG/HTTP fixture from
`.local/pm-clarification/task-tracker/.local/b-sdlc01-reservation-3277c112/analysis-reservation-metadata-v1.json`.
The producer export is in `backend/server/tests/support/analysis_reservation.rs`
lines 278-283 at the Tracker ref above. Its three original event digests are not
rewritten; the existing nine-kind fixtures remain unchanged. Focused Rust tests
cover the real resource shapes, closed fields, canonical identities, prepared
routing, fences/ordinals, digest and cursor failures. Rust/PG execution and hosted
acceptance remain pending; this task runs only formatting and pure/source checks.
