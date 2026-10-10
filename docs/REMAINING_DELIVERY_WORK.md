# Remaining Delivery Work

## Scope And Evidence Boundary

This is the delivery map for the agreed runtime/execution scope: isolated agents,
configuration activation, Tracker assignment consumption, Hermes execution and
recovery, PM tools/continuation, production Chats, Forge delivery and shared Base
utilities. Leaders remain outside this scope. Tracker and Workflow are read-only
producer dependencies, not permission to implement a second scheduler in Fleet.
The complete execution goal is not reduced to the PM clarification vertical.

Use [CURRENT_STATE](CURRENT_STATE.md) for current exact-source CI evidence and
[GAP_REGISTER](GAP_REGISTER.md) for release blockers. Historical attempts below
retain their original identities; they are not the current implementation status.

Implemented means present in source; verified means the specific evidence below
exists for its own inputs; remaining means implementation or acceptance is still
required. A narrow PR can become merge-ready independently of the complete PM
vertical, but must preserve disabled/held paths and disclose its dependencies.
Neither that merge nor a healthy process enables PM admission.

## Current Workstreams

| Scope                  | Current boundary                                                                                                                                    | Required next evidence                                                                                         |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Isolated agents/config | Lifecycle, drain, effective revisions and rollback are in source; current full backend gate has not reached PG                                      | Current-source lifecycle/activation/rollback and physical two-agent isolation                                  |
| Tracker assignments    | PM has a dedicated path; generic task dispatch and readiness still hold. No all-seven-role lease consumer exists                                    | Producer-backed admission/heartbeat/fencing/first-step integration, without a Fleet scheduler                  |
| Hermes recovery        | Bounded original-key PM replay is integrated after independent review; optional free-chat recovery extension is not a PM prerequisite               | Actual production submission CAS/ACK, restart and unchanged native inference count                             |
| PM tools/continuation  | Structured tools, saved-answer continuation, owner controls and current Workflow Draft assignment alignment are integrated after independent review | Compatible real Tracker/Workflow/Base/Hermes calls and terminal/checkpoint/rebind acceptance                   |
| Production Chats       | Agent-to-task tabs and owner controls exist; history fixture correction is published; creation form remains an unapproved isolated preview          | Current three-engine fixtures/captures plus real owner/foreign-user and PM clarification flow                  |
| Forge                  | Full12 attempt stopped before cache allocation on its declared capacity policy; no first-job test ran                                               | Resolve environment admission, then exact-source attempts/pipelines/deployment/health/acceptance/rollback      |
| Base                   | Maintenance PR183 is published; its private CI jobs did not start due to billing/spending limits                                                    | Exact-head CI and native consumer checks; no automatic installed-packet promotion                              |
| Integration/release    | Normal-history integration preserves main and worker commits; full gates remain failed/incomplete                                                   | Regenerate API after domain changes, qualify final heads, inspect captures and publish reviewed task-owned PRs |

Read-only producer audit: Tracker PR114 at `357caa7a60a717eb7b0ac72f286b793326992931`
provides Analysis intent/reservation/heartbeat, but reservation does not authorize
dispatch and lacks a completed safe-stop release/reacquire path. Corresponding
execution lifecycles for Architect, Developer, Reviewer, Tester and DevOps are not
present. Workflow PR90 at `66e5d6db9fc2ae9129c9162688bacb1a98c7a4a3` has the role/mode
catalog and generic endpoints, but catalog presence is not execution authority.
Fleet now accepts the two existing Analysis metadata event types after independent
source review; Rust/HTTP/PG execution remains unqualified. See
[metadata compatibility](TRACKER_METADATA11_COMPATIBILITY.md).
These are explicit implementation/dependency gaps, not missing Hermes hooks.

## Historical Qualification Checkpoints

The following entries describe superseded sources, not current jobs or readiness.
At the earlier checkpoint, hosted frontend run38032377582 passed its default unit stage but failed
OpenAPI compatibility against newer main; union API verification remains required.
Backend run38033313593 fails with the exact authority-INSERT hint, not accepted.
Forge run38032103574 fails
bootstrap before any first-job tests; authenticated cleanup passes, root cause is
not yet known. Frozen PM source `fc29a9d` is an opt-in initial dispatch candidate,
not a completed tools/answer/resume flow. See [CURRENT_STATE](CURRENT_STATE.md)
for the exact evidence boundaries. No custom Hermes pre-model gate is in scope.

