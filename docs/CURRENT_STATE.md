# Current State

## Current Integration Snapshot: 10 October 2026

Status: implementation and qualification in progress; **not merge-ready or live
accepted**. This is a source/check snapshot, not a job monitor. Linked CI runs
remain authoritative for subsequent completion.

## Source And Scope

- Published runtime integration `3c900b00f15aeda2016a45d080d850fa028cdcfd`
  contains PM tools/continuation, owner stop/steer, delivered-answer recovery,
  Workflow Draft assignment alignment and bounded original-key unknown-ACK
  replay. Its canonical24/split27 migrations include authority021, PM022 and
  human controls023. Source presence is not execution acceptance.
- The latest Chats foundation `219f94ae04a352fcaec6483ac199b220341b6ced`
  adds private history and a real task-bound projection. Normal-history
  reconciliation with the runtime assembly is in progress; neither branch alone
  qualifies the eventual combined source.
- The latest backend failure identifies three further Clippy findings. A narrow
  correction `c85d67f2fe711238d6664c3cfae44ea353bfd507` passes independent
  source review and bounded Rust1.88/boolean checks, but is not a full Cargo/PG
  acceptance result. Integration and the combined-source gate remain required.
- The authenticated private runtime-control fixture correction is integrated.
  That case now passes Chromium; the following clarification recovery case
  times out. The safe artifact does not retain its failed action.
- Hermes stays unchanged. No custom pre-model hook, reserved-run handshake,
  second scheduler or host-controller service is required. Fleet checks ordinary
  owner/project/current assignment and workflow state before dispatch.
- Tracker and Workflow are read-only dependencies. Leaders remain legacy data
  outside current Chats/PM work. Java lifecycle is retained; automated Java SDLC
  requires separate compatible chat/control evidence.

## Current Evidence

| Scope                  | Verified evidence                                                                                                                                                                                                                                                                                                                                                                               | Still open                                                                                                                        |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Rust OpenAPI, source3c | [38052082418](https://github.com/FerrPOINT/fleet-control/actions/runs/38052082418) SUCCESS; authenticated artifact11669374937; schema SHA256 `e1b17e723abf43866c4f913c9fa4fba8b201bef5e3532b4a8f6cdc32ccbcce76`; ignored TypeScript client regenerated; seven Tracker DTO comparisons and eight verifier units pass                                                                             | Regenerate after foundation domain reconciliation; API generation does not qualify infra, DB or execution                         |
| Backend, source3c      | [38052804881](https://github.com/FerrPOINT/fleet-control/actions/runs/38052804881), controls566a5db, FAIL at Clippy after fmt/check; artifact11670850956 identifies pm_readback.rs:49, runtime/mod.rs:1446 and runtime_acceptance.rs:366; scratch/DB cleanup pass                                                                                                                               | Integrate the reviewed correction and run the full83-stage gate; PG tests were not reached                                        |
| Frontend, source3c     | [38052893981](https://github.com/FerrPOINT/fleet-control/actions/runs/38052893981), controls8b91a83, FAIL after20 gates; artifact11670207485: Chromium42 passed/1 timedOut/12 skipped; runtime-controls.spec.ts:324                                                                                                                                                                             | Diagnose the second clarification fixture without guessing its failed action; Firefox/WebKit and fresh captures remain unaccepted |
| PM execution           | Dedicated dispatch, structured tools, continuation, stream/final/recovery and controls are integrated and source-reviewed                                                                                                                                                                                                                                                                       | Real compatible service calls, production-path PG/CAS/ACK, checkpoint/rebind and the live owner flow                              |
| Forge                  | [38051604438](https://github.com/FerrPOINT/CI-CD/actions/runs/38051604438), controlsdcad652, FAIL before cache allocation: host free104524414976 < required108279229428 bytes; artifact11669588732; cleanup passes. Successor [38052952880](https://github.com/FerrPOINT/CI-CD/actions/runs/38052952880), controls9e3241e, has completed jobA successfully and is running jobB at this snapshot | Terminal full12 and authenticated per-stage receipts; a completed first partition is not full delivery/rollback acceptance        |
| Base                   | Draft [PR183](https://github.com/FerrPOINT/services-base/pull/183), exact6602c63, published with normal main reconciliation and unchanged maintenance helpers; parent repeats92 focused checks. Run38050301364 has10 no-runner/no-step jobs and a billing/spending-limit annotation                                                                                                             | Exact-head private CI and native consumer checks; no installed-packet promotion                                                   |

Backend controls preserve all83 stages, including three mandatory production
submission/CAS recovery cases with controlled HTTP/process boundaries and owned
PostgreSQL. Parent repeats148 control checks:145 pass, three explicit Windows
platform skips. This does not execute the production PG cases.

Frontend controls preserve all23 gates, three engines, assertions and timeouts.
Parent repeats79 controls checks. Default unit gates pass in the actual attempt;
a browser failure still makes the complete frontend gate fail.

Forge controls preserve all12 stages, exact product/SDK inputs, resource budgets
and scoped cleanup. The successor only adds a guarded disposable-host toolcache
reclaim; its effectiveness and later stages require actual receipts.

The creation API client has component evidence, but the creation form is still
a separate unapproved preview, not a production Chats entrypoint.

## Release Decision

Qualify one reviewed combined source before another full Fleet gate. Do not
repeat unchanged failed attempts or count source review, fixtures, a healthy
process, runtime ACK or completed run as SDLC acceptance. Existing screenshots
are historical; fresh inspected captures and the real PM clarification/
confirmation flow are pending.

See [active gaps](GAP_REGISTER.md), [delivery order](REMAINING_DELIVERY_WORK.md),
[approved plan](CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md),
[runtime scope decision](contracts/CHAT_CLARIFICATION_CONTRACT.md#runtime-scope-decision-2026-10-10)
and [full SDLC scope](SDLC_IMPLEMENTATION.md). Superseded receipts remain in
[state history](CURRENT_STATE_HISTORY_2026-10-10.md),
[gap history](GAP_REGISTER_HISTORY_2026-10-10.md) and the
[previous state snapshot](https://github.com/FerrPOINT/fleet-control/blob/3c900b00f15aeda2016a45d080d850fa028cdcfd/docs/CURRENT_STATE.md).
