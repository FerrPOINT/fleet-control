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
read the instructions or completed the workflow. Latest backend38112575090/1
on a78/sourcec3fc is terminal FAILURE at config_revision_pg: gate exit1,
command exitnull, UNKNOWN with no retained test/frame. Source inspection finds
three expected selectors versus four selected source tests; the receipt does not
prove four PG passes or diagnose the c3fc guard. Prior9e/source2dc38110519035/1
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

| Scope                    | Verified evidence                                                                                                                                                                                                                                                                                                                                                  | Still open                                                                                                                                                                       |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust OpenAPI, sourceaabe | [38107719356](https://github.com/FerrPOINT/fleet-control/actions/runs/38107719356)/1 SUCCESS on controls12de; artifact11689594281; original strict readback independently verified. Schema SHA256 `afa46ac37b726232eda73df46c24d1d42c796f8873eefb68454fbe0f243df501`, unchanged; genuine307-input export includes the changed app trait                            | Complete backend/DB/native checks remain separate from schema generation                                                                                                         |
| Backend                  | [38112575090](https://github.com/FerrPOINT/fleet-control/actions/runs/38112575090)/1 FAIL on a78/sourcec3fc at config_revision_pg; original strict artifact11692069237 readback: gate exit1, command exitnull, UNKNOWN, empty diagnostics/failed_tests, readable/untruncated, harnessnull; both scoped cleanups true                                               | Source count drift3 expected/4 selected is proven, not four PG PASS, credentials PG PASS or a failure of the c3fc fix; final backend/API/native/live qualification remains open  |
| Backend pure controls    | [38112684097](https://github.com/FerrPOINT/fleet-control/actions/runs/38112684097)/1 SUCCESS on3b379; exacta78 checkout authenticated; native Ubuntu24 original suite201 PASS,0 skips/failures/errors,8.895s, exit0                                                                                                                                                | Pure controls only, not backend/PG/native PM acceptance. Prior [38111234565](https://github.com/FerrPOINT/fleet-control/actions/runs/38111234565)/1 201/0 skips qualifies only9e |
| Frontend, sourceaca/fc0  | [38072655687](https://github.com/FerrPOINT/fleet-control/actions/runs/38072655687) SUCCESS; original strict readback verifies artifact11677891567. All23 gates,461 units,135 catalogue/186 fixture images; all three browsers47 passed/zero flaky with nine opt-in live skips each. 135 catalogue and9 PM views imported, selected corrected captures reviewed     | Complete live PM acceptance; fixture success does not qualify backend or installed runtime                                                                                       |
| PM execution             | Supported MCP tools, checkpoint continuation, controls, stream recovery and instruction receipts are integrated; historical source5bc passed PM recovery13 before failing runtime stream bounds51                                                                                                                                                                  | Latest a78/sourcec3fc config_revision_pg UNKNOWN does not qualify the PM/PG matrix or owner flow; current source corrections remain unqualified                                  |
| Native candidate build   | [38113987817](https://github.com/FerrPOINT/fleet-control/actions/runs/38113987817)/1 FAIL onf876/sourcec3fc, step9 Hermes candidate build outer exit1/UNKNOWN/FULL; closed recipe_instruction=uv_sync, inner_exit_code=2; five parities true, guarded cleanup passed, artifacts0; native-nine/cuts skipped                                                         | Identifies the grouped RUN only, not root cause, failed subcommand or receipt acceptance; no offline/native qualification or runtime promotion                                   |
| Forge                    | [38109147644](https://github.com/FerrPOINT/CI-CD/actions/runs/38109147644)/1 on030/source756 FAIL: A5 PASS; B PG5/24 negatives/14 cleaned journals PASS; C OCI7 rollback assertion oci_delivery.rs:625:5, then secondary cleanup assertion477:17, closed child ValueError; stages8-12 NOT_RUN; original aggregate rejects C, owned cleanup/zero remaining verified | Published source84f QA rollback fix has Linux81+43 pure PASS only; native/full12 pending. Neither the closed class nor pure tests establish actual root cause or acceptance      |
| Base                     | [PR183](https://github.com/FerrPOINT/services-base/pull/183) merged externally as66b7faf; three maintenance helper blobs match the previously qualified payload and the Forge successor pins that merged object                                                                                                                                                    | Consumer/native checks remain separate; no silent SDK or installed-packet promotion                                                                                              |

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
The later a78/sourcec3fc38112575090/1 is terminal FAILURE at config_revision_pg.
Original strict artifact11692069237 readback verifies ZIP SHA256
`47e2b32a00f3956d0002a2b2fbab556b7e8d4b43f724b24dbc65f60f4153f996`;
gate exit1, command exitnull, UNKNOWN, no retained diagnostics/failed tests,
readable/untruncated log, null harness result and both scoped cleanups true.
Canonical a78 inventory expects3 config_revision_pg tests while its filter
selects4 in c3fc, including the archive recheck regression. This source-proven
qualification drift does not establish all four PG passes, credentials PG PASS
or a c3fc fix failure. Linux201 pure-controls PASS is a separate result.
Published source candidatec976c27 rejects config activation with409 before
drain when a bound PM run is pending, including no-journal/prepared/unknown
acceptance. Accepted-running PM retains normal drain. It adds no migration;
its PG regression is pending and the source8f receipts do not qualify it.

Native [38113987817](https://github.com/FerrPOINT/fleet-control/actions/runs/38113987817)/1
fails onf876/sourcec3fc at step9 Hermes candidate build: outer exit1, UNKNOWN/FULL,
closed `recipe_instruction=uv_sync`, `inner_exit_code=2`. Original frozen readback
identifies the grouped hash-checked RUN, not a failed subcommand or root cause.
Five parities true, builder cleanup cleaned and exact-owned alias cleanup passed;
artifacts0, native-nine/cuts skipped. This authorized isolated QA is not offline/
native acceptance or runtime promotion. Historical4a/source2dc38111540024 remains
UNKNOWN/FULL without recipe attribution;257/source5db38081865039 remains
UNKNOWN/TAIL. Neither is reclassified. Exact receipts and earlier
431/579/1fa/bb failures remain in the
[verification ledger](CHAT_CLARIFICATION_VERIFICATION.md#current-hosted-qualification-2026-10-10).

Latest known point-in-time capacity observation,2026-10-11 01:49UTC, records
C:53.760GiB free against30GiB, available physical memory18.664GiB and available
commit2.990GiB, below the unchanged6GiB floor; this is not a fresh-now readback.
Daemon free disk is unconfirmed. Historical grouping audit35/0/0 passes with only
permanent projects, not capacity admission.
No local QA was launched under that capacity observation; hosted candidate-build
failure above supplies no native qualification.

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
