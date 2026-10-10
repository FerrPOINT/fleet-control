# Current State

## Current Integration Snapshot: 10 October 2026

Status: implementation and qualification in progress; **not merge-ready or live
accepted**. This snapshot is not a job monitor. Linked CI runs are authoritative.

## Source And Scope

Integration input `9b823d22fa0e0035ad2677b7cd98ec64fb60b38f` normally
merges the Chats foundation and runtime assembly, original-command recovery,
canonical clarification answers and accepted-PM-run following while new dispatch
is disabled. Migration024 repairs the ACK constraint additively; original022 is
unchanged. Canonical/split lineages are25/28. These fixes are source-reviewed;
current-source Rust/PostgreSQL and native qualification remain required.
Frontend candidate `4e0e185fb3061a08cd9e1e004f66af390bb9c020` adds the final
live-region update barrier without changing its assertion or production code.
The integration also includes exact-run PM instruction receipts in the existing
tool journal. New question/revision claims require a validated matching receipt;
continuation cannot reuse the old run's proof. This is not proof that the model
read the instructions or completed the workflow. Source review passes; current
compilation and PostgreSQL/HTTP regressions remain pending.

The alternative unused chat controller/storage helper/CSS were removed.
Legacy sessions and leaders are retained outside current Chats development.
The remaining legacy session controller and new chat controller are separate;
their consolidation must preserve history and compatibility.

Hermes stays unchanged. Fleet checks ordinary owner/project/current assignment
and workflow state before dispatch. No custom pre-model hook, reserved-run
handshake, second scheduler or host-controller service is required.
Tracker and Workflow are read-only dependencies; full non-PM execution authority
remains a producer dependency. Java lifecycle exists, but automatic Java SDLC
requires separate compatible chat/control evidence.

## Current Evidence

| Scope                    | Verified evidence                                                                                                                                                                                                                                                                                                           | Still open                                                                                                                                     |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust OpenAPI, source3445 | [38063507780](https://github.com/FerrPOINT/fleet-control/actions/runs/38063507780) SUCCESS; artifact11673994046; original strict readback verified; schema SHA256 `afa46ac37b726232eda73df46c24d1d42c796f8873eefb68454fbe0f243df501`. Seven DTOs compared against immutable Tracker357 source and eight verifier units pass | Complete backend/DB/native checks remain separate from schema generation                                                                       |
| Backend, source ce4153   | [38059608279](https://github.com/FerrPOINT/fleet-control/actions/runs/38059608279) FAIL at credentials_pg after fmt/check/Clippy and earlier gates; artifact11672249463 locates the first PM ACK record; owned scratch/DB cleanup pass                                                                                      | Qualify024 against installed022 and clean DB, boundary vectors, original MCP case, disabled-dispatch following and all remaining backend gates |
| Frontend, source60d030f  | [38063560512](https://github.com/FerrPOINT/fleet-control/actions/runs/38063560512) FAIL at unit after typecheck; artifact11673359596 has one live-region text-change assertion frame. Prior six frames are absent; no passed-count receipt is retained                                                                      | Qualify the final await barrier and complete unit/lint/build, three browsers and fresh captures                                                |
| PM execution             | Structured tools, checkpoint continuation, controls, stream recovery and exact-run instruction receipts are integrated and source-reviewed                                                                                                                                                                                  | Compile and execute the new receipt regressions; real compatible service calls and owner flow remain required                                  |
| Forge                    | [38058842111](https://github.com/FerrPOINT/CI-CD/actions/runs/38058842111) FAIL at native OCI, exit101. Authenticated A/B/C artifacts verify first five stages, PostgreSQL3/3,24 negatives, immutable Base66b7 input and scoped cleanup                                                                                     | OCI cause is not retained; obtain closed diagnostics before a corrective run. Later five stages and full12 remain unaccepted                   |
| Base                     | [PR183](https://github.com/FerrPOINT/services-base/pull/183) merged externally as66b7faf; three maintenance helper blobs match the previously qualified payload and the Forge successor pins that merged object                                                                                                             | Consumer/native checks remain separate; no silent SDK or installed-packet promotion                                                            |

Backend controls retain all existing checks. The new024 ignored migration test
must be explicitly selected and its own disposable database cleaned.
Source/pure checks do not execute PostgreSQL, HTTP or native runtimes.

Frontend diagnostics preserve the original unit command, pool and assertions.
Typecheck success is not unit/build/browser success. Existing screenshots are
historical, not acceptance of the combined source.

The PM creation API and typed client exist. The form is still a separate preview
awaiting design approval, not a production Chats entrypoint.

## Release Decision

Qualify the reviewed fixes on their final relevant inputs before release.
Do not repeat unchanged failed attempts or replace actual execution with fixtures,
a healthy process, runtime ACK or a completed run. The real owner PM clarification
and exact-revision confirmation flow and the broader seven-agent delivery/
integration scenario are both still required.

See [active gaps](GAP_REGISTER.md), [delivery order](REMAINING_DELIVERY_WORK.md),
[approved plan](CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md),
[runtime scope decision](contracts/CHAT_CLARIFICATION_CONTRACT.md#runtime-scope-decision-2026-10-10)
and [full SDLC scope](SDLC_IMPLEMENTATION.md). Superseded receipts remain in
[state history](CURRENT_STATE_HISTORY_2026-10-10.md),
[gap history](GAP_REGISTER_HISTORY_2026-10-10.md) and the
[previous snapshot](https://github.com/FerrPOINT/fleet-control/blob/ce4153f453e730dad1e315131d65ca030243264c/docs/CURRENT_STATE.md).
