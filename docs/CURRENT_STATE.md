# Current State

## Current Integration Snapshot: 10 October 2026

Status: implementation and qualification in progress; **not merge-ready or live
accepted**. This page is a dated snapshot, not a job monitor. Linked CI runs
remain authoritative for later completion.

## Source And Scope

- Integration checkpoint `3a907ae5edb6c9325e240337e010c4150305c9ee` preserves main
  Chats safeguards, the dialogue/clarification/requirements UI and PM tools,
  continuation and shared stream/recovery. It adds owner stop/steer, delivered
  answer continuation recovery, reviewed compiler corrections and browser fixture
  corrections. Source review passes; it is not runtime acceptance.
- Workflow Draft assignment alignment and bounded original-key PM unknown-ACK
  recovery are integrated after independent review. They do not modify Hermes
  or require the optional free-chat recovery extension. New Rust/PG/native
  qualification remains required; codegen4449 now covers both domain changes.
- Strict Tracker metadata now accepts both existing Analysis event types without
  advancing execution or weakening owner/digest/cursor checks. Independent source
  review passes; its authored Rust cases and actual event ingestion remain open.
- Independently frozen product `32b9f063f9b5099ff61bca24ecdfeb9952889034` is the
  source for the earlier backend run. Its result does not qualify PM integration.
- Current PM integration includes authority021, PM022 and human controls023:
  canonical24/split27. Sourcec59 passes fmt/check, then fails Clippy before PG
  qualification. Older source32b has canonical22/split25; its receipts do not
  qualify the current migration or runtime assembly.
- Reviewed successors correct the nested PM authorization guard (`8b1f350`)
  and the private runtime-control fixture (`8f53740`) without changing production
  permissions, privacy, assertions or timeouts. The fixture now supplies its own
  authenticated owner and actual free-chat reads. These source corrections do
  not change the failed c59 receipts below; fresh codegen and full gates are required.
- Hermes is consumed unchanged through its existing API. No custom pre-model
  hook, reserved-run handshake, second scheduler or host-controller service is
  required. Fleet still checks owner/project/current assignment before dispatch.
- Task Tracker and project-workflow are read-only dependencies. Java lifecycle
  is retained; automated Java SDLC still requires compatible chat/control proof.
  Leaders remain legacy data outside the current Chats/PM vertical slice.

## Current Evidence

| Scope                | Verified evidence                                                                                                                                                                                                                                                                                           | Still open                                                                                                                                                                      |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust OpenAPI         | Source4449 [38048577514](https://github.com/FerrPOINT/fleet-control/actions/runs/38048577514) PASS; strict artifact11667814381 readback, schema SHA256 `e1b17e723abf43866c4f913c9fa4fba8b201bef5e3532b4a8f6cdc32ccbcce76`, byte-identical to saved schema; ignored TypeScript client regenerated            | Full workspace/infra/test qualification remains; API generation is not runtime acceptance                                                                                       |
| Backend, source c59  | [38050428605](https://github.com/FerrPOINT/fleet-control/actions/runs/38050428605), controlsb6e9332, terminal FAIL at Clippy after fmt/check; strict artifact11669517037 locates task_chats.rs:65:5 without a retained lint identifier; scratch/DB cleanup pass                                             | Correct the actual lint; all83 stages, PostgreSQL and native qualification remain open                                                                                          |
| Frontend, source c59 | [38050036501](https://github.com/FerrPOINT/fleet-control/actions/runs/38050036501), controls41e7f34, terminal FAIL after20 gates; strict artifact11668908981: Chromium41 passed/1 timedOut/13 skipped, declaration runtime-controls.spec.ts:90                                                              | First fixture has null authentication despite waiting for an authenticated session read; exact timed-out action was not retained. Firefox/WebKit and captures remain unaccepted |
| PM integration       | Shared stream/final/recovery, typed continuation, owner controls, Workflow Draft assignment alignment and bounded unknown-ACK replay are integrated after independent review; current API codegen passes                                                                                                    | Production submission CAS/ACK, PG/HTTP/native/live flow remain open                                                                                                             |
| Forge                | Exact controlsab623f1 [38043788156](https://github.com/FerrPOINT/CI-CD/actions/runs/38043788156) FAIL before cache allocation; strict artifact11667031292 readback: host free100599193600 < required108279229428 bytes; data/inode checks pass, all five first-job stages NOT_RUN, cleanup/daemon stop pass | Resolve measured environment admission without claiming an OS disk-full error or lowering inherited budgets just to pass; physical per-stage proof and full12 receipt remain    |
| Base maintenance     | Draft [PR183](https://github.com/FerrPOINT/services-base/pull/183), exact6602c63; normal main reconciliation63e7a77 is MERGEABLE; parent repeats92 focused checks. Run38050301364 has10 no-runner/no-step jobs with billing/spending-limit annotation                                                       | Private CI has not tested this head; native installation and consumer acceptance remain; no installed packet promotion                                                          |

Frontend controls41e7f34 retain all23 gates, source blobs, three browser engines,
timeouts and assertions. `--max-failures=1` only stops after an actual failure;
it cannot turn partial execution into PASS. Parent repeats79 control tests.

Backend controlsb6e9332 preserve all82 previous stages and add mandatory
`pm_recovery_pg`: three explicitly selected production submission/CAS tests with
controlled HTTP/process boundaries and their own disposable PostgreSQL database.
Parent repeats148 control tests:145 pass,3 inherited Windows platform skips.
Compilation passing does not execute these PostgreSQL cases; Clippy stopped the
attempt before them. Exact original request/key/deadline guards remain enabled.

Forge controlsab623f1 preserve the runtime-only delegation drop-in and actual
container CPU/memory/PID readback, reusing existing capacity/error reporting.
All12 stages, product/SDK inputs and budgets remain. Parent repeats182 control
tests:175 pass,7 explicit Linux-only skips. These are not native/full12 success.

The PM creation API client now validates the authentic runtime-acceptance states;
77 one-shot assertions execute its transpiled source with mocked HTTP/error
dependencies. The sourcec59 frontend job passes its default unit-test stage,
but the complete frontend gate fails in browser fixtures.
The creation form remains a separate preview, not a production Chats entrypoint.

## Release Decision

The current assembly is not released by the narrower foundation/configuration
PRs. No fixture, healthy process, runtime ACK, source probe or completed run is
SDLC acceptance. Existing screenshots remain historical; fresh production
captures and the real PM clarification/confirmation scenario are pending.

See the [active gaps](GAP_REGISTER.md),
[approved plan](CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md),
[runtime scope decision](contracts/CHAT_CLARIFICATION_CONTRACT.md#runtime-scope-decision-2026-10-10)
and [full SDLC scope](SDLC_IMPLEMENTATION.md). Earlier receipts and investigations
are retained in [state history](CURRENT_STATE_HISTORY_2026-10-10.md) and
[gap history](GAP_REGISTER_HISTORY_2026-10-10.md), not repeated in the active status.
