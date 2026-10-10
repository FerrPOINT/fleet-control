# Current State

## Current Integration Snapshot: 10 October 2026

Status: implementation and qualification in progress; **not merge-ready or live
accepted**. This page is a dated snapshot, not a job monitor. Linked CI runs
remain authoritative for later completion.

## Source And Scope

- Published integration `ee169cbb0253494048109be6d096b62ca1f8832a` preserves main
  Chats safeguards and the dialogue/clarification/requirements UI, merges PM
  tools/continuation and shared stream/recovery `8e6c25b` through `83091f0`, then
  adds the test-only recovered-stop drain correction `0b606d3`. Source review
  passes; it is not runtime acceptance.
- Independently frozen product `32b9f063f9b5099ff61bca24ecdfeb9952889034` is the
  source for the earlier backend run. Its result does not qualify PM integration.
- Migration order in PM integration is authority repair021 followed by PM022:
  canonical23/split26. Frozen32b has canonical22/split25. Human controls023 are
  separate work, not part of either tested source.
- Hermes is consumed unchanged through its existing API. No custom pre-model
  hook, reserved-run handshake, second scheduler or host-controller service is
  required. Fleet still checks owner/project/current assignment before dispatch.
- Task Tracker and project-workflow are read-only dependencies. Java lifecycle
  is retained; automated Java SDLC still requires compatible chat/control proof.
  Leaders remain legacy data outside the current Chats/PM vertical slice.

## Current Evidence

| Scope | Verified evidence | Still open |
| --- | --- | --- |
| Rust OpenAPI, PM source830 | [38040550767](https://github.com/FerrPOINT/fleet-control/actions/runs/38040550767) PASS; strict artifact11666035358 readback, schema SHA256 `ad980604beb2cff0890f4d1a07a185c97a444fda166985f2a6da465a222d129c`; saved schema synchronized | New human-control API codegen and full backend/test qualification |
| Backend, source32b | [38037641800](https://github.com/FerrPOINT/fleet-control/actions/runs/38037641800), controls6f23fd84, terminal FAIL; authenticated artifact11664703020 identifies one activation PG failure at `container_activation.rs:1483`; scratch/DB cleanup pass | Test-only correction merged, isolated PG execution and complete current-source full81 success pending |
| Frontend, PM source830 | [38040245406](https://github.com/FerrPOINT/fleet-control/actions/runs/38040245406), controlsa5c0142, terminal FAIL after20 gates; strict artifact11665915258 readback identifies `chats-directory.spec.ts:59` in Chromium after the SSO/CORS repair; private cleanup passes | Exact browser failure repair, all engines, captures and visual acceptance; fail-fast leaves the other engines unexecuted |
| PM integration | Shared stream/final persistence/restart attachment, typed continuation and phase cursor/report replay fixes merged; production Rust codegen compiles | Owner controls, delivered-answer resume discovery, Rust tests/PG/HTTP/live flow; source checks are not runtime acceptance |
| Forge | [38040722608](https://github.com/FerrPOINT/CI-CD/actions/runs/38040722608), controls970f785, published diagnostic successor; current run must be read for its result | Prior authenticated failure38039120713 stopped between project creation and cache seal; exact step, physical per-stage limit readback and full12 receipt remain open |
| Base maintenance | Draft [PR183](https://github.com/FerrPOINT/services-base/pull/183), exact43d0205;92 focused checks | Private CI, native installation and consumer acceptance; no installed packet promotion |

Frontend controlsa5c0142 retain all23 gates, source blobs, three browser engines,
timeouts and assertions. `--max-failures=1` only stops after an actual failure;
it cannot turn partial execution into PASS. Parent repeats75 control tests.

Forge controls970f785 preserve the runtime-only delegation drop-in and actual
container CPU/memory/PID readback, adding closed diagnostic labels only. All12
stages, product/SDK inputs and budgets remain. Parent repeats178 control tests:
171 pass,7 explicit Linux-only skips. These are not native/full12 success.

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
