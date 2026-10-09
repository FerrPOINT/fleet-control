# Parallel Remaining Work: 9 October 2026

Status: five renewed implementation/verification assignments dispatched. This document
records work ownership, not completion or permission to deploy.

## Current Dispatch

The renewed parallel split keeps the existing workers and their isolated
checkouts rather than restarting completed work:

| Owner    | Current assignment                                                 | Required handoff                                                                                                              |
| -------- | ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------- |
| Feynman  | Integrate frozen configuration18 with recovery fixes               | Normal merge of bde64862 and be1b040; preserve original parents, one new migration and all config/preparation/recovery guards |
| Pascal   | Real two-agent Docker/Hermes acceptance driver preparation         | Isolated lifecycle/config/rollback live-driver source and explicit unsupported recovery holds; no execution before review/ACK |
| Ptolemy  | Correct demonstrated Forge smoke orchestration overhead            | Preserve all assertions, connection/role semantics and original timeouts; new reviewed packet, no automatic replay            |
| Anscombe | Authenticate actual lookup codegen artifact                        | Exact run37953053154/attempt1 and artifact11626597579 provenance without rerun or parent edits                                |
| Leibniz  | Independent preparation17/recovery16 safety review                 | Precise intent/permit/original-identity/concurrency findings; preserve QA0e until final source/codegen freeze                 |
| Parent   | Chat reload UI, browser checks, integration and scoped publication | Metadata-only recovery, generated client, frontend/browser evidence, reviewed worker integration and exact-head PR checks     |

These are continuations of the existing five workers, not additional competing
implementations. Each has a separate write set; the parent UI is read-only to
reviewers. Heavy jobs remain parent-admitted one at a time. The Forge slot has
been released after the terminal failure/cleanup below; no successor is admitted
merely because a worker prepared an invocation.

The four recovery16 fixes and provisioning17 have been normally merged at
`be1b040597a9ddd0847aca2c10fdaadb96e4c4a9`, preserving both original parents.
The 18 unique light cases pass; Rust/PG/physical Docker remain unverified.
QA0e coverage review found all 130 ignored identities and strict schema gates
present; that static audit is not a native test pass.

Parent reload recovery is a separate UI source unit: sessionStorage contains
only actor/session-scoped command handles, never steer text. Distinct original-key
readback and explicit receipt settlement are implemented in the UI, but depend
on Anscombe's new API and authentic generation before integration acceptance.
Earlier frozen results below remain historical evidence, not evidence for these
new changes.

The parent reload fixture now passes Chromium, Firefox and WebKit (three cases)
with original-key recovery and no automatic POST after reload. The first run's
Chromium beforeAll build exceeded the unchanged 120-second limit; Firefox and
WebKit passed. That failed packet remains in `frontend/test-results/runtime-controls-css`.
The complete successful rerun is separate in
`frontend/test-results/runtime-controls-css-recheck`. Its nine screenshots are
published through the explicit-input generator and verified; neither run is
live runtime acceptance. Independent review subsequently found two UI defects:
late callbacks after settlement and pre-reservation oversized input. Both are
fixed with eight new regression cases; all328 frontend tests pass. Independent
closure proofs pass both cases. Final browser verification in
`frontend/test-results/runtime-controls-css-final` passes all three engines,
including the UTF-8 guard, and its nine captures replace the fixture manifest.
These results are not inferred from the earlier packet.

UI source `ae027dd` and lookup producer `cfa30f39` are normally merged at
`82b7c8e`. The parent normal-pushed only the reviewed build controls at93a2d9d;
actual run37953053154 succeeds. Worker and parent independently authenticate
artifact11626597579 and its source-plan provenance. Actual schema SHA256
`b074c77295f7ad89912e3667124545ab66f6727184257d1f72030b4417c87f82`
and regenerated TypeScript are integrated; equality, fresh-main compatibility
and typecheck pass. Backend/PG/native acceptance remains required; codegen is
not a release or SDLC success.

Pascal's QA24 packet for be1b040 is prepared with 19 helper cases, 13 Fleet
Python and five original Base tests passing. Configuration18 is separately
frozen at bde64862, with 23 light Python cases passing. Neither has native
Rust/PG/physical Docker acceptance. Their normal integration and missing
live-driver preparation are independent successor tasks.

