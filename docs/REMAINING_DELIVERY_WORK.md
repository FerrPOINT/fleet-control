# Remaining Delivery Work

## Scope And Evidence Boundary

Source checkpoint: `5cc1fbb75f9096f10dc32250bf7bab5c36386f9c`, 10 October 2026.
This is a delivery map for the current Chats/PM clarification slice, not an
operational diary, deployment permission or a new test execution. Leaders,
delegation UI and the complete autonomous SDLC are outside this slice.

Implemented means present in source; verified means the specific evidence below
exists for its own inputs; remaining means implementation or acceptance is still
required. A narrow PR can become merge-ready independently of the complete PM
vertical, but must preserve disabled/held paths and disclose its dependencies.
Neither that merge nor a healthy process enables PM admission.

The following are recorded checkpoints, not live status widgets. Parent updated
terminal readbacks after the initial source5cc1fbb documentation review:

| Evidence | Verified boundary | Still not accepted |
| --- | --- | --- |
| Frontend `f2e8495`, retained at this `5cc1fbb` docs head | Actual 348 tests in 36 files, Node22/threads/one worker; typecheck passed; later38023185173 also passes default hosted unit stage | Browser gate, fresh production screenshots and live cross-service flow |
| [Backend run38021438217](https://github.com/FerrPOINT/fleet-control/actions/runs/38021438217), source `e369fed`, controls `eb214954` | Authenticated failure artifact11658528950; reaches container_activation_pg; both cleanup checks pass | Terminal FAILURE in eight recovered-activation cases; hash fixture correction requires a new full gate |
| [Backend run38023648801](https://github.com/FerrPOINT/fleet-control/actions/runs/38023648801), source `56daff9`, controls `6f648430` | Authenticated artifact11659287588; both cleanup checks pass; eight failures now at authorize calls259/286 after the corrected configuration probe | Terminal FAILURE at container_activation_pg; authorization cause and full activation acceptance remain open |
| [C11 run38021888246](https://github.com/FerrPOINT/fleet-control/actions/runs/38021888246), source `994f29d`, controls `c5ee9bc` | 309 inputs independently checked; 99 Linux pure control tests; closed terminal telemetry and both cleanup checks | Terminal FAILURE at runtime_inventory; no credential/real-Auth test acceptance |
| [C11 run38024208930](https://github.com/FerrPOINT/fleet-control/actions/runs/38024208930), source `994f29d`, controls `4244772` | Authenticated artifact11659222328; complete241/242 defaults,17/17 ignored, no extras, only orphan UUID test missing; both cleanup checks pass | Terminal FAILURE; source3d1a108 links the existing test module without lowering counts; new compilation/Auth/PG acceptance required |
| [C11 run38026078533](https://github.com/FerrPOINT/fleet-control/actions/runs/38026078533), source `3d1a108`, controls `cfe7805` | Normal public fast-forward; parent canonical135/105 Fleet/Rust blobs,309 combined inputs and248/18 declaration closure;114 Windows pure passes/four skips; worker118 Linux passes | Confirmed in progress at publication; actual Rust/PG/real-Auth terminal receipt required, not old994 acceptance |
| [Frontend run38023185173](https://github.com/FerrPOINT/fleet-control/actions/runs/38023185173), source `5cc1fbb`, controls `bea500d` | Canonical835-file source inventory;67 pure control tests; default348 unit/typecheck/lint/build/theme/format passes; worker authenticates artifact11659871068, same two failed scenario declarations per engine,24 expected/nine skipped/zero flaky | Terminal FAILURE; captures/manifest skipped. No assertion/DOM evidence. Locator fix2398ff0 and command-journal fixture correction need integration and a fresh browser gate |
| [Frontend run38019603974](https://github.com/FerrPOINT/fleet-control/actions/runs/38019603974), source `b0ad56c`, controls `ba60890` | Terminal FAILURE in original three-browser fixtures | No browser acceptance; no claim that the prepare-time CRLF defect was its sole cause |
| Union API codegen [38015043570](https://github.com/FerrPOINT/fleet-control/actions/runs/38015043570) | Authenticated artifact11656020208; current schema SHA256 `1167220ea9f3d65ddca4cce1112a26d53c77f8c1684ef958859f737f20210953`; client/drift/compatibility checks | Not Rust/PG/runtime or PM acceptance |

The seven added scenarios in [fleet-control.spec.ts](../frontend/e2e/fleet-control.spec.ts)
are two conflict cases (409/412), four keyboard/URL-tab/unsaved-cancel viewport
cases, and one uncertain-custody retry/reload case. The current full browser gate
fails; authenticated readback now names the uncertain-custody scenario among two
failures in each engine, but does not identify its executed failing assertion.
Earlier screenshots/browser results remain
tied to their older sources. Hosted default unit success does not accept E2E.

## Delivery Matrix

Owners below are repository/contract owners, not permission to edit a sibling.
Tracker and Workflow remain read-only inputs for Fleet work. Parent coordinates
integration, exact-head gate publication and any admitted heavy execution.

| Task / state | Repo owner | Prerequisites | Independent work | Acceptance evidence required | Current blocker / stop condition |
| --- | --- | --- | --- | --- | --- |
| Combined backend and recovery: implemented, not full-gate verified | Fleet / parent; Pascal recovery diagnosis | Exact product/control/source inventory, SDK19a, Auth01388, utility/package pins and genuine generated schema | Diagnose recovered authorization after the corrected hash probe; retain the101-record regression and every authority guard | Terminal81-stage result with actual named unit/PG/HTTP/migration selectors, both21/24 lineages, explicit ignored cases, schema parity and owned cleanup; authenticate run/head/attempt/artifact | Authenticated38023648801/source56 fails eight authorize calls259/286; no full-gate PASS or proved authorization cause |
| Durable owner answer custody: implemented; live delivery unverified | Fleet; Tracker owns answer mutation | Current human/owner/project access, immutable task binding, compatible Tracker replay and original request/key | Review journal concurrency/expiry/reload and negative authorization cases; prepare UI recovery evidence | Actual PG/HTTP custody cases plus authenticated owner reload/lost-response flow: original key/body/hash, no fresh command/run after unknown, revoked/foreign access denied | Saved-answer GET is not original-command GET. Fleet journal readback is available; it cannot invent upstream acceptance proof or treat an answer as PM delivery |
| Real Auth credential semantics: implemented preparation, not runtime handoff | Fleet credential unit / Base Auth owner; parent gate | Published source-qualified Auth01388 distinct from build SDK19a; real delegation policy, current parent/child subject/scopes and assignment | Qualify reviewed source3d1a108's added unit/PG/real-Auth regressions and connected UUID test without lowering inventory | Exact C11 Linux/Auth/PG result; real issuer replay/conflict/revoke/expiry and restart evidence; unchanged ACK custody and retained-down refusal; separately verify deployment before opt-in | Run38026078533 is confirmed in progress on the corrected source; no terminal product receipt. Component evidence cannot prove installed Auth, secret handoff, lease or model authority |
| PM tools and first-model admission: missing producer/custody integration | Hermes producer + Fleet admission/tool owner; Base identity owner | Authenticated reserved native run ID, awaited fail-closed pre-model veto, server credential custody, fenced assignment/workspace/config and Workflow first-step proof | Implement closed validators, durable original operation/send permit and scoped recovery tests behind the hold; no enabled gateway or guessed capability | Producer tests with zero first-provider calls on missing/deny/timeout/stale/unknown; one exact admitted run; concurrent/stale-run isolation; lost tool-write response/restart with no redispatch; no bearer in env/model/logs | Pinned Hermes `bbaf7af` lacks the qualified barrier/context handoff. `register_tool` and ContextVars are not authority; conversation ID and env fallback cannot substitute |
| Compatible PM producers and continuation: source contracts exist; Fleet consumption incomplete | Tracker PR114 / Workflow PR90 owners (Fleet read-only) | Exact deployed producer builds, Tracker lease/CAS/current assignment, genuine Workflow catalog/build, claim/checkpoint/native readiness and first-step proof | Fleet-side DTO/response validation and receiver fencing tests; record exact missing producer primitive rather than adding a local bypass | Actual authenticated Tracker outbox/reconnect; fenced predispatch claim/readback; terminal-or-proved-stop checkpoint/rebind; late-answer and unknown-CAS holds | Recorded PR114 `357caa7a60a717eb7b0ac72f286b793326992931` Draft/main and PR90 `9b4107f0e8c886f37b4bace9921c15921b6f1604` Draft/master are not merged/installed. Namespace PR99/126 does not close this gate; unavailable genuine native catalog/build remains a prerequisite |
| Production Chats UI/browser: implemented, partially verified | Fleet / Curie and parent | Authentic schema/client, exact final UI source, stable fixture/backend environment; live dependencies for live tests | Integrate reviewed2398ff0 locator fix and align the old runtime fixture with journal/store/delivery, preserving uncertainty; execute all original/new fixtures and inspect fresh captures | Standard23 frontend gates/default pool, Chromium/Firefox/WebKit, screenshot integrity plus human visual review; then actual owner/foreign-user/project-revocation flow | Authenticated38023185173 fails two scenario declarations per engine after actual default348 unit success; no executed assertion evidence or fresh captures. Fixes and live PM remain unaccepted |
| Native lifecycle/config/recovery: implemented source and prepared drivers, not physical acceptance | Fleet runtime / Base protocol owners | Reviewed final-source driver, qualified controller/Hermes images and provenance, published compatible Base utility/helper, resource/ownership ACK | Source/pure driver/cut review and safe image recipe/provenance qualification; retain held unsupported cuts | Real two-isolated-Hermes Compose flow: original mapping/home/workspace/config, health/unique transcript, drain/new generation, failure/exact rollback/peer unchanged, restart/unknown no second run or generation; exact cleanup | Latest retained controller/Hermes images were absent; local commit reserve was below6GiB. Fresh checks required, no guard waiver. Crash-cut source closure is not physical acceptance |
| Published Base maintenance input: fake-contract verified only | Private Base / maintenance owner | Published immutable private commit with all three required helper blobs, authenticated delivery and private CI access | Qualify private Git pin and public-safe reference-only controls; no copied private source in public ancestry | Three exact blob/hash matches, helper-v2 journal/cleanup compatibility and exact-head private CI; native consumer evidence stays separate | Forge manifest commit remains null. PR180 at815 does not supply the required three-file packet; private billing blocks execution, not a failed product test |
| Forge full12 and deployment/rollback: prepared/source-reviewed, not full accepted | Forge / Feynman and parent coordinator | Qualified private maintenance packet, public-safe history, unchanged source/SDK/locks/budgets, fresh resources and explicit ACK | Close the immutable maintenance-packet prerequisite with minimal public-safe source/tests; retain deadlines and exact resource lists | All12 stages on exact source, real PG/OCI/deployment/evidence/rollback, original deadlines and final empty owned resources with permanent images/runtime unchanged | Public-safe1dbedf8 pure/source closure does not accept full12. Public Forge CI37973076579 SUCCESS on another source does not restore private Base CI or waive maintenance pin |

## Plan Versus Actual Source

This reconciles [CHAT_CLARIFICATION_IMPLEMENTATION_PLAN](CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md),
not every historical sentence in the state ledgers.

| Approved plan item | Actual implemented boundary | Remaining deliverable |
| --- | --- | --- |
| Draft -> private PM chat | [Creation coordinator](../backend/app/src/pm_draft.rs) persists/reconciles Draft/input/reservation/chat; fresh namespace checks; [response](../backend/domain/src/pm_draft.rs) always has `dispatch_allowed=false` | Approve/connect production creation UI/controller, then fenced admission/first delivery; the isolated Draft preview is not approved production implementation or authority |
| Scoped PM credentials/tools | [Credential coordinator](../backend/infra/src/pm_credentials/coordinator.rs) saves original intent/ACK, verifies fresh principals/context; `prepare` discards the in-memory credential | Server custody and producer-authenticated exact-run tool handoff; no raw parent PAT/env inheritance; retain [task-bound runtime hold](../backend/infra/src/runtime/mod.rs) |
| Answer persistence separate from delivery | [Protected command routes](../backend/api/src/routes/clarification_commands.rs) and [journal](../backend/infra/src/clarification_commands.rs) store original request/key/hash before delivery, expose scoped readback and hold ambiguity; [UI](../frontend/src/pages/chat-detail/index.tsx) loads pending custody | Actual current-source PG/HTTP and browser/live reload evidence; delivery ACK does not resume a run or publish requirements |
| Transcript/questions/requirements/confirmation | Production tabs, explicit choices/draft/conflict handling and exact-revision confirmation are present; frontend unit/typecheck evidence above | Current three-engine keyboard/mobile/desktop evidence and real owner exact-revision/hash -> Tracker Backlog acceptance; changed revision must invalidate confirmation |
| Projection/checkpoint/resume | Durable inbox/cursors/mirrors and opt-in poller have component evidence in the [verification ledger](CHAT_CLARIFICATION_VERIFICATION.md) | Authenticated producer crash/reconnect, real checkpoint/terminal-stop/rebind and prerequisite verifier; projection is neither delivery nor stage completion |
| Documentation/screens and rollout | Genuine union schema is integrated; older fixture manifests are labeled by source | Fresh inspected production evidence, required exact-head checks/reviews and opt-in only for compatible projects; disabling must preserve history |

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
Docker acceptance. Required ComposeHelper SHA256 remains
`2569882c3bb6a8b86ebb42367254720c5f802d5423bc98ef02c6fe3283e0874f`;
PR180's observed helper hash differs and the other two required maintenance
files are absent at815 (metadata-only observation at02:09:45 UTC, retained in
`.local/fleet-base180-helper-refresh-20261010/REPORT.md`). This does not prove
that no other private commit qualifies. Do not substitute SDK19a or a dirty local
installation. Neither external input was re-polled or executed for this document.

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
4. **PM admission:** STOP until genuine producer pre-model veto, exact native-run
   context, current Tracker/Workflow fences and server credential custody are
   implemented and exercised. Missing/expired/unknown authority means zero model
   dispatch and no replacement key/run. Namespace readback, credentials, runtime
   health and configuration readiness cannot individually authorize admission.
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

Priority order: close the diagnosed backend/credential qualification failures;
complete the current UI fixture/browser gate and narrow credential regressions; qualify private
maintenance/images and execute approved runtime/Forge gates; implement the missing
producer admission/tool/continuation contract before attempting live PM acceptance.
These tracks can progress independently within their owners' scope; none grants
permission to cross another track's stop condition.

Detailed selector commands and evidence are in [TESTING](TESTING.md); historical
results remain in [CURRENT_STATE](CURRENT_STATE.md), [GAP_REGISTER](GAP_REGISTER.md)
and the [parallel work ledger](plans/2026-10-09-parallel-remaining-work.md).
