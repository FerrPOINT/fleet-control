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
read the instructions or completed the workflow. Latest backend38114888529/1
on02b/sourcec3fc is terminal FAILURE at pm_ack_migration, gate exit101, named
ACK repair test and frame pm_ack_bounds.rs:169:52; both scoped cleanups passed.
Source inspection proves the fixture upgraded through025/026 before its
down-one assertion, testing026 rather than repair024. The parent's bounded024
fixture correction is pending PG qualification, not a migration-guard failure
or a new test PASS. Prior9e/source2dc38110519035/1
failed the fence-negative assertion at1218:13. Published c3fc changes that custody
refusal to Conflict and asserts no new run/control/workflow mutations; PG
qualification remains open, distinct from the separate Linux201 pure-controls PASS.
Historical50cb/source5bc passed
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

External Fleet PR65 and Tracker PR127 merged into feature branches, not main,
on11 October. Their mandatory native/model admission contract differs from
this assembly's unchanged-Hermes decision. They are not imported or runtime
qualification here; see [external contract status](contracts/CHAT_CLARIFICATION_CONTRACT.md#external-pm-contract-divergence-11-october-2026).

Workflow [PR90](https://github.com/FerrPOINT/project-workflow/pull/90) is merged
asef2cf9e with new source994bc8e, including a checkpoint fix; PM OpenAPI is
unchanged. Authenticated generic assign/bind/step exists for Analyst, Architect,
Developer, Reviewer, Tester and DevOps; Workflow remains the technical sink/cursor,
not the missing Tracker-owned admission/heartbeat/fencing and verified-stop
capacity-release authority. Its documented PM acceptance uses different Fleet/Tracker/SDK
candidates, not their main releases or ourb97/SDK19a assembly. This grants no
new acceptance here; exact dependency provenance is in the verification ledger.

## Current Evidence

| Scope                    | Verified evidence                                                                                                                                                                                                                                                                                                                                              | Still open                                                                                                                                                 |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust OpenAPI, sourceaabe | [38107719356](https://github.com/FerrPOINT/fleet-control/actions/runs/38107719356)/1 SUCCESS on controls12de; artifact11689594281; original strict readback independently verified. Schema SHA256 `afa46ac37b726232eda73df46c24d1d42c796f8873eefb68454fbe0f243df501`, unchanged; genuine307-input export includes the changed app trait                        | Complete backend/DB/native checks remain separate from schema generation                                                                                   |
| Backend                  | [38114888529](https://github.com/FerrPOINT/fleet-control/actions/runs/38114888529)/1 FAIL on02b/sourcec3fc at pm_ack_migration; original strict artifact11693008835 readback retains gate exit101, named ACK repair test, pm_ack_bounds.rs:169:52, readable/untruncated log and both scoped cleanups true                                                      | Source-proven fixture target drift, not a migration-guard failure; parent's bounded024 correction and final backend/PG/native qualification remain pending |
| Backend pure controls    | [38114924292](https://github.com/FerrPOINT/fleet-control/actions/runs/38114924292)/1 SUCCESS onbf4; authenticated exact02b, original Linux suite201 PASS,0 skips/failures/errors,8.558s, exit0                                                                                                                                                                 | Pure controls only, not backend/PG/native PM acceptance. Prior3b379/38112684097 qualifies onlya78; eb2/38111234565 only9e                                  |
| Frontend, sourceaca/fc0  | [38072655687](https://github.com/FerrPOINT/fleet-control/actions/runs/38072655687) SUCCESS; original strict readback verifies artifact11677891567. All23 gates,461 units,135 catalogue/186 fixture images; all three browsers47 passed/zero flaky with nine opt-in live skips each. 135 catalogue and9 PM views imported, selected corrected captures reviewed | Complete live PM acceptance; fixture success does not qualify backend or installed runtime                                                                 |
| PM execution             | Supported MCP tools, checkpoint continuation, controls, stream recovery and instruction receipts are integrated; historical source5bc passed PM recovery13 before failing runtime stream bounds51                                                                                                                                                              | Latest02b/sourcec3fc fails pm_ack_migration; complete PM/PG matrix and owner flow remain unqualified                                                       |
| Native candidate QA      | [38116254698](https://github.com/FerrPOINT/fleet-control/actions/runs/38116254698)/1 on3c244/sourcec3fc FAIL; original closed readback verifies step9 cold/offline qualification and builder cleanup PASS, step10 original-nine wrapper state=failed, cuts skipped, exact alias cleanup PASS, artifacts0                                                       | Inner phase/class/cause and per-matrix cleanup unknown; no successful native scenario, cut acceptance or runtime promotion inferred                        |
| Forge                    | [38114891349](https://github.com/FerrPOINT/CI-CD/actions/runs/38114891349)/1 on c633/source84f is live at the latest metadata snapshot: A SUCCESS with original strict stages1-5 readback, B in progress                                                                                                                                                       | Fresh B/C receipts and original full12 aggregate pending; no native/full12 acceptance or old030 receipt reuse                                              |
| Base                     | [PR183](https://github.com/FerrPOINT/services-base/pull/183) merged externally as66b7faf; three maintenance helper blobs match the previously qualified payload and the Forge successor pins that merged object                                                                                                                                                | Consumer/native checks remain separate; no silent SDK or installed-packet promotion                                                                        |

Source8f/025 and fixture949 require qualification on their final inputs; passing
PM recovery on older5bc does not qualify these later changes. afab and Forge20e
remain distinct historical receipts in the verification ledger.

Forge [source84f0962](https://github.com/FerrPOINT/CI-CD/commit/84f0962ce14b95983a015e7d2303cd6f00ec59a7)
normally follows756: two QA files distinguish a new OCI operation path from
same-path replay when rollback repeats a Compose body; PG same-body rejection
and custody guards remain. Two existing docs also changed. Parent Linux pure
checks passed81/0 skips in0.320s and43/0 skips in5.748s, not Docker/native tests.
Old030 receipts are unchanged; historical278/sourcee781 DiskSpaceGuard is not
the latest failure. No recipe/policy/resource cause is inferred from ValueError.

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
Historical [38105834026](https://github.com/FerrPOINT/fleet-control/actions/runs/38105834026)/1
includes the reviewed continuation and archive corrections through source729,
but fails credentials_pg10/84 at
`pm_credential_creation::pm_mcp_publishes_tracker_receipts_then_resumes_only_after_saved_answer_and_terminal_proof`,
`backend/infra/tests/support/pm_credential_creation.rs:1178:75`. Original8411
readback verifies artifact11689882691; both scoped cleanups passed. The receipt
does not retain the error reason or identify the matrix case. No diagnosis or
successful qualification of the later foundation/PM gates follows from it.
The test-only successor maps continuation errors to distinct static panic sites,
so the existing safe frame report can identify a closed refusal category without
retaining error messages, SQL or private payloads. It does not identify a matrix
case, diagnose the previous failure or weaken any scenario/outcome assertion.
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
Rust OpenAPI generation (changed internal app trait) are separate gates.
Sourceaabe now has authentic generation38107719356/1, including the changed
dependency closure. Controls138342ee normally merge8411+aabe, retain all84
backend stages and add both Stop/archive regressions to the existing selections.
The [backend run38108696548](https://github.com/FerrPOINT/fleet-control/actions/runs/38108696548)/1
is terminal FAILURE. Original138 readback verifies artifact11690881634, ZIP SHA256
`f6588baf36704a08858f9ae223568bfbc259100947a9cec06707371cb6fe396d`;
both scoped cleanups passed. The static line23 frame means
`Conflict("effective configuration readback failed")`. Source review proves a
fixture inconsistency: replacement captures three enabled seed skills without
content, whereas the installed profile deliberately contains no skills.
Reviewed, published2dcff77 removes only those verified seed entries for its own
fixture before installation and checks production readback for the initial and
replacement/rollback profiles. All ten cases, replays and custody denials remain;
production guards are unchanged. This source defect matches the refusal category,
but the receipt does not establish which matrix case failed or qualify the fix.
Its API dependency closure is unchanged from authentic sourceaabe; reuse of that
export must retain its original provenance and verify exact closure parity.
The later9e/source2dc run38110519035/1 fails the fence-negative assertion at
line1218:13, not the earlier effective-config frame. Source review shows changed
custody returned Unavailable, which the public continuation wrapper maps to
Pending; the receipt itself does not retain that returned outcome or a matrix case.
Published c3fc175 returns Conflict before new run/control/workflow mutations.
The regression requires that exact refusal, unchanged predecessor/ledger and no
new saved-run/dispatch custody. Credential coordination/preparation precedes the
guard; this is not proof of zero side effects. It does not change the API closure
or relax a fence.
Historical a78/sourcec3fc38112575090/1 failed config_revision_pg UNKNOWN;
its three-selector inventory versus four selected tests is source-proven drift,
not proof of four PG PASS or a c3fc guard failure. It is not the latest backend result.
Latest02b/sourcec3fc38114888529/1 fails pm_ack_migration, gate exit101, test
`pm_ack_bounds_repairs_installed_022_preserving_custody_and_empty_roundtrip`,
frame `backend/migration/tests/pm_ack_bounds.rs:169:52`. Parent and Pascal original
strict readbacks verify artifact11693008835, ZIP SHA256
`66addc20dddcbff38129b1d7e0cedd34cdcedf11a3fcb16933f060b827fdd99f` and safe JSON
SHA256 `e0ece117ab3fa555a336606159fdfd8b59f22de28b7a41eceed6f98de0b12c6a`.
The log is untruncated; both scoped cleanups are true. No error body is retained.
Source inspection proves `up(None)` installed025/026 before `down(1)`, so the
fixture did not exercise024's downgrade guard. The parent-owned correction
bounds upgrade to REPAIR024 and asserts the ledger's last key. Its PG result is
pending; no production migration guard was weakened or shown faulty by this drift.
Separate bf4/38114924292/1 authenticates exact02b and passes Linux201/0 skips
in8.558s, not PostgreSQL or native acceptance. API reuse still requires exact
unchanged dependency closure and original sourceaabe codegen provenance.
Published source candidatec976c27 rejects config activation with409 before
drain when a bound PM run is pending, including no-journal/prepared/unknown
acceptance. Accepted-running PM retains normal drain. It adds no migration;
its PG regression is pending and the source8f receipts do not qualify it.

Native [38116254698](https://github.com/FerrPOINT/fleet-control/actions/runs/38116254698)/1
on3c244/sourcec3fc follows corrected two-phase UV component5ac37 (115 pure PASS).
Original closed readback verifies step9 cold build/offline qualification and
builder cleanup PASS. Step10 original-nine wrapper emitted `state=failed`;
no inner phase, class, cause, compile proof or scenario PASS is exposed. Cuts
were skipped; exact-owned alias cleanup passed and artifacts0. Alias absence
does not prove per-matrix volume cleanup or a complete daemon audit. Retained
receipt SHA256 `3d1d65106006c5c873b151adaaf16170c69bc829be1229843d59db8ff70eda46`.
This authorized isolated QA is not native acceptance or runtime promotion. Historical
f876/38113987817 remains Hermes UNKNOWN/FULL, outer1, grouped uv_sync/inner2,
five parities/guarded cleanup true, artifacts0 and native matrices skipped, not
root-cause proof. Historical4a UNKNOWN/FULL and257 UNKNOWN/TAIL are unchanged.
Exact receipts and earlier
431/579/1fa/bb failures remain in the
[verification ledger](CHAT_CLARIFICATION_VERIFICATION.md#current-hosted-qualification-2026-10-10).

Latest known point-in-time capacity observation,2026-10-11 01:49UTC, records
C:53.760GiB free against30GiB, available physical memory18.664GiB and available
commit2.990GiB, below the unchanged6GiB floor; this is not a fresh-now readback.
Daemon free disk is unconfirmed. Historical grouping audit35/0/0 passes with only
permanent projects, not capacity admission.
No local QA was launched under that capacity observation; the hosted wrapper
failure above supplies no native acceptance.

Base main was rechecked as380c66e on11 October04:00UTC. The three maintenance
helpers remain byte-identical to66b7; Auth cookie-isolation changes do not promote
the pinned SDK or installed packet. Exact-main
[38109161470](https://github.com/FerrPOINT/services-base/actions/runs/38109161470)/1
failed before any steps because of billing/spending limits, not test failures.
The published tree still lacks the workspace grouping audit and service-name
utilities; their foreign root copies are not a published dependency.

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
