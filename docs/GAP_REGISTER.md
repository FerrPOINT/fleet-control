# Gap Register

## Current Open Release Gates: 10 October 2026

Status: open. This checklist does not narrow the approved runtime/execution
objective to already implemented code. Exact source/check boundaries are in
[Current State](CURRENT_STATE.md#current-integration-snapshot-10-october-2026).
Historical attempts are in [gap history](GAP_REGISTER_HISTORY_2026-10-10.md).

## Required Before Merge-Ready

| Gate | Current limitation | Exit evidence |
| --- | --- | --- |
| Isolated agents/configuration | Authority repair and test-only drain correction merged; earlier full32b PG receipt still fails recovered stop | Current-source PG lifecycle, marker/path/junction safety, effective config, drain/activation/rollback and readiness tests pass; no config change under an active run |
| Tracker assignment consumption | Creation/binding and PM dispatch code do not prove the complete admission/lease flow | Fresh owner/project/assignment, effective config, lease/fence, heartbeat and first-step checks; stale lease/replacement cannot cause side effects; no Hermes patch |
| Hermes dispatch/recovery | Component receipts do not qualify the complete runtime union | Exact request/key survives unknown acceptance and restart; one native run; stream EOF is not completion; final transcript persists once |
| PM structured tools | Six tool source paths are integrated, not live-qualified | Real scoped Base/Tracker/Workflow calls; no owner impersonation; current phase after advancement; report retry reuses the original immutable body/key |
| PM answer continuation | Saved-answer continuation source exists; delivered answer can remain undiscoverable when resume is pending | Separate durable delivery/continuation state; same answer/command recovery after refresh/restart; old run terminal proof and exact checkpoint/rebind before confirmed continuation |
| PM stream and human controls | Shared stream/final/recovery source merged and production code compiles; human controls and runtime acceptance remain open | Real messages/deltas/final response, reconnect/restart attachment, owner stop/steer and safe-stop readback; approval is run/action-bound and audited |
| Idle PM dialogue | Existing inspected Workflow resume requires a genuine waiting checkpoint and answer event | Supported existing contract for a new idle turn, or an explicit producer dependency and visible capability restriction; no fabricated question/checkpoint or silent free-chat fallback |
| Production Chats | Three tabs have source/component evidence; directory fixture repairs need browser proof; PM Draft creation API has no production form caller in this assembly | Connected PM creation/recovery entrypoint, agent-to-task navigation, own-user filters, owner-only answers/confirmation, conflict input preservation, delivery/continuation partial success, denied/count/stream isolation; three engines pass |
| Requirements/confirmation | Gateway and exact-revision UI are not full vertical acceptance | Real PM questions, saved owner answers, final full revision/hash, separate owner confirmation and actual Tracker Backlog; stale revision and operator proxy confirmation rejected |
| Forge task delivery | Source candidate and isolated harness do not prove installed task attempts | Candidate branch/exact SHA, real repo pipeline, attempt leases, artifacts and deployment/health/acceptance receipts; negative receipt checks and rollback; full12 success |
| Base shared utilities | Maintenance43d0205 is published separately; private CI/native consumer acceptance remains | Required auth/scopes and maintenance packet verified on exact source; safe installer/cleanup and real consumer checks; no business scheduler/clarification logic in Base |
| Contracts/migrations/API | PM830 authentic Rust OpenAPI passes and saved schema is synchronized; human-control API and DB checks remain | Current Rust/PG/HTTP tests, clean migration up/down/history preservation, generated OpenAPI/client and producer contract comparisons on the final heads |
| Screenshots/docs/release | Old screenshots and historical receipts are not current production proof | Fresh required viewport captures/manifest, visual review, docs/link/redaction/Compose gates, separate reviewed task-owned PRs and exact remote heads |
| Live acceptance | No completed genuine PM or full runtime/execution acceptance receipt for this assembly | Compatible services complete approved live and negative flows, including restart/unknown outcomes, without duplicate chats/answers/runs or false business success |

## Execution Rules

- Reuse existing supervisor, stream decoder and command journals. Keep PM business
  state in Tracker/Workflow; do not create another scheduler, transcript engine
  or generic orchestration framework.
- Keep Fleet/Forge/Base commits separate and preserve other work. Existing
  narrower PRs remain dependencies, not vehicles for an unrelated broad diff.
- Do not modify Tracker, Workflow or Hermes to manufacture a passing consumer
  test. Record an actual missing producer contract explicitly.
- Local resource guards still apply. A hosted or pure-control result is not an
  installed-runtime result; Linux-only skips are not passing native checks.
- Run completion, tool acceptance and workflow stage completion are different.
  Requirements confirmation is a separate owner decision, not a PM tool action.
- Existing tasks/chats are not automatically enrolled in SDLC. Rollout stays
  opt-in; disablement preserves history and stops new assignments.

The seven-agent/full automatic delivery and integration acceptance remains in
[SDLC_IMPLEMENTATION](SDLC_IMPLEMENTATION.md); the narrower PM vertical plan
does not remove that later runtime/execution obligation. Follow the
[approved clarification plan](CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md) and
[contract](contracts/CHAT_CLARIFICATION_CONTRACT.md), with the unchanged-Hermes
scope decision taking precedence over historical custom-handshake proposals.
