# Current State

## Current Integration Snapshot: 10 October 2026

Status: implementation and qualification in progress; **not merge-ready or live
accepted**. This page is a dated snapshot, not a job monitor. Linked CI runs
remain authoritative for later completion.

## Source And Scope

- Published integration `212d07391b83b8a5081c946b87b4215053c4a153` preserves main
  Chats safeguards and the dialogue/clarification/requirements UI, then merges
  PM tools/continuation `899ec22` through ordinary merge `3b234a1`. Source
  preservation review passes; it is not runtime acceptance.
- Independently frozen product `32b9f063f9b5099ff61bca24ecdfeb9952889034` is the
  source for the backend/frontend runs below. Those runs do not qualify PM212d.
- Migration order in PM212d is authority repair021 followed by PM022:
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
| Rust OpenAPI, source32b | [38036399848](https://github.com/FerrPOINT/fleet-control/actions/runs/38036399848) PASS; generated schema equals committed schema | PM integration codegen and full backend qualification |
| Backend, source32b | [38037641800](https://github.com/FerrPOINT/fleet-control/actions/runs/38037641800), controls6f23fd84, terminal FAIL; authenticated artifact11664703020 identifies one activation PG failure at `container_activation.rs:1483`; scratch/DB cleanup pass | Recovered stop transition repair and complete full81 success; absence of the former eight failures is not all-gate acceptance |
| Frontend, source32b | [38039117605](https://github.com/FerrPOINT/fleet-control/actions/runs/38039117605), controls9bfa434, terminal FAIL after20 gates; authenticated artifact11665645393 identifies `chats-directory.spec.ts:115` in Chromium; private cleanup passes | Directory browser failure repair, all engines, captures and visual acceptance; fail-fast leaves the other engines unexecuted |
| PM integration | Structured tools and saved-answer continuation source merged; source/rustfmt checks only | Shared stream/final persistence/restart recovery, phase cursor/report replay fixes, owner controls, delivered-answer resume discovery, Rust/PG/HTTP/live flow |
| Forge | [38039120713](https://github.com/FerrPOINT/CI-CD/actions/runs/38039120713), controls3f366b5, terminal FAIL; authenticated artifact11665047793 confirms admission sealed, daemon versions/projects recorded, no cache/execution seal and successful cleanup | Later bootstrap failure diagnosis, physical per-stage limit readback and full12 receipt; all first-job test stages remain NOT_RUN |
| Base maintenance | Draft [PR183](https://github.com/FerrPOINT/services-base/pull/183), exact43d0205;92 focused checks | Private CI, native installation and consumer acceptance; no installed packet promotion |

Frontend controls9bfa434 retain all23 gates, source blobs, three browser engines,
timeouts and assertions. `--max-failures=1` only stops after an actual failure;
it cannot turn partial execution into PASS. Parent repeats74 control tests.

Forge controls3f366b5 replace the unsupported dynamic Delegate setter with an
owned runtime-only drop-in and add actual-container CPU/memory/PID readback.
All12 stages, product/SDK inputs and budgets remain. Parent repeats174 control
tests:167 pass,7 explicit Linux-only skips. These are not native/full12 success.

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
