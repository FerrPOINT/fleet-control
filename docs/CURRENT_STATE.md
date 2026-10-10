# Current State

## Current Integration Snapshot: 10 October 2026

Status: implementation and qualification in progress; **not merge-ready or live
accepted**. This snapshot is not a job monitor. Linked CI runs are authoritative.

## Source And Scope

Integration input `dd482b91517d8faf30947c6abec6771502a5c0de` normally
merges the Chats foundation and runtime assembly, original-command recovery,
canonical clarification answers and accepted-PM-run following while new dispatch
is disabled. Migration024 repairs the ACK constraint additively; original022 is
unchanged. Canonical/split lineages are25/28. These fixes are source-reviewed;
current-source Rust/PostgreSQL and native qualification remain required.
Frontend candidate `dc7d1ee2574c1a05b55d794f4252b6ba37916a82` retains the final
live-region update barrier and corrects the browser fixture to read the existing
`returnTo` parameter. Its filter/cursor/owner/navigation assertions stay intact.
The integration also includes exact-run PM instruction receipts in the existing
tool journal. New question/revision claims require a validated matching receipt;
continuation cannot reuse the old run's proof. This is not proof that the model
read the instructions or completed the workflow. Current3fc backend qualification
passes6 GiB admission and the gates before `pm_recovery_pg`, then stops there
with SIGABRT and no identified failing test/frame or cause. Existing custody,
denial, ledger and no-redispatch assertions remain; prior fixture corrections
and migration-boundary limitations are recorded in the verification ledger.
Full PG/HTTP and native acceptance remain open. The UI correction hides the unavailable warning after a
verified requirements confirmation while preserving disabled/recovery guards;
its regression and complete frontend fixture gate now pass on exactaca917.

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