The following are recorded checkpoints, not live status widgets. Parent updated
terminal readbacks after the initial source5cc1fbb documentation review. Newest
rows supersede older in-progress entries only for their exact input boundaries:

| Evidence                                                                                                                             | Verified boundary                                                                                                                                                                                                                                  | Still not accepted                                                                                                                                                                                                                         |
| ------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [C11 run38028716924](https://github.com/FerrPOINT/fleet-control/actions/runs/38028716924), source `0e49430`, controls `7296506`      | Strict authenticated artifact11661626156;28 gates, workspace248 passed/18 ignored, credential unit8/PG15/real-Auth2 passed, both cleanup checks                                                                                                    | Component acceptance only; not installed Auth, secret custody, native runtime or PM/SDLC acceptance                                                                                                                                        |
| [Frontend run38027721811](https://github.com/FerrPOINT/fleet-control/actions/runs/38027721811), source `bf0ca7a`, controls `0917f33` | Authenticated artifact11660848489;21 stages including three-engine fixtures pass                                                                                                                                                                   | Terminal FAILURE at capture; no assertion/DOM or fresh images. Combined source60f35db needs its own full gate                                                                                                                              |
| [Backend run38028965529](https://github.com/FerrPOINT/fleet-control/actions/runs/38028965529), source `7dd6020`, controls `1200321`  | Authenticated artifact11661374366; preceding probes and both cleanup checks pass                                                                                                                                                                   | Terminal FAILURE in the same eight authorization cases at341/368; cause/full backend acceptance remain open                                                                                                                                |
| [Frontend run38030851556](https://github.com/FerrPOINT/fleet-control/actions/runs/38030851556), source `60f35db`, controls `4972214` | Authenticated artifact11661818565;10 preceding gates and cleanup pass; canonical837 blobs and69 pure controls checks                                                                                                                               | Terminal FAILURE at unit; artifact has no failing assertion. Local chat-detail reproduction59/60 leads to test-only59d00fe; all60 then pass in bounded Node22/threads/one-worker execution, not full355/default/browser/capture acceptance |
| Frontend `f2e8495`, retained at this `5cc1fbb` docs head                                                                             | Actual 348 tests in 36 files, Node22/threads/one worker; typecheck passed; later38023185173 also passes default hosted unit stage                                                                                                                  | Browser gate, fresh production screenshots and live cross-service flow                                                                                                                                                                     |
| [Backend run38021438217](https://github.com/FerrPOINT/fleet-control/actions/runs/38021438217), source `e369fed`, controls `eb214954` | Authenticated failure artifact11658528950; reaches container_activation_pg; both cleanup checks pass                                                                                                                                               | Terminal FAILURE in eight recovered-activation cases; hash fixture correction requires a new full gate                                                                                                                                     |
| [Backend run38023648801](https://github.com/FerrPOINT/fleet-control/actions/runs/38023648801), source `56daff9`, controls `6f648430` | Authenticated artifact11659287588; both cleanup checks pass; eight failures now at authorize calls259/286 after the corrected configuration probe                                                                                                  | Terminal FAILURE at container_activation_pg; authorization cause and full activation acceptance remain open                                                                                                                                |
| [C11 run38021888246](https://github.com/FerrPOINT/fleet-control/actions/runs/38021888246), source `994f29d`, controls `c5ee9bc`      | 309 inputs independently checked; 99 Linux pure control tests; closed terminal telemetry and both cleanup checks                                                                                                                                   | Terminal FAILURE at runtime_inventory; no credential/real-Auth test acceptance                                                                                                                                                             |
| [C11 run38024208930](https://github.com/FerrPOINT/fleet-control/actions/runs/38024208930), source `994f29d`, controls `4244772`      | Authenticated artifact11659222328; complete241/242 defaults,17/17 ignored, no extras, only orphan UUID test missing; both cleanup checks pass                                                                                                      | Terminal FAILURE; source3d1a108 links the existing test module without lowering counts; new compilation/Auth/PG acceptance required                                                                                                        |
| [C11 run38026078533](https://github.com/FerrPOINT/fleet-control/actions/runs/38026078533), source `3d1a108`, controls `cfe7805`      | Authenticated artifact11660525394, safe Clippy location coordinator.rs333 and both cleanup checks successful; canonical309 inputs/248 default/18 ignored                                                                                           | Terminal FAILURE; test-only iterator correction0e49430 requires a new exact-source gate                                                                                                                                                    |
| [C11 run38026636805](https://github.com/FerrPOINT/fleet-control/actions/runs/38026636805), source `0e49430`, controls `01f32fe`      | Normal public fast-forward; only one compiled blob changes; all helper functions/28 stages/248 default/18 ignored retained;115 Windows pure passes/four skips                                                                                      | Confirmed in progress at publication; actual Rust/PG/real-Auth terminal receipt required                                                                                                                                                   |
| [Frontend run38023185173](https://github.com/FerrPOINT/fleet-control/actions/runs/38023185173), source `5cc1fbb`, controls `bea500d` | Canonical835-file source inventory;67 pure control tests; default348 unit/typecheck/lint/build/theme/format passes; worker authenticates artifact11659871068, same two failed scenario declarations per engine,24 expected/nine skipped/zero flaky | Terminal FAILURE; captures/manifest skipped. No assertion/DOM evidence. Locator fix2398ff0 and command-journal fixture correction need integration and a fresh browser gate                                                                |
| [Frontend run38019603974](https://github.com/FerrPOINT/fleet-control/actions/runs/38019603974), source `b0ad56c`, controls `ba60890` | Terminal FAILURE in original three-browser fixtures                                                                                                                                                                                                | No browser acceptance; no claim that the prepare-time CRLF defect was its sole cause                                                                                                                                                       |
| Union API codegen [38015043570](https://github.com/FerrPOINT/fleet-control/actions/runs/38015043570)                                 | Authenticated artifact11656020208; current schema SHA256 `1167220ea9f3d65ddca4cce1112a26d53c77f8c1684ef958859f737f20210953`; client/drift/compatibility checks                                                                                     | Not Rust/PG/runtime or PM acceptance                                                                                                                                                                                                       |

The seven added scenarios in [fleet-control.spec.ts](../frontend/e2e/fleet-control.spec.ts)
are two conflict cases (409/412), four keyboard/URL-tab/unsaved-cancel viewport
cases, and one uncertain-custody retry/reload case. The earlier full browser gate
failed; its authenticated readback names the uncertain-custody scenario among two
failures in each engine, but does not identify its executed failing assertion.
Successor38027721811 passes fixtures and fails at capture; no full gate is accepted.
Earlier screenshots/browser results remain
tied to their older sources. Hosted default unit success does not accept E2E.

## Historical Delivery Matrix

Owners below are repository/contract owners, not permission to edit a sibling.
Tracker and Workflow remain read-only inputs for Fleet work. Parent coordinates
integration, exact-head gate publication and any admitted heavy execution.

| Task / state                                                                                       | Repo owner                                              | Prerequisites                                                                                                                                                | Independent work                                                                                                                                                                           | Acceptance evidence required                                                                                                                                                                                                     | Current blocker / stop condition                                                                                                                                                                                                                                             |
| -------------------------------------------------------------------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Combined backend and recovery: implemented, full gate failed                                       | Fleet / parent; artifact successor under review         | Exact product/control/source inventory, SDK19a, Auth01388, utility/package pins and genuine generated schema                                                 | Diagnose actual authorization failure after all read-only probes pass; retain the101-record regression and every authority guard                                                           | Terminal81-stage result with actual named unit/PG/HTTP/migration selectors, both21/24 lineages, explicit ignored cases, schema parity and owned cleanup; authenticate run/head/attempt/artifact                                  | Controls1200321/run38028965529 fail eight authorize cases at341/368; authenticated artifact11661374366 and cleanup do not establish root cause                                                                                                                               |
| Durable owner answer custody: implemented; live delivery unverified                                | Fleet; Tracker owns answer mutation                     | Current human/owner/project access, immutable task binding, compatible Tracker replay and original request/key                                               | Review journal concurrency/expiry/reload and negative authorization cases; prepare UI recovery evidence                                                                                    | Actual PG/HTTP custody cases plus authenticated owner reload/lost-response flow: original key/body/hash, no fresh command/run after unknown, revoked/foreign access denied                                                       | Saved-answer GET is not original-command GET. Fleet journal readback is available; it cannot invent upstream acceptance proof or treat an answer as PM delivery                                                                                                              |
| Real Auth credential semantics: narrow component verified, not runtime handoff                     | Fleet credential unit / Base Auth owner; parent gate    | Published source-qualified Auth01388 distinct from build SDK19a; real delegation policy, current parent/child subject/scopes and assignment                  | Prepare scoped publication with dependency disclosure; preserve admitted hold and credential custody semantics                                                                             | C11 controls7296506/source0e49430/run38028716924 authenticate all28 stages, credential unit8/PG15/real-Auth2 and cleanup; separately verify deployment before opt-in                                                             | Installed Auth, secret handoff, lease and model authority remain unverified. Earlier green01f incomplete artifact is rejected, not reused                                                                                                                                    |
| PM runtime/tools: Fleet wiring incomplete; Hermes unchanged                                        | Fleet runtime/tool owner; Tracker/Workflow coordination | Existing Hermes `/v1/runs` and supported tools, current task assignment/workflow state, ordinary backend authorization and isolated config                   | Implement actual Fleet dispatch/tool integration and continuation; remove the rejected custom producer-hook prerequisite without bypassing business state                                  | Real PM dispatch/questions/answers/revision flow, no duplicate run after unknown acceptance, correct owner/project checks and scoped tool writes                                                                                 | Extra Hermes pre-model authorization/reserved-run handshake is explicitly out of scope by owner decision. Current held Fleet paths are not working implementation                                                                                                            |
| Compatible PM producers and continuation: source contracts exist; Fleet consumption incomplete     | Tracker PR114 / Workflow PR90 owners (Fleet read-only)  | Exact deployed producer builds, Tracker lease/CAS/current assignment, genuine Workflow catalog/build, claim/checkpoint/native readiness and first-step proof | Fleet-side DTO/response validation and receiver fencing tests; record exact missing producer primitive rather than adding a local bypass                                                   | Actual authenticated Tracker outbox/reconnect; fenced predispatch claim/readback; terminal-or-proved-stop checkpoint/rebind; late-answer and unknown-CAS holds                                                                   | Recorded PR114 `357caa7a60a717eb7b0ac72f286b793326992931` Draft/main and PR90 `9b4107f0e8c886f37b4bace9921c15921b6f1604` Draft/master are not merged/installed. Namespace PR99/126 does not close this gate; unavailable genuine native catalog/build remains a prerequisite |
| Production Chats UI/browser: implemented, partially verified                                       | Fleet / Curie and parent                                | Authentic schema/client, exact final UI source, stable fixture/backend environment; live dependencies for live tests                                         | Qualify59d00fe including the keyboard activation wait, capture/native4 hook and seven new owner/journal/session-switch cases; execute all original/new fixtures and inspect fresh captures | Standard23 frontend gates/default pool, Chromium/Firefox/WebKit, screenshot integrity plus human visual review; then actual owner/foreign-user/project-revocation flow                                                           | Authenticated38030851556 fails at unit. Local chat-detail60 and native capture4 pass; full355/default/browser/screens and live PM remain unaccepted                                                                                                                          |
| Native lifecycle/config/recovery: implemented source and prepared drivers, not physical acceptance | Fleet runtime / Base protocol owners                    | Reviewed final-source driver, qualified controller/Hermes images and provenance, published compatible Base utility/helper, resource/ownership ACK            | Source/pure driver/cut review and safe image recipe/provenance qualification; retain held unsupported cuts                                                                                 | Real two-isolated-Hermes Compose flow: original mapping/home/workspace/config, health/unique transcript, drain/new generation, failure/exact rollback/peer unchanged, restart/unknown no second run or generation; exact cleanup | Latest retained controller/Hermes images were absent; local commit reserve was below6GiB. Fresh checks required, no guard waiver. Crash-cut source closure is not physical acceptance                                                                                        |
| Published Base maintenance input: safety source verified, CI/native open                           | Private Base / parent; Forge consumer owner             | Reviewed published43d0205/PR183 and exact new helper blobs; private CI access                                                                                | Qualify explicit caller policy/new hashes in a separate Forge successor; no automatic installed-packet replacement                                                                         | Parent92 focused tests and README/hub checks pass; require exact new blob/hash matches, private CI and separate native consumer proof                                                                                            | Old717 remains rejected. PR183 CI jobs do not start due to explicit billing-limit annotation; root Base/PR180/installed packet are untouched                                                                                                                                 |
| Forge full12 and deployment/rollback: prepared/source-reviewed, not full accepted                  | Forge / Feynman and parent coordinator                  | Qualified private maintenance packet, public-safe history, unchanged source/SDK/locks/budgets, fresh resources and explicit ACK                              | Close the immutable maintenance-packet prerequisite with minimal public-safe source/tests; retain deadlines and exact resource lists                                                       | All12 stages on exact source, real PG/OCI/deployment/evidence/rollback, original deadlines and final empty owned resources with permanent images/runtime unchanged                                                               | Public-safe1dbedf8 pure/source closure does not accept full12. Public Forge CI37973076579 SUCCESS on another source does not restore private Base CI or waive maintenance pin                                                                                                |

## Historical Plan Versus Source

This preserves the earlier source assessment of
[CHAT_CLARIFICATION_IMPLEMENTATION_PLAN](CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md).
Its held-path statements are superseded by the Current Workstreams above and
current state/gap documents; its historical source receipts are not upgraded.

| Approved plan item                             | Actual implemented boundary                                                                                                                                                                                                                                                                                       | Remaining deliverable                                                                                                                                                      |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Draft -> private PM chat                       | [Creation coordinator](../backend/app/src/pm_draft.rs) persists/reconciles Draft/input/reservation/chat; fresh namespace checks; [response](../backend/domain/src/pm_draft.rs) always has `dispatch_allowed=false`                                                                                                | Approve/connect production creation UI/controller, then fenced admission/first delivery; the isolated Draft preview is not approved production implementation or authority |
| Scoped PM credentials/tools                    | [Credential coordinator](../backend/infra/src/pm_credentials/coordinator.rs) saves original intent/ACK, verifies fresh principals/context; `prepare` discards the in-memory credential                                                                                                                            | Server custody and producer-authenticated exact-run tool handoff; no raw parent PAT/env inheritance; retain [task-bound runtime hold](../backend/infra/src/runtime/mod.rs) |
| Answer persistence separate from delivery      | [Protected command routes](../backend/api/src/routes/clarification_commands.rs) and [journal](../backend/infra/src/clarification_commands.rs) store original request/key/hash before delivery, expose scoped readback and hold ambiguity; [UI](../frontend/src/pages/chat-detail/index.tsx) loads pending custody | Actual current-source PG/HTTP and browser/live reload evidence; delivery ACK does not resume a run or publish requirements                                                 |
| Transcript/questions/requirements/confirmation | Production tabs, explicit choices/draft/conflict handling and exact-revision confirmation are present; frontend unit/typecheck evidence above                                                                                                                                                                     | Current three-engine keyboard/mobile/desktop evidence and real owner exact-revision/hash -> Tracker Backlog acceptance; changed revision must invalidate confirmation      |
| Projection/checkpoint/resume                   | Durable inbox/cursors/mirrors and opt-in poller have component evidence in the [verification ledger](CHAT_CLARIFICATION_VERIFICATION.md)                                                                                                                                                                          | Authenticated producer crash/reconnect, real checkpoint/terminal-stop/rebind and prerequisite verifier; projection is neither delivery nor stage completion                |
| Documentation/screens and rollout              | Genuine union schema is integrated; older fixture manifests are labeled by source                                                                                                                                                                                                                                 | Fresh inspected production evidence, required exact-head checks/reviews and opt-in only for compatible projects; disabling must preserve history                           |

Source interoperability already has a bounded proof: Fleetb249/Tracker357 seven
DTO comparison passed39/39 (20 comparator, nine schema-oracle, ten lexical).
That verifies nested closure/nullability/bounds/Analysis and response/owner replay
order at those inputs, not live HTTP or all later source. Likewise PM tools24
offline cases mean16 synthetic oracle cases plus8 real-module probes, not24
live tool calls. See the [handoff requirements](contracts/PM_TOOLS_HANDOFF_REQUIREMENTS.md)
for the missing producer primitives and acceptance matrix; no endpoint is assumed.

Base815's distinct retained fake-contract run passed224/224, zero skips,44.041s
at `.local/base-815-linux224-capture-20261009/run-020e27bd131b` (outside Git).
Raw SHA256 `52e85c0a93b1aacb38585f7a4c87f6a9c84d665159ff82f472da858d03378221`;
receipt SHA256 `dac80a5db4b024119b63d01db8ac83be099eb988e9c382a716131a3eacc5f5fa`.
This is not the historical88.807s run, private CI, helper publication or native
Docker acceptance. The historical installed-packet ComposeHelper SHA256 is
`2569882c3bb6a8b86ebb42367254720c5f802d5423bc98ef02c6fe3283e0874f`;
PR180's observed helper hash differs and the other two required maintenance
files are absent at815 (metadata-only observation at02:09:45 UTC, retained in
`.local/fleet-base180-helper-refresh-20261010/REPORT.md`). This does not prove
that no other private commit qualifies. Safety successor43d0205 changes the helper
and installer hashes and needs new consumer qualification; do not preserve unsafe
bytes merely to match the old pin. Do not substitute SDK19a or a dirty local
installation. The historical815 observation is not current43d0205 acceptance.

## Stop / Go Criteria

1. **Narrow release review:** GO only with an explicit task-owned diff,
   dependencies, exact migration ownership (at most one new migration per PR),
   generated API provenance and current required checks/review. STOP on a failed,
   running, missing or mismatched exact-head gate. The broad runtime assembly is
   not a substitute release PR; parent owns split/integration/publication decisions.
2. **Backend/recovery acceptance:** GO only after authenticated terminal receipts
   cover the complete reviewed selectors/lineages/package branch and cleanup.
   STOP on an ignored/conditional branch counted as PASS, aggregate-only hint,
   partial run or inheritance of older-source success. A diagnostic hint is not
   a root cause and does not authorize weakening a guard.
3. **Browser/UI acceptance:** GO for presentation only after original and new
   fixtures pass with all three engines, standard gates and fresh inspected
   captures at375x812,1920x1080,2560x1440 plus tablet dialogs. STOP on unit-only,
   old-source images, skipped browser cases or unknown delivery shown as success.
   Fixture GO still does not mean live PM GO.
4. **PM dispatch:** Hermes stays unchanged; no custom producer pre-model veto or
   reserved native-run authorization handshake is required. GO after Fleet
   dispatch/tools use the existing runtime API with current task/workflow state,
   ordinary owner/project access and durable one-run delivery. STOP on missing
   Fleet wiring or unknown prior dispatch, not on the rejected producer proposal.
5. **Live vertical:** GO only for real Draft -> PM questions -> owner answer ->
   final revision -> owner exact confirmation -> Tracker Backlog, with restarts,
   late/stale/foreign/project-revoked denials and no duplicate chat/answer/run.
   Test tool approval separately from business clarification. STOP while
   tools/delivery/checkpoint/rebind/prerequisite verification is absent.
6. **Native/Forge:** STOP until private maintenance pin, qualified images,
   ownership/resources and execution ACK exist; then require real exact-source
   matrices, original deadlines and final cleanup. Do not bypass private billing,
   expand resources implicitly, publish private source or change accepted runtime
   to manufacture evidence.

Priority order: integrate independently reviewed Workflow/PM recovery corrections;
regenerate the API and run current-source backend/browser gates; close real
assignment-consumer and producer-contract gaps in parallel; qualify private
maintenance/resources and run native/Forge acceptance before live SDLC claims.
These tracks can progress independently within their owners' scope; none grants
permission to cross another track's stop condition.

Detailed selector commands and evidence are in [TESTING](TESTING.md); historical
results remain in [CURRENT_STATE](CURRENT_STATE.md), [GAP_REGISTER](GAP_REGISTER.md)
and the [parallel work ledger](plans/2026-10-09-parallel-remaining-work.md).
