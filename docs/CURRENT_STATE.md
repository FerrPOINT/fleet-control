# Current State

## Current Integration Snapshot: 10 October 2026

Status: implementation and qualification in progress; **not merge-ready or live
accepted**. This page is a dated snapshot, not a job monitor. Linked CI runs
remain authoritative for later completion.

## Source And Scope

- Published integration `f7d586be10a958f4f454c357831018250c779256` preserves main
  Chats safeguards, the dialogue/clarification/requirements UI and PM tools,
  continuation and shared stream/recovery. It adds owner stop/steer, delivered
  answer continuation recovery, reviewed compiler corrections and browser fixture
  corrections. Source review passes; it is not runtime acceptance.
- Independently frozen product `32b9f063f9b5099ff61bca24ecdfeb9952889034` is the
  source for the earlier backend run. Its result does not qualify PM integration.
- Migration order in PM integration is authority repair021 followed by PM022:
  canonical23/split26. Frozen32b has canonical22/split25. Human controls023 are
  integrated in the human-controls candidate (canonical24/split27), not part of either
  tested source. Its source reviews and compiler corrections are not new CI proof.
- Hermes is consumed unchanged through its existing API. No custom pre-model
  hook, reserved-run handshake, second scheduler or host-controller service is
  required. Fleet still checks owner/project/current assignment before dispatch.
- Task Tracker and project-workflow are read-only dependencies. Java lifecycle
  is retained; automated Java SDLC still requires compatible chat/control proof.
  Leaders remain legacy data outside the current Chats/PM vertical slice.

## Current Evidence

| Scope                      | Verified evidence                                                                                                                                                                                                                                                                                                                                        | Still open                                                                                                                                                                           |
| -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Rust OpenAPI               | PM source830 [38040550767](https://github.com/FerrPOINT/fleet-control/actions/runs/38040550767) PASS; schema SHA256 `ad980604beb2cff0890f4d1a07a185c97a444fda166985f2a6da465a222d129c`. Human source31ab [38043726635](https://github.com/FerrPOINT/fleet-control/actions/runs/38043726635) FAIL at generator, no artifact/retained compiler diagnostics | Current f7d source fixes the independently identified API identity-type mismatch; new authentic schema and full backend/test qualification remain                                    |
| Backend, PM sourcefacb     | [38041711484](https://github.com/FerrPOINT/fleet-control/actions/runs/38041711484), controls0d1e5a4, terminal FAIL at workspace check; strict artifact11666431970 readback identifies13 Rust compiler diagnostics; scratch/DB cleanup pass                                                                                                               | Compiler corrections, isolated PG execution and complete current-source full81 success; earlier recovered-stop test correction has not yet passed PG                                 |
| Frontend, PM source34ee5f0 | [38041893267](https://github.com/FerrPOINT/fleet-control/actions/runs/38041893267), controls66a446c, terminal FAIL after20 gates; strict artifact11665858046 readback identifies directory timeout. One-line free-chat back-link locator correction is published in the integration                                                                      | New browser proof, all engines, captures and visual acceptance; the bounded receipt does not prove the exact timed-out action                                                        |
| PM integration             | Shared stream/final persistence/restart attachment, typed continuation, owner controls and delivered-pending recovery integrated; compiler and review corrections have source evidence only                                                                                                                                                              | Last full workspace check fails on its older source; corrected compilation, human-control API codegen, PG/HTTP/live flow remain open. The API codegen crate does not depend on infra |
| Forge                      | Exact controlsab623f1 [38043788156](https://github.com/FerrPOINT/CI-CD/actions/runs/38043788156) FAIL before cache allocation; strict artifact11667031292 readback: host free100599193600 < required108279229428 bytes; data/inode checks pass, all five first-job stages NOT_RUN, cleanup/daemon stop pass                                              | Resolve measured environment admission without claiming an OS disk-full error or lowering inherited budgets just to pass; physical per-stage proof and full12 receipt remain         |
| Base maintenance           | Draft [PR183](https://github.com/FerrPOINT/services-base/pull/183), exact43d0205;92 focused checks; run38030482035 has10 no-runner/no-step jobs with billing/spending-limit annotations                                                                                                                                                                  | Private CI has not tested this head; native installation and consumer acceptance remain; no installed packet promotion                                                               |

Frontend controls66a446c retain all23 gates, source blobs, three browser engines,
timeouts and assertions. `--max-failures=1` only stops after an actual failure;
it cannot turn partial execution into PASS. Parent repeats77 control tests.

Forge controlsab623f1 preserve the runtime-only delegation drop-in and actual
container CPU/memory/PID readback, reusing existing capacity/error reporting.
All12 stages, product/SDK inputs and budgets remain. Parent repeats182 control
tests:175 pass,7 explicit Linux-only skips. These are not native/full12 success.

The PM creation API client now validates the authentic runtime-acceptance states;
77 one-shot assertions execute its transpiled source with mocked HTTP/error
dependencies. The source34 frontend job passes its default unit-test stage,
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