The actual Forge full12 packet dcbba1cb4e3f terminates exit1 at the unchanged
300-second smoke deadline. Python75 and row-smoke pass; smoke and the nine later
stages do not. Diagnostic call350/phaseL0236 is interrupted and reaped after
349 completed calls; partial markers are not acceptance. Parent inspected the
terminal probe and same-daemon cleanup: owned containers/networks/volumes empty,
baseline20/source265/protected caches preserved. No replay occurred. The next
task diagnoses and corrects demonstrated transport overhead without shrinking
the matrix, replacing real PostgreSQL or relaxing limits.

The parent integration is frozen at
`7f9ae892f9db482bbf44fda5d3082a074b6421b1`: a normal merge of the checked
foundation/UI source and backend `37ec604a`. Backend, CI and migrations match
that backend commit; frontend/API/Base inputs match `3f8ed8f`. Formatting and
staged-diff checks pass. The earlier 270 frontend tests apply to the identical
frontend tree, not to native execution of the newly integrated backend.
The acceptance worker receives this exact immutable source, including its 130
ignored integration cases. An inventory or prepared command is not a test pass.

All five assignments have been sent to the workers. Only the parent coordinates
heavy-job admission, integration and publication; workers do not independently
deploy or push release branches.

## Latest Frozen Results

Parent source `0e854c0e97beb9b3ac64eb1415247ce81604a460` retains the integrated
backend and adds authentic generated OpenAPI, typed GET-only command readback,
explicit security migration checks and regenerated fixture evidence. All286
frontend tests, typecheck/build/targeted lint, schema equality, contract snapshot,
compatibility8 tests and all three browser fixture cases pass. Nine screenshots
are generated/verified; native execution and automatic UI uncertainty settlement
remain unaccepted. Leibniz retargets the strict backend preparation to this SHA.

