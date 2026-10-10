# Gap Register

## Current Open Release Gates: 10 October 2026

Status: open. The approved runtime/execution objective is not narrowed to code
already implemented. [CURRENT_STATE](CURRENT_STATE.md) records exact current
source/check evidence; [delivery order](REMAINING_DELIVERY_WORK.md) identifies
workstreams without duplicating the history of every qualification attempt.

## Required Before Merge-Ready

| Gate                     | Current limitation                                                                                                                                                               | Exit evidence                                                                                                                                                                                                |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Combined source          | Chats foundation and PM runtime assembly are normally merged; reviewed corrections, actual-controller regressions and new projection still require combined-source qualification | Reviewed normal merge preserving both histories, task-bound access, paginated private history, owner controls and original command custody                                                                   |
| Agents/configuration     | Lifecycle and revisions exist; complete current-source PG/native qualification remains open                                                                                      | Path/marker/junction safety, real two-agent isolation, readiness, drain/activation/interrupted recovery/rollback; peer unchanged and no replacement under an active run                                      |
| Tracker assignments      | Dedicated PM path exists; no generic seven-role execution consumer. Analysis reservation is not dispatch authority; other non-PM lifecycles remain producer gaps                 | Compatible producer-backed ownership, assignment, effective config, lease/fence, heartbeat and first step; stale/replaced execution cannot perform side effects                                              |
| Hermes dispatch/recovery | Bounded original-key PM replay is reviewed, not fully production-path qualified                                                                                                  | Original request/key survives crash/unknown acceptance; one native run; durable stream/final mirror once; EOF never implies completion                                                                       |
| PM tools                 | Exact-run instruction receipt gates new question/revision claims; Rust API generator passes, test-target compilation and PG/HTTP qualification remain pending                    | Real scoped Tracker/Workflow/Base calls; current-run receipt before publication, no owner impersonation; current assignment checked; retries retain immutable body/key                                       |
| PM continuation          | Delivered-pending discovery/original-ID recovery exist; unprovable legacy delivered history blocks023 upgrade                                                                    | PG/HTTP and historical-data rehearsal; real old-run terminal/safe-stop proof and exact checkpoint/rebind before confirmed continuation                                                                       |
| PM stream/controls       | Latest ce4153 backend passes fmt/check/Clippy, fails first PM ACK; additive024 and accepted-run following await qualification                                                    | Complete Rust/PG gate, actual delta/final/reconnect, owner stop/steer, safe-stop readback and audited run/action-bound approvals                                                                             |
| Idle PM dialogue         | Workflow requires a genuine waiting checkpoint and answer event, not an arbitrary idle turn                                                                                      | Supported producer contract, or explicit capability restriction; no fabricated question/checkpoint or silent free-chat fallback                                                                              |
| Production Chats         | Latestff012d frontend passes20 gates including unit/lint/build; Chromium directory fixture fails. returnTo fixture correction and creation form remain unaccepted                | Reviewed production creation/recovery entrypoint, agent-to-task navigation, filters, owner-only actions, preserved drafts, explicit partial success and foreign-user/count/stream denial; three engines pass |
| Requirements             | Gateway and exact-revision UI are not live vertical acceptance                                                                                                                   | Real questions/answers/final document, owner exact revision/hash confirmation and actual Tracker Backlog; stale revision and operator proxy rejected                                                         |
| Forge                    | Successor0527882 has authenticated A/B/C receipts but fails native OCI101; later five stages remain unaccepted                                                                   | Terminal exact-source full12; actual candidate SHA/repo pipeline/attempt/artifact/deployment/health/acceptance receipts and rollback; scoped cleanup                                                         |
| Base                     | Maintenance PR183 merged externally with unchanged helper blobs; private CI still stops before steps on billing/spending limits. Merge alone is not consumer acceptance          | Required auth/scopes/maintenance input verified on exact source and native consumers; no business scheduler or clarification logic in Base                                                                   |
| Contracts/migrations     | Actuala5f Rust OpenAPI and seven producer DTO comparisons pass;024 upgrade and full backend/native tests remain required                                                         | Final-source generated API/client, clean DB migrations/history preservation, PG/HTTP and contract checks                                                                                                     |
| Docs/screens/release     | Existing captures and historical receipts do not qualify a new assembly                                                                                                          | Fresh required viewports/manifest and visual review; links/redaction/Compose gates; separate reviewed task-owned PRs and exact remote heads                                                                  |
| Live acceptance          | No genuine complete PM or full runtime/execution acceptance receipt for this assembly                                                                                            | Compatible services complete approved positive/negative flows and seven-agent delivery/integration without duplicate work or false success                                                                   |

## Execution Rules

- Use one reviewed source candidate for the next qualification cycle. Reuse the
  existing supervisor, stream decoder, command journals and CI checks; do not
  introduce another scheduler, transcript engine or generic controls framework.
- Keep Fleet/Forge/Base commits separate and preserve other work. Narrower
  foundation/configuration PRs remain dependencies, not vehicles for unrelated
  broad diffs. Preserve Git history without force push.
- Do not modify read-only Tracker/Workflow or unchanged Hermes to manufacture a
  passing consumer. Record actual missing producer contracts explicitly.
- Local resource guards remain mandatory. Pure checks and platform skips are not
  native acceptance; a successful source generation is not a successful runtime.
- Answer persistence, delivery, run completion, workflow completion and owner
  requirements confirmation are separate states.
- Existing tasks/chats stay unenrolled. Opt-in disablement preserves history and
  prevents new assignments, rather than silently cancelling active work.

The [clarification plan](CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md) is a vertical
slice, not removal of the [full SDLC obligation](SDLC_IMPLEMENTATION.md).
The unchanged-Hermes [contract decision](contracts/CHAT_CLARIFICATION_CONTRACT.md)
supersedes historical custom-handshake proposals. Older limitations and receipts
remain in [gap history](GAP_REGISTER_HISTORY_2026-10-10.md) and the
[previous gap snapshot](https://github.com/FerrPOINT/fleet-control/blob/3c900b00f15aeda2016a45d080d850fa028cdcfd/docs/GAP_REGISTER.md).