| Scope                    | Verified evidence                                                                                                                                                                                                                                                                                                                                              | Still open                                                                                                                                                                                    |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust OpenAPI, sourceaa11 | [38066257094](https://github.com/FerrPOINT/fleet-control/actions/runs/38066257094) SUCCESS; artifact11674599663; original strict readback verified; schema SHA256 `afa46ac37b726232eda73df46c24d1d42c796f8873eefb68454fbe0f243df501`. Schema bytes equal the actuala5f schema compared against seven immutable Tracker357 DTOs; eight verifier units pass      | Complete backend/DB/native checks remain separate from schema generation                                                                                                                      |
| Backend, source3fc       | [38079858809](https://github.com/FerrPOINT/fleet-control/actions/runs/38079858809) FAIL on8ce at pm_recovery_pg13/84, exit101/SIGABRT; original artifact readback verified, both cleanups true.6 GiB admission passes, not all84 gates                                                                                                                         | Published78d3727 adds fixed fatal diagnostics without changing gates; [38081721112](https://github.com/FerrPOINT/fleet-control/actions/runs/38081721112) has no terminal result recorded here |
| Frontend, sourceaca/fc0  | [38072655687](https://github.com/FerrPOINT/fleet-control/actions/runs/38072655687) SUCCESS; original strict readback verifies artifact11677891567. All23 gates,461 units,135 catalogue/186 fixture images; all three browsers47 passed/zero flaky with nine opt-in live skips each. 135 catalogue and9 PM views imported, selected corrected captures reviewed | Complete live PM acceptance; fixture success does not qualify backend or installed runtime                                                                                                    |
| PM execution             | Structured tools, checkpoint continuation, controls, stream recovery and exact-run instruction receipts are integrated; current backend qualification stops at PM recovery                                                                                                                                                                                     | Complete PG/HTTP receipt and recovery regressions; real compatible service calls and owner flow remain required                                                                               |
| Forge                    | [38077188736](https://github.com/FerrPOINT/CI-CD/actions/runs/38077188736) FAIL on abf/sourced744. Fresh A/B first six stages including PG3/24 pass; C OCI Cargo101/CLI1 at591, checkpointchild; later five NOT_RUN. All scoped cleanup complete                                                                                                               | CLI cause remains unknown; qualify full12 and actual task delivery/rollback, not metadata presence                                                                                            |
| Base                     | [PR183](https://github.com/FerrPOINT/services-base/pull/183) merged externally as66b7faf; three maintenance helper blobs match the previously qualified payload and the Forge successor pins that merged object                                                                                                                                                | Consumer/native checks remain separate; no silent SDK or installed-packet promotion                                                                                                           |

Native [38081865039](https://github.com/FerrPOINT/fleet-control/actions/runs/38081865039)/1
fails on257/source5db at Hermes candidate build, exit1/UNKNOWN/TAIL; five
parities true, artifacts0, native-nine/cuts skipped. No UV category or Rust code
is retained. Controller build/metadata passed, not offline qualification;
cleanup proves only absence of this attempt's two exact aliases, not a full
resource audit. No recipe cause is established. Exact receipts and earlier
431/579/1fa/bb failures remain in the
[verification ledger](CHAT_CLARIFICATION_VERIFICATION.md#current-hosted-qualification-2026-10-10).

Backend controls retain all existing checks. The new024 ignored migration test
must be explicitly selected and its own disposable database cleaned.
Source/pure checks do not execute PostgreSQL, HTTP or native runtimes.

Frontend diagnostics preserve the original unit command, pool and assertions.
The historical failure receipt records all23 completed gates without browser
counters; zero projection values were not passing test counts. Its successor
now has qualified image evidence and actual counters. The correctedaca917
packet passes all three engines without flaky cases. Selected mobile/desktop
images were inspected. All135 catalogue captures and nine PM views are now
tracked with [routes and provenance](assets/screens/manifest.md) and
[144 image hashes](assets/screens/qualified-import-38072655687.json).
Original public metadata is byte-exact; regenerated Markdown does not pretend
to be the unavailable original capture manifest. Historical PM images remain
unchanged. Frontend/OpenAPI/Base bytes are equal between sourceaca917 and
source3fc; this scoped parity does not qualify the newer backend. See the
[exact verification scope](CHAT_CLARIFICATION_VERIFICATION.md#current-hosted-qualification-2026-10-10).

Idle task-bound PM prompts already have an explicit capability restriction,
including a disabled composer. They do not fabricate a waiting checkpoint or
fall back to a private-chat run. Server/live qualification remains separate.

The PM creation API and typed client exist. The form is still a separate preview
awaiting design approval, not a production Chats entrypoint.

## Release Decision

Qualify the reviewed fixes on their final relevant inputs before release.
Do not repeat unchanged failed attempts or replace actual execution with fixtures,
a healthy process, runtime ACK or a completed run. The real owner PM clarification
and exact-revision confirmation flow and the broader seven-agent delivery/
integration scenario are both still required.

Release audit of exact698 versus mainc39 finds523 changed paths and15 new
migrations010-024. That assembly is not a narrow one-migration main PR. Existing
PR47 owns010; PR64's configuration delta requires reconciliation with current
foundation/main. The prepared C11 unit owns011 relative to that prerequisite,
not relative to current main. Keep release units and additive repairs separate;
do not alter dependency PRs or migration history to make the broad diff appear
ready. Full integration checks do not replace each release prefix's own gates.

See [active gaps](GAP_REGISTER.md), [delivery order](REMAINING_DELIVERY_WORK.md),
[approved plan](CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md),
[runtime scope decision](contracts/CHAT_CLARIFICATION_CONTRACT.md#runtime-scope-decision-2026-10-10)
and [full SDLC scope](SDLC_IMPLEMENTATION.md). Superseded receipts remain in
[state history](CURRENT_STATE_HISTORY_2026-10-10.md),
[gap history](GAP_REGISTER_HISTORY_2026-10-10.md) and the
[previous snapshot](https://github.com/FerrPOINT/fleet-control/blob/ce4153f453e730dad1e315131d65ca030243264c/docs/CURRENT_STATE.md).
