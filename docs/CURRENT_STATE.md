# Current State

## Current Integration Snapshot: 11 October 2026

Status: implementation and qualification in progress; **not merge-ready or live
accepted**. This snapshot is not a job monitor. Linked CI runs are authoritative.

## Source And Scope

Integration input `dd482b91517d8faf30947c6abec6771502a5c0de` normally
merges the Chats foundation and runtime assembly, original-command recovery,
canonical clarification answers and accepted-PM-run following while new dispatch
is disabled. Migration024 repairs the ACK constraint additively; original022 is
unchanged. Reviewed source8f8e69d adds025 for custodial owner Stop after lost
guidance ACK and shares strict terminal validation;010-024 remain unchanged.
The current source candidate adds026 for explicit PM Stop under config drain
and a Stop-only PM MCP profile-verifier bypass; other authority/custody checks
remain unchanged. Its canonical/split lineages are27/30;025 is unchanged.
Source inspection is not PostgreSQL/runtime verification;
current-source Rust/PostgreSQL and native qualification remain required.
Frontend candidate `dc7d1ee2574c1a05b55d794f4252b6ba37916a82` retains the final
live-region update barrier and corrects the browser fixture to read the existing
`returnTo` parameter. Its filter/cursor/owner/navigation assertions stay intact.
The integration also includes exact-run PM instruction receipts in the existing
tool journal. New question/revision claims require a validated matching receipt;
continuation cannot reuse the old run's proof. This is not proof that the model
read the instructions or completed the workflow. Latest terminal8411/source729
backend qualification fails at credentials_pg10/84, exit101, with a retained
test/frame but no assertion reason. Check, Clippy and earlier Auth/API stages
completed in the ordered prefix, not credentials PG, foundation or the final API
comparison. Historical50cb/source5bc passed
PM recovery13 before failing at runtime_stream_bounds51/84. Fixture949
isolates each invalid-stream case and its background-worker runtime, correcting
a verified lifetime defect, not proving the cause or a qualified fix. Existing custody,
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