Original ancestry recovery is complete in a standalone object DB, with fsck
passing and no rewritten history. The parent normal-pushed build-only commit
`db2bb4bd`; [hosted run37945042300](https://github.com/FerrPOINT/fleet-control/actions/runs/37945042300)
successfully generated the exact fc6 source schema. Both worker and parent
authenticated artifact11622004476. No release PR or runtime deployment follows
from this generator run. The worker now reviews the consumer independently.

Automatic initial Docker preparation is frozen separately at
`337aac87092be42027f669fc0730858f3724824f`, one own migration17. Fourteen Python
checks and light source gates pass; Rust/PG/physical Docker remain pending.
Pascal's independent16 review found canonical Git/CRLF utility hash mismatch,
Unicode mapping serialization mismatch, slow-loop lease expiry and foreign-owner
heartbeat settlement risks. Fixes are a separate source unit, not silently
folded into the frozen provisioning commit. Config activation and replacement
remain Feynman's next implementation scope, not completed by preparation17.

Forge's reviewed diagnostic full12 helper passes35 pure integration tests
independently. Prepare-only orchestration is authorized with the original
resource/ownership guards; actual run requires a fresh seal review and separate
ACK. No inherited partial stage result is promoted to acceptance.

## Work Ownership

| Owner    | Independent task                                          | Deliverable and boundary                                                                                                                                                                                                                    |
| -------- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ptolemy  | Forge smoke diagnostics                                   | Private helper with phase/call timings, bounded child waits, cleanup and pure regression tests; retain the 300-second aggregate deadline and all assertions. No product change without a demonstrated cause.                                |
| Pascal   | Steer transcript durability                               | New backend follow-up from controls13: persist exactly one redacted control-message with original author/session/run identity when acknowledged; uncertain/rejected delivery must not become successful history. No UI or approval14 edits. |
| Leibniz  | Approval integration and backend verification preparation | Normal-merge frozen approval14 into recovery/controls in a new checkout; preserve every test/fence and prepare an exact combined inventory/helper. Do not alter generated contracts manually or treat preparation as execution.             |
| Feynman  | Docker mapped-volume/controller recovery                  | Continue the isolated lifecycle successor from `333c06d9`; retain Base custody, original generation/origin and durable receipts. No socket access for agents or unrelated SDK/pin changes.                                                  |
| Anscombe | Hosted Rust OpenAPI generation preparation                | Minimal build-only workflow, pinned source/Base/Rust, generated artifact with provenance and hashes. No push/dispatch until reviewed; no hand-edited schema or weakened existing CI.                                                        |
| Parent   | Integration, UI, documentation and publication            | Review source changes, generate client types from authentic Rust output, implement receipt states/readback/reload recovery, verify UI, run gates and publish scoped PRs.                                                                    |

Each worker owns a separate checkout/write set. Frozen commits and previous
evidence packets remain immutable. Task Tracker and project-workflow remain
read-only. Generated API/client files belong to the parent integration path.

## Successors And Verified Progress

The table above records the initial split. Completed source work now advances
through these non-overlapping successor assignments:

- Ptolemy: integrate the sealed diagnostic component into a new prepare-only
  full Forge orchestrator, keeping all 12 stages and the 300-second deadline.
  Parent independently passed all 32 diagnostic pure tests; no native rerun yet.
- Pascal: frozen steer mirror `71b17da7` adds seven regression cases and no
  migration. Review frozen mapped/controller16 correctness independently while
  Leibniz integrates that mirror. Linux/PG/native mirror acceptance is pending.
- Leibniz: frozen approval integration `a26d8b35` preserves normal parents
  recovery/controls and approval14; prepare42 passes 25 pure tests and inventories
  123 ignored cases. Next normal merge adds the separate steer mirror, not UI.
- Feynman: frozen mapped/controller16 `fc2e27b7` adds one migration and remains
  source-only. Next isolated unit implements automatic preparation with a durable
  pre-create intent and original Base reconciliation, not config activation.
- Anscombe: reviewed hosted-codegen controls are committed at `db2bb4bd`.
  Both pushes were rejected by missing shallow-ancestry object `09b35f1e`;
  no workflow run exists. Recover complete ancestry in a separate object DB,
  without modifying existing shared histories or inventing an ancestor.
- Parent: foundation [PR47](https://github.com/FerrPOINT/fleet-control/pull/47)
  is review-ready at `28c9a5ee`, main/CLEAN, all five exact-head CI jobs green
  after readiness reconciliation. CI passes 262 unit tests, 48 browser fixtures
  and 135 screenshot entries; 27 live tests skip. Local readiness browser9 pass,
  captures were inspected and the owned preview stopped.

The new foundation/UI integration passes all 270 unit tests across 33 files,
typecheck, production build, Rust formatting and 109 Markdown link checks. The
existing bundle-size warning is retained. Backend, API, Base pin and runtime
control UI code are unchanged by the foundation merge. Nine existing control
fixture images verify again. This does not certify controls/recovery on Linux,
physical Docker behavior, model admission or the live PM vertical.

## Existing Results

- Approval14 source is frozen at `68b59625e1b8a57139131a7672d90b6a3f465271`:
  one new migration, 21 added cases prepared, light checks pass. Rust/PG/native
  execution remains pending.
- Recovery/controls QA preparation contains 37 stages and explicitly covers all
  106 ignored cases; 20 pure tests pass. Its original source is `ed798638` and
  it must be retargeted/reviewed for later source and generated contracts.
- Native codegen preparation passes 17 pure tests but cannot execute under
  current host memory/commit reserves. The resource guard is not waived.
- Forge's latest full packet failed at the smoke aggregate timeout: only 2 of
  12 stages accepted. Owned resources were cleaned and independently checked.
  The delay's exact cause is not proven; diagnostic preparation is next.
- Integrated UI source `f32ecb7` retains 267 passing unit tests and three-browser
  fixture evidence with nine screenshots. Native controls/recovery are not
  accepted by those fixture results.
- Base Auth [PR126](https://github.com/FerrPOINT/services-base/pull/126) is
  review-ready at `dc43d0e25d60afa073c201e74d1a2cfe9aab8939`, with all ten
  exact-head CI checks successful at review. It is not merged or installed by
  this work split and does not establish end-to-end PM admission.

## Integration Order

1. Review backend approval integration and steer mirroring as separate changes;
   preserve original recovery/control identities and normal Git history.
2. Generate OpenAPI from the final Rust source, verify provenance, then generate
   frontend types. Retarget strict schema-parity and backend test inventories.
3. Complete receipt-state/readback/reload UI and regression/browser evidence.
4. Execute reviewed backend and Forge gates using explicit immutable source
   heads; retain failures and unknown outcomes instead of relabelling success.
5. Publish scoped PRs with exact-head CI and dependencies. Keep migration units
   separately reviewable; never publish the broad historical runtime tail as
   one release or claim fixture evidence as live SDLC acceptance.

## Resource And Acceptance Rules

Source work runs in parallel. Owned heavy Docker/Rust jobs run one at a time
after review and explicit orchestration ACK. Use genuine temporary Compose
projects, Base cleanup journal v2, exact disposable resource lists and immutable
Git exports. Do not restart Docker/WSL, prune shared resources, lower guards or
change accepted runtime images to make a test pass.

The remaining producer admission/readback dependency in project-workflow is a
separate integration blocker; no worker may introduce a model-admission bypass.
Completion requires actual Linux/PG, runtime and contract evidence, followed by
checked PR publication. Source commits, prepared helpers and healthy processes
alone are insufficient.
