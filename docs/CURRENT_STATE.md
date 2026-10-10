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
  qualification remains required; prior codegen is for the earlier sourcef7d.
- Independently frozen product `32b9f063f9b5099ff61bca24ecdfeb9952889034` is the
  source for the earlier backend run. Its result does not qualify PM integration.
- Current PM integration includes authority021, PM022 and human controls023:
  canonical24/split27. The current backend attempt reaches compilation, not PG
  qualification. Older source32b has canonical22/split25; its receipts do not
  qualify the current migration or runtime assembly.
- Hermes is consumed unchanged through its existing API. No custom pre-model
  hook, reserved-run handshake, second scheduler or host-controller service is
  required. Fleet still checks owner/project/current assignment before dispatch.
- Task Tracker and project-workflow are read-only dependencies. Java lifecycle
  is retained; automated Java SDLC still requires compatible chat/control proof.
  Leaders remain legacy data outside the current Chats/PM vertical slice.

## Current Evidence

| Scope                 | Verified evidence                                                                                                                                                                                                                                                                                           | Still open                                                                                                                                                                   |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust OpenAPI          | Current sourcef7d [38044630680](https://github.com/FerrPOINT/fleet-control/actions/runs/38044630680) PASS; strict artifact11667646855 readback, schema SHA256 `e1b17e723abf43866c4f913c9fa4fba8b201bef5e3532b4a8f6cdc32ccbcce76`; saved schema and ignored TypeScript client regenerated                    | Full workspace/infra/test qualification remains; API generation is not runtime acceptance                                                                                    |
| Backend, source d458  | [38045475100](https://github.com/FerrPOINT/fleet-control/actions/runs/38045475100), controls9565, terminal FAIL at check; strict artifact11667463613 identifies two E0599 test connection-clone errors; scratch/DB cleanup pass                                                                             | Both test errors corrected in c737; current-source compilation, isolated PG and all82 gates remain open                                                                      |
| Frontend, source d458 | [38045832446](https://github.com/FerrPOINT/fleet-control/actions/runs/38045832446), controlsaa787, terminal FAIL after20 gates, including typecheck,411 units/38 files,lint,build,format; strict artifact11667649166 identifies history fixture at fleet-control.spec.ts:888                                | Scoped task-bound history fixture correction; all browser engines, fresh captures and visual acceptance remain open                                                          |
| PM integration        | Shared stream/final/recovery, typed continuation, owner controls, Workflow Draft assignment alignment and bounded unknown-ACK replay are integrated after independent review                                                                                                                                | New current-source codegen, production submission CAS/ACK, PG/HTTP/native/live flow remain open                                                                              |
| Forge                 | Exact controlsab623f1 [38043788156](https://github.com/FerrPOINT/CI-CD/actions/runs/38043788156) FAIL before cache allocation; strict artifact11667031292 readback: host free100599193600 < required108279229428 bytes; data/inode checks pass, all five first-job stages NOT_RUN, cleanup/daemon stop pass | Resolve measured environment admission without claiming an OS disk-full error or lowering inherited budgets just to pass; physical per-stage proof and full12 receipt remain |
| Base maintenance      | Draft [PR183](https://github.com/FerrPOINT/services-base/pull/183), exact43d0205;92 focused checks; run38030482035 has10 no-runner/no-step jobs with billing/spending-limit annotations                                                                                                                     | Private CI has not tested this head; native installation and consumer acceptance remain; no installed packet promotion                                                       |

Frontend controlsaa787 retain all23 gates, source blobs, three browser engines,
timeouts and assertions. `--max-failures=1` only stops after an actual failure;
it cannot turn partial execution into PASS. Parent repeats79 control tests.

Forge controlsab623f1 preserve the runtime-only delegation drop-in and actual
container CPU/memory/PID readback, reusing existing capacity/error reporting.
All12 stages, product/SDK inputs and budgets remain. Parent repeats182 control
tests:175 pass,7 explicit Linux-only skips. These are not native/full12 success.

The PM creation API client now validates the authentic runtime-acceptance states;
77 one-shot assertions execute its transpiled source with mocked HTTP/error
dependencies. The sourced458 frontend job passes its default unit-test stage,
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