Workflow [PR90](https://github.com/FerrPOINT/project-workflow/pull/90) is merged
asef2cf9e with new source994bc8e, including a checkpoint fix; PM OpenAPI is
unchanged. Authenticated generic assign/bind/step exists for Analyst, Architect,
Developer, Reviewer, Tester and DevOps; Workflow remains the technical sink/cursor,
not the missing Tracker-owned admission/heartbeat/fencing and verified-stop
capacity-release authority. Its documented PM acceptance uses different Fleet/Tracker/SDK
candidates, not their main releases or ourb97/SDK19a assembly. This grants no
new acceptance here; exact dependency provenance is in the verification ledger.

## Current Evidence

| Scope                    | Verified evidence                                                                                                                                                                                                                                                                                                                                              | Still open                                                                                                                                                       |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust OpenAPI, sourceaa11 | [38066257094](https://github.com/FerrPOINT/fleet-control/actions/runs/38066257094) SUCCESS; artifact11674599663; original strict readback verified; schema SHA256 `afa46ac37b726232eda73df46c24d1d42c796f8873eefb68454fbe0f243df501`. Schema bytes equal the actuala5f schema compared against seven immutable Tracker357 DTOs; eight verifier units pass      | Complete backend/DB/native checks remain separate from schema generation                                                                                         |
| Backend                  | [38105834026](https://github.com/FerrPOINT/fleet-control/actions/runs/38105834026)/1 FAIL on8411/source729 at credentials_pg10/84, exit101; artifact11689882691 strict readback retains test_failure and pm_credential_creation.rs:1178:75, readable/untruncated, both scoped cleanups true                                                                    | Assertion reason not retained. Foundation/later gates and final API comparison were not reached; current-source/native/live acceptance remain open               |
| Frontend, sourceaca/fc0  | [38072655687](https://github.com/FerrPOINT/fleet-control/actions/runs/38072655687) SUCCESS; original strict readback verifies artifact11677891567. All23 gates,461 units,135 catalogue/186 fixture images; all three browsers47 passed/zero flaky with nine opt-in live skips each. 135 catalogue and9 PM views imported, selected corrected captures reviewed | Complete live PM acceptance; fixture success does not qualify backend or installed runtime                                                                       |
| PM execution             | Structured tools, checkpoint continuation, controls, stream recovery and instruction receipts are integrated; historical source5bc passed PM recovery13 before failing runtime stream bounds51                                                                                                                                                                 | Source729 failed credentials_pg10 before foundation/PM13. Complete PG/HTTP, compatible service calls and owner flow remain required                              |
| Forge                    | [38093642467](https://github.com/FerrPOINT/CI-CD/actions/runs/38093642467)/1 on278/sourcee781 FAIL: authenticated A five stages PASS; B PG5/24 negatives/14 cleaned journals PASS; C OCI7 exit101, CLI1/child DiskSpaceGuard. Stages8-12 NOT_RUN; scoped cleanup/all remaining0                                                                                | Original aggregate rejects C; no full12/PM/native Hermes acceptance. Initial disk measurements do not prove capacity at the later child failure; no floor waiver |
| Base                     | [PR183](https://github.com/FerrPOINT/services-base/pull/183) merged externally as66b7faf; three maintenance helper blobs match the previously qualified payload and the Forge successor pins that merged object                                                                                                                                                | Consumer/native checks remain separate; no silent SDK or installed-packet promotion                                                                              |

Source8f/025 and fixture949 require qualification on their final inputs; passing
PM recovery on older5bc does not qualify these later changes. afab and Forge20e
remain distinct historical receipts in the verification ledger.

Historical3e/source7a8 retained E0369 with a nullable frame, not a localized cause;
the earlier6843/source8f receipt remains historical. Historical
[38102509149](https://github.com/FerrPOINT/fleet-control/actions/runs/38102509149)/1
on controls8090/source300c fails foundation11/84 at
`pm_events::pm_observed_terminal_remains_recoverable_until_mirrored_and_marker_is_monotonic`,
`backend/infra/tests/support/pm_events.rs:296:9`. No assertion reason is retained;
ordered progression past check/Clippy/credentials PG is not complete backend or
final API qualification, nor proof of the earlier compiler cause. The run excludes
the reviewed shared-probe P2 published as4d4bc557. Independent source review passed;
PG/native qualification remains pending. Cached Stopped/Cancelled/Failed needs no mirror
commit; Completed may bypass the native probe only with `terminal_committed=true`.
Uncommitted Completed still requires original strict native proof or stays held
after replacement; the candidate does not fabricate that marker. Its controlled
HTTP/repository activation and origin fixtures do not qualify physical Docker
generation replacement or rollback.
Latest [38105834026](https://github.com/FerrPOINT/fleet-control/actions/runs/38105834026)/1
includes the reviewed continuation and archive corrections through source729,
but fails credentials_pg10/84 at
`pm_credential_creation::pm_mcp_publishes_tracker_receipts_then_resumes_only_after_saved_answer_and_terminal_proof`,
`backend/infra/tests/support/pm_credential_creation.rs:1178:75`. Original8411
readback verifies artifact11689882691; both scoped cleanups passed. The receipt
does not retain the error reason or identify the matrix case. No diagnosis or
successful qualification of the later foundation/PM gates follows from it.
Source inspection found that a shared DB does not guarantee a fixture is in the
first20-item page. The test-only candidate6c5c89c uses keyset membership checks
and a22-fixture page-boundary case; product query/timeouts are unchanged.
PG validation is pending; the retained receipt has no assertion reason proving
this was the actual failure cause.
The archive candidate rechecks drain, current runtime status and non-exited
container custody under the agent-row lock after Stop. Rejected archival leaves
history/configuration unchanged; its authored regression awaits PG/native checks.
The separate never-started Stop candidate preserves the creation witness under
the same agent-row lock and rejects private/DB custody. Archival also rejects
unresolved preparation, allowing only a matching exited generation/receipt.
It adds no migration or controller; new API/repository regressions and fresh
Rust OpenAPI generation (changed internal app trait) remain unqualified.
Published source candidatec976c27 rejects config activation with409 before
drain when a bound PM run is pending, including no-journal/prepared/unknown
acceptance. Accepted-running PM retains normal drain. It adds no migration;
its PG regression is pending and the source8f receipts do not qualify it.

Native [38081865039](https://github.com/FerrPOINT/fleet-control/actions/runs/38081865039)/1
fails on257/source5db at Hermes candidate build, exit1/UNKNOWN/TAIL; five
parities true, artifacts0, native-nine/cuts skipped. No UV category or Rust code
is retained. Controller build/metadata passed, not offline qualification;
cleanup proves only absence of this attempt's two exact aliases, not a full
resource audit. No recipe cause is established. Exact receipts and earlier
431/579/1fa/bb failures remain in the
[verification ledger](CHAT_CLARIFICATION_VERIFICATION.md#current-hosted-qualification-2026-10-10).

Latest known point-in-time capacity observation,2026-10-11 01:49UTC, records
C:53.760GiB free against30GiB, available physical memory18.664GiB and available
commit2.990GiB, below the unchanged6GiB floor; this is not a fresh-now readback.
Daemon free disk is unconfirmed. Historical grouping audit35/0/0 passes with only
permanent projects, not capacity admission.
No QA was launched; native qualification remains held.

Backend controls retain all existing checks. Additive024/025/026 migration tests
must be explicitly selected and their own disposable databases cleaned.
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

Historical exact698 versus mainc39 had523 changed paths and15 new migrations010-024.
Historical8f versus that same mainc39 has524 changed paths and16 new migrations010-025.
The current026 candidate has17 new migrations010-026 against mainc39;
its final source inventory and qualification remain pending.
This assembly is not a narrow one-migration main PR. Existing
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
