# Implementation Plan

## Merge-Ready Remaining Gates: 8 October 2026

This is the remaining full execution scope, not a completion percentage. The
integration branch is not an installed release. No fixture, process health or
individual terminal run substitutes for business acceptance.

| Gate | Required implementation and acceptance | Ownership/dependency |
| --- | --- | --- |
| Runtime/configuration | Complete initialized/loaded inventory and readiness; remaining activation stop/post-commit/ack crash and backup-loss cases; preserve original custody and unknown-command holds. | Fleet; consume exact compatible Base utilities. |
| Predispatch admission | Trusted current assignment, lease claim/heartbeat/fence and original-key recovery; a non-circular first workflow step before any model call. No invented delivery/decomposition references before PM. | Tracker/Workflow producer changes are external, read-only here; Fleet consumes verified contracts. |
| PM execution/resume | Structured scoped tools, saved-answer delivery distinct from persistence, checkpoint/rebind after terminal or proven safe stop, and unknown acceptance readback without redispatch. | Fleet, after producer admission compatibility. |
| Production Chats | Agent/task hierarchy and dialogue/clarification/requirements against real APIs; owner-only actions, stale/conflict/partial-success states, no parallel run or prompt bypass of clarification. | Authorized parallel Chats task; preserve separate agent transcripts. |
| Forge delivery | Current-source native PostgreSQL/OCI recovery, task attempt/candidate pipeline and artifact checks, deployment/health/acceptance receipts and rollback. Local verification remains non-admitting until trusted producer authority exists. | CI-CD; PR87 is a separate prerequisite, PR88 stays Draft pending gates. |
| Live acceptance/evidence | Actual PM question -> owner answer -> final exact revision -> owner confirmation; full agreed seven-agent delivery/integration/Rework/deployment flow and restart/foreign-access/stale-lease negatives. Current screenshots and docs must distinguish live evidence from fixtures. | Compatible isolated test project; do not promote accepted runtime. |
| Ordered publication | Separate task-owned commits/PRs, normal history reconciliation, each migration released in its owned packet, exact main-target heads and current CI/review evidence. | Fleet/CI-CD/Base only; no force push or automatic merge. |

Read-only producer audit at Tracker114 `357caa7` and Workflow90 `1139871`
confirms that enrolled Tracker business commands reject without verified PM
admission; the existing Workflow PM bind requires an already-running Fleet run,
and its Base first-step gate still refuses missing trusted owner evidence.
The lease GET implemented by Fleet is observation, not a claim or permission.
Producer acceptance must cover admitted transition/original-key readback,
expired-lease quiescence and a pre-model first-step/identity contract before
Fleet wires tools or resume. See [producer gaps](GAP_REGISTER.md#exact-producer-predispatch-limits-8-october-2026).

Current publication boundaries: Base PR169 at `4a7d4a0` has ten successful
exact-head checks and remains unmerged; Fleet integration at `b965298` is
published but has no full main-target release gate. Its normal reconciliation
with accepted main `c8093aa`, published at `683345d`, preserves Base875 and unchanged integration
migrations;661 Linux/PG and448 frontend cases,84 three-engine fixture flows and
controlled access-boundary regressions pass. See
[evidence](CHAT_CLARIFICATION_VERIFICATION.md#main-history-reconciliation-8-october-2026).
This branch is not a giant main release PR or installed SDLC acceptance.
Forge PR88's four successful
checks are for `56f1217`, not subsequent uncommitted reader/proof hardening.
Latest frozen259-input epoch0f560932eb2b completed on8 October with PostgreSQL
3/3 passing, including all five physical SIGKILL checkpoints. Its37 Python
diagnostic tests, SQL row/safety smoke, locked check and strict Clippy passed.
The full packet nevertheless failed in OCI:0/1 passed, with an elapsed timeout
at the original runner completion wait in `tests/support/task_delivery.rs:287`.
No complete OCI artifact/rollback acceptance or subsequent full follow-up gates
ran. Current/frozen source parity and exact-project cleanup were verified.
Diagnose runner completion without widening timers or treating partial build
output as success. The previous epoch8f596c025d25 remains a separate failed
2/3 PostgreSQL packet; its backup rehearsal failure cause is still unknown.
The new pass does not explain or erase that historical failure. Old-head CI
does not certify the later dirty source packet.

Parallel Chats PR60 at `99d1c7f` targets the integration branch rather than the
approved `main`. Directly changing its base would import the broad integration
tail; do not do that or claim the PR as main-ready. Prepare the dependency/main
release sequence first. Its component/browser/screenshot evidence does not
prove a live PM round trip. Tracker and Workflow remain unchanged by this task.

### Remaining Delivery Order And Exit Criteria

Seven work packages remain; they are not seven equally sized fixes or a
percentage estimate. Implemented/component-verified code is retained. Final
readiness requires the following exits, not another mock-only demonstration.

1. **Runtime and configuration recovery (Fleet/Base).** Complete loaded native
   inventory and SDLC readiness; accept drain/activation/rollback, remaining
   stop/post-commit/ack crash cases and backup loss. Re-run the physical observer
   matrix with the guarded failed-export cleanup correction. Source tests77
   container and54 native-host cases do not replace physical acceptance. Exit:
   isolated agents preserve effective configuration and peer state across every
   required crash, with unknown effects held and exact cleanup verified.
   Latest9d0d86 packet passes212 components and guarded own cleanup, but actual
   before-create recovery fails at the original180s deadline. Retain a safe
   phase/guard diagnostic before the next fresh physical attempt; cause remains
   unknown and none of the four observer combinations is accepted.
2. **Predispatch contract (external producers, Fleet consumer).** Obtain trusted
   Tracker assignment/claim/heartbeat/fencing and a Workflow first step before
   model dispatch, without requiring an already-running Fleet run or future
   decomposition evidence. Exit: expired/foreign assignments and missing steps
   cause zero model calls; accepted assignments recover by original-key readback.
   Tracker/Workflow producer fixes require their own writable tasks, not edits here.
3. **Real PM tools and continuation (Fleet).** Wire structured questions,
   saved-answer delivery, exact requirement revision, checkpoint/rebind and safe
   stop into compatible producer contracts. Exit: persistence and delivery are
   distinguishable; restart/unknown acceptance never redispatches an uncertain
   command; only the owner confirms the current revision to advance the task.
4. **Production Chats (Fleet, parallel UI work).** Connect agent/task navigation,
   dialogue/clarification/requirements, real execution states and owner actions.
   Preserve conflict input, partial success and reconnect history. Exit: a real
   PM question/answer/requirements/confirmation flow works in all three browsers;
   fixture flows and screenshots remain explicitly separate evidence.
5. **Actual delivery and rollback (Forge, independent work).** Diagnose the OCI
   runner completion failure, then accept current-source image/artifact identity,
   candidate pipeline, deployment health/acceptance receipts and rollback. Retain
   the new PostgreSQL3/3 packet and historical failures. Exit: both physical PG
   and OCI acceptance plus the full exact-source follow-up gates pass; prerequisite
   PR87 is handled separately from PR88, with no invented business authority.
6. **Cross-service business acceptance (all compatible services).** Run the
   actual seven-agent scenario: PM clarification/publication, two child tasks,
   defect/Rework, integration and a deployed application with acceptance evidence.
   Include foreign access, stale lease/revision, duplicate commands and restart.
   Exit: verified business receipts and live UI captures, not healthy processes
   or successful component runs.
7. **Ordered merge-ready publication and documentation (owned repositories).**
   Reconcile normal history, preserve accepted migration bytes and publish each
   dependent one-new-migration release packet with exact-head CI/review. Update
   contracts, ledgers, route/screenshots manifests and operational instructions.
   Exit: no undisclosed blockers or draft-only evidence presented as a release.
   Merge and installed-runtime promotion remain separate explicit decisions.

Packages1 and5 can proceed independently. Package4 can finish UI behavior while
producer work proceeds, but its live exit depends on2 and3. Package6 follows
runtime/admission/PM/Chats/delivery acceptance; publication is incremental, while
whole-product readiness follows6. Heavy native Docker checks share a controlled
execution slot; independent source/test work can remain parallel.

Do not promise a finish date from test counts. After runtime/OCI failures and
the external admission contract are closed, re-estimate against the remaining
live scenarios and dependent release units. Until then the honest status is
substantial verified implementation, with full execution acceptance incomplete.

### Next Main Release Unit

Before reconciliation, readback identified main `c8093aa`,180 integration-only
commits and12 main-only commits. Published normal merge `683345d` retains both
histories; that does not release the integration migration tail. Foundation PR47 is Draft/main at
`4cc9a8ade539b9df67b9aaebf6ebe039d72814e5`, CLEAN with five successful checks.
Its only new migration is task chats000010; its published populated-down guard
must survive subsequent normal history reconciliation. It still does not claim
live PM admission. Do not import the integration tail or retarget Chats PR60.

After foundation47 is accepted, the next schema unit is persisted PM credential
preparation from `9d92f1b`: immutable credential intent/receipt, coordinator,
repository persistence, Draft continuation and server wiring, with only000011
registered for both supported database lineages. Exclude journal12+, later lease
readback and observer/container/controller work. Retain the accepted main SDK pin
`875cac2`; earlier gates against another pin do not certify this release head.
Required acceptance: populated canonical/split upgrade preserving data/history,
empty down/up, populated credential downgrade refusal, concurrent/lost-ACK/restart
preparation, rotation/revocation/expiry, audit rollback, redaction and zero model
dispatch. Publish against main only after exact-source gates and review.

Release order then follows the actual registry: journal12, controls13, time14,
control outcomes15, approval outcomes16, launches17, preparations18, endpoints19,
recovery20, recovery delivery21 and stop delivery22. Each release owns at most one
new migration and retains accepted historical bytes. Preserve all integration
commits and main fixes without rebase/squash/force push; existing source packets
are not permission to bypass dependent release checks or install the candidate.

## Current Publication And Remaining Work

The original preparation custody correction is source/component verified:
existing-only checks prevent cached receipts from reconstructing a missing DB
claim; 208 actual PostgreSQL/runtime cases and 64 host cases pass. Base producer
[PR169](https://github.com/FerrPOINT/services-base/pull/169) is ready with ten
successful exact-head checks, not merged/installed. The owned before-create/
lost-ACK packet `929400f20d40` passes actual physical recovery and cleanup.
Fresh candidate-running packet `22a5078c52f5` also passes after the independently
reviewed QA teardown correction, with327 other frozen inputs identical. Publish
the scoped source packet without claiming ordered release or whole-SDLC readiness.
Keep unknown starts held; initial continuation may only read back the same pending
preparation without
an open launch. Record failed packets and cleanup repairs without turning them
into acceptance. See [evidence](CHAT_CLARIFICATION_VERIFICATION.md#preparation-readback-8-october-2026).

The original PM proof packet binds private reservations to launch/controller/
origin/credential, rechecks physical custody after blocking row locks and makes
terminal replay write-free. Final source verification passes623 workspace cases,
22 migration cases, three profile cases and one renderer export with byte-identical
OpenAPI; see [evidence](CHAT_CLARIFICATION_VERIFICATION.md#original-pm-runtime-proof-8-october-2026).
No migration, SDK pin, installed update or dispatch authority is added. Ordered
release-head CI and actual PM tools/delivery/checkpoint/resume remain required.

The canonical heartbeat correction is integrated as `7dd19f1` without importing
unrelated auth/SDK changes. Combined workspace, PostgreSQL migration/profile,
renderer/OpenAPI, frontend and three-browser fixture evidence is recorded in
the [verification ledger](CHAT_CLARIFICATION_VERIFICATION.md#combined-heartbeat-closure-8-october-2026).
The source/PostgreSQL heartbeat blocker is closed; this is not an installed
runtime update or whole-product merge readiness.

The authorized parallel Forge task publishes OCI artifact identity, read-only
data compatibility and a mutable PostgreSQL backup/drain/migration/restore
packet in
[CI-CD PR88](https://github.com/FerrPOINT/CI-CD/pull/88), exact head
`56f1217cfbd7264c219f8d6bee184c9252520785`, with four successful CI checks.
The worker reports real PostgreSQL 3/3, OCI 1/1, backend 297 and frontend 201
cases passing, owned cleanup and a clean Docker audit. Parent independently
read back the exact head and four checks; it did not rerun the Forge gates.
Its prerequisite PR87/0039, authoritative SDLC admission and full business
acceptance remain separate; the PR stays Draft and no installed deployment changes.

The same independent Forge chat has completed its bounded mutable PostgreSQL
packet without a new Forge SQL migration or changes to prerequisite PR87.
Its write scope is CI-CD only. Do not reinterpret the prior read-only packet
`205a7b9` or this controlled PostgreSQL acceptance as full SDLC authority;
missing producer authority, unknown writers and unverified backups must hold
effects. Fleet runtime/PM development remains separate.

Next required gates remain initialized native inventory; trusted Tracker
assignment and Workflow first-step authority; PM structured tools, saved-answer
delivery/checkpoint/rebind; and live SDLC acceptance. Consume only verified
producer contracts: matching seven DTOs does not authorize dispatch. Keep
Task Tracker/project-workflow read-only in this task. Release the existing
thirteen migration candidates in ordered single-migration packets with exact
main-head checks, never as one giant integration PR. Do not enable installed
flags or advertise100% readiness from component/fixture acceptance.

Current exact producer source limits narrow the next admission slice to a
machine-only Tracker lease GET/readback with closed identity/version/expiry
validation. Keep it independent from human creation continuation and return
prerequisite observation only, never dispatch permission. Tracker114 still
requires verified PM admission and quiescence recovery for expired leases;
Workflow90's Base step requires trusted owner evidence, while its existing PM
bind expects a running Fleet observation/catalog v2. Resolve the non-circular
predispatch claim/first-step and pre-decomposition assignment contract before
issuing a lease claim or first model POST. See
[current producer limits](GAP_REGISTER.md#exact-producer-predispatch-limits-8-october-2026).

## Pinned Skill Readback Follow-Up

The integration candidate closes physical HOME/skills to the pinned package's
materialized canonical files. Seven-role production Git/materialization tests
and native synthetic support-file readback are recorded in the
[verification ledger](CHAT_CLARIFICATION_VERIFICATION.md#closed-pinned-skill-files-8-october-2026).
No new migration, producer source, accepted runtime or task dispatch is changed.
Next native inventory work must use captured initialized owner state and explicit
incompleteness for plugins/lazy sources, managed/profile settings, transformations
and frozen session instructions; see [runtime limits](RUNTIME.md#native-inventory-limits).
Trusted Tracker assignment/Workflow first-step authority, scoped PM tool delivery/
checkpoint/resume, complete Forge acceptance and the ordered main release remain
separate mandatory gates. Do not bundle the integration migration tail into a
foundation PR or equate this file observation with pre-model admission.

## Signed Activation Reconciliation Follow-Up

The integration candidate implements a closed signed-v3 journal, original
candidate/effective snapshot and launch binding, protected restart loader and
default-off reconciliation in the existing activator. Stopped rollback and
committed-candidate acknowledgement preserve database authority; failed
settlement retains drain/journal and replay uses one digest-only audit. Legacy
v1/v2, rotated secrets, changed snapshots/drain and unknown commands stay held.
See [ADR0035](adr/0035-signed-configuration-recovery.md) and
[verification](CHAT_CLARIFICATION_VERIFICATION.md).

Historical packet5a3b6ccf9139 verifies candidate-running/before-settlement crash
rollback. New packets929400/22a507 accept before-create/lost-preparation-ACK and
fresh candidate-running recovery, respectively, with previous SOUL, unchanged
peer, once-only audit and final namespace exit. Other stop/post-commit/ack crash
points, complete unknown-command recovery, full loaded/readiness and backup-loss
proof remain. Failed preliminary packets stay distinct in the verification ledger.
No installed flag, public API, migration or Java lifecycle
is changed. Keep this component distinct from pre-model Tracker/Workflow
admission, PM tools/answer delivery/checkpoint/rebind, complete Forge receipts and
ordered main release; the full goal is not reduced to this recovery slice.

The parallel Chats/PM consumer is published at `f58db4e` on
`fix/chats-session-recovery-20261007`. Its source/evidence changes relative to
integration `03c26d2` are frontend-only plus task documentation/screenshots.
Reported 446 unit and 96 browser-fixture tests are not live PM acceptance; the
release packet retains its eight main-integration conflicts and prerequisites.
Combine verified sources without rewriting either history, then verify exact
hashes/contract boundaries and repeat combined release gates before readiness.
Normal merge `cb137879` now performs that source combination: backend/OpenAPI
remain byte-identical to the runtime packet and frontend/screens to the consumer
packet. Post-merge hashes and local docs checks pass. Main still has eight
integration conflicts and thirteen ordered migration releases; combined live
gates and the full acceptance scope above are not replaced by source matching.

## Current Run-State Follow-Up

Validated original namespace exit now has an atomic candidate for cancelling
known accepted free-chat runs and pending approvals without granting permission.
Existing durable events drive Chats; replay must preserve the cursor and audit.
Fresh strict/seven-stop/22-migration component gates pass. Published807e1e2
packet5df02ed3fe1b failed on the preparation Compose-exec timeout, before
custody/stop acceptance, and cleaned its owned resources. Bounded read-only
observation retries preserve the original total deadline and live process;
39 host cases pass. Actual packet067e06b0c94a on published2d69908 now passes
custody, recovered-owner stop and generation-bound run/approval cancellation
with preserved transcript and once-only terminal events/audit. Its probe needed
0 retries; it does not establish the cause of the earlier stall. The earlier
namespace-only stop proof deliberately retained session state.
Unknown command outcomes, task/PM terminal authority, interrupted activation,
producer admission and Forge remain separate required work. Keep the full SDLC
goal and ordered release scope; no new migration/SDK/UI change is introduced.

## Publication Readback

The actual two-agent custody gate `2c9e86b1c29a` now passes all six phases and
two physical Fleet restarts with real expiry and unchanged original state.
Keep its scope distinct from resumed execution, interrupted activation,
producer PM compatibility and Forge. Those and the ordered release remain
required; default-off recovery is not enabled by this evidence alone.

The verified native-work candidate uses additive Base `heartbeat_controller_live`
and replaces four separate traversals with two closed live proofs around DB CAS.
Producer/consumer and actual two-restart gate have been verified at the exact
heads documented in the evidence ledger. Preserve the 30-second lease, exact
source hashes and effect fences; no historical-only fallback or in-place
active-controller source upgrade is allowed.

Base runtime-control [PR166](https://github.com/FerrPOINT/services-base/pull/166)
is merged into main as `3a48de8c5696dd20b78c94205d9feb7dbb69c8e0`,
from verified head04f5527 with10 successful CI checks and no review findings.
Post-merge CI run37620081039 also passes all10 checks on exact merge3a48de8.
This publishes the native prerequisite, not an installed SDK pin or permission
for restored execution. The live stop packet retains exact source04f5527;
merging the producer does not silently upgrade an existing launch's source hash.
Fleet's thirteen additive runtime/chat migrations still require ordered release
packets after the accepted foundations; see [migration lineage](MIGRATIONS.md).

Candidate000022 connects explicit recovered-owner namespace stop with a stable
intent, once-only dispatch claim, current DB/native authority and validated
original exit settlement. The initial full captured packet passed582 tests,
strict checks and exact OpenAPI. The follow-up passes strict workspace/all-target
Clippy, six stop cases and22 migration cases without skips, including late exit
without custody renewal and both migration foundations. Actual project9777599d297a
now passes all six custody phases, three restarts and both normal Hermes stops
with independent native/Engine proof at published Fleet93d036b. Live lost-reply
acceptance, session/control/approval reconciliation and interrupted activation
remain required. Do not relabel the earlier custody-only proof or
publish the accumulated thirteen-migration branch as one main PR.

Actual stop packets1f947520ed8f and7dcc9e260024 retain the six custody proofs
but fail the added stop gate; the latter exposes the background health probe
overwriting a confirmed namespace exit. The narrow terminal-proof guard now
passes strict/all-target checks and six-stop/22-migration regression tests.
The complete `--controller-recovery --controller-stop` gate now passes on the
repaired source; hashes and closure scope are in the verification ledger. A lost reply may use the
claimed-command read-only path, never a second kill or a longer lease.

Independent browser boundary [Fleet56](https://github.com/FerrPOINT/fleet-control/pull/56)
is merged into main as940b7de. Retain PR-head test evidence separately from
post-merge CI/deployment. The integrated runtime/PM branch and its migration
release order remain separate work; do not bundle them into this completed fix.

## Current Integration Follow-Up

Chats94e889d and runtime46df7aa are combined by normal merge. Tab reload recovery
and late PM ACK authority checks are no longer missing frontend wiring; retain
their original-key and unknown-outcome holds. The four shell regression fixes
are tested with the actual pinned Base SDK, not a cached sibling SDK checkout.
See [evidence](CHAT_CLARIFICATION_VERIFICATION.md#chats-reload-and-identity-integration-7-october-2026).

Same-SPA identity isolation now has a generation-keyed cache/form boundary,
scoped permissions, original-token request guards and stale SSO checks. The
independent main release is [Fleet56](https://github.com/FerrPOINT/fleet-control/pull/56);
it is merged with all five post-merge main CI jobs successful. Retain the unresolved
original command across reset; implement authoritative PM command discovery
after remount before claiming complete PM recovery.
Keep actual producer/PM/runtime/Forge acceptance and ordered main-release work
below; a source-branch push does not enable or deploy automatic SDLC.

## Connect Fenced Controller Recovery To Native Custody

The000020 storage flow reserves immutable recovery epochs under
the agent-row lock, with exact original hashes/PID, versioned30-second DB-clock
lease, idempotent readback and held unknown acceptance. Any epoch fences old
queue/permit/endpoint/lifecycle effects. Candidate000021 now retains the original
native command, commits one dispatch claim and saves a validated original-key ACK
atomically with owner outcome and audit. Its trusted entry point is now connected
to an explicit default-off startup worker with10-second dual heartbeat and an
8-second initial recovery and20-second heartbeat budgets. Last-version catch-up precedes DB renewal;
native live observation is mandatory before and after extension. Base handover/
private epoch source is published at3facb28; pins and installed runtime remain
unchanged. Complete fresh dual-fence effect admission and interrupted activation settlement before
enabling restored execution.
The actual two-agent gate exposed budget starvation from repeating the initial
handshake before every heartbeat. The follow-up separates initial reconciliation
from renewal and directs acknowledged owners to the full dual-lease heartbeat.
The observed native critical path exceeded the former8-second heartbeat budget;
the new20-second execution budget remains below the unchanged30-second lease.
It retains expiry guards and the once-only dispatch claim. Require live
two-restart/expiry evidence before accepting this change as restored custody.
The subsequent contention fix separates20-second lifecycle-lock admission from
20-second native heartbeat execution in a41-second worker envelope. Revalidate
both leases after lock admission; waiting must not consume native work time or
renew custody. Verify the actual startup worker under contention before accepting
continuous maintenance, not just a saved version4 snapshot.
Prove actual restart, competing owner and interrupted activation; do not rewrite
the original launch or use expiry as a redispatch permit. See
[the exact contract](contracts/CONTROLLER_RECOVERY_V1.md). The fresh562-case
Linux/PostgreSQL source gate passes; see
[evidence](CHAT_CLARIFICATION_VERIFICATION.md#fenced-controller-recovery-storage-7-october-2026).
Candidate000021 historical positive outcome after expiry without renewal passes
the fresh568-case component gate. Next prove actual Fleet/Base/ongoing-Hermes restart, next owner
against physical cessation, and lost native ACK across OS-process crashes.

## After Integrating The Original-Key Chats Consumer

Normal merge49c11f5 combines the independently published Chats16b7516 and
runtime9a11bde without modifying backend bytes. Original-key recovery is now
production frontend behavior, not a remaining adapter wiring task. See
[combined verification](CHAT_CLARIFICATION_VERIFICATION.md#integrated-chats-and-runtime-candidate-7-october-2026).

The separate Chats task next proves browser reload/logout/identity-change
recovery using only opaque command identity and server-authorized GET. It must
not store raw guidance, transcript or credentials, release uncertainty from a
local cache, or duplicate a POST. Resolve the pinned SDK stream-denial finding
separately; successful component/fixture rechecks do not close it.

Runtime work continues below: durable fenced controller transfer and interrupted
activation, original-generation bounded/redacted collection, exact producer
admission/first-step authority, PM checkpoint/delivery and actual Forge/SDLC
acceptance. Keep the compatible published release lineage and one-migration
boundary; no installed opt-in follows from merging source or fixture screenshots.

## Controller Ownership Recovery After Read-Only Witness

The Base same-container restart observer is published at
[9171fe6](https://github.com/FerrPOINT/services-base/commit/9171fe6b1b6b05b9504d33fb881f274c0b5541e0).
Actual native proof retains the original running synthetic agent and byte-exact
journals with lifecycle effects denied. Fleet's typed consumer verifies original
mapping/registration plus fresh immutable DB launch/ACK/PID for degraded health;
this is not adoption. See
[evidence](CHAT_CLARIFICATION_VERIFICATION.md#original-controller-restart-observation-7-october-2026).

Complete recovery with a durable CAS/lease owner record and immutable controller
epoch history, without rewriting original launch identity. Only exact retained
private source/config/launch witnesses and confirmed old controller cessation
may authorize the new owner. Reconcile activation checkpoints and unknown native
commands before resuming; a lost journal, competing owner, new container ID or
source-policy upgrade must remain held until its explicit recovery rule is proven.
Validate real Fleet/Base/Docker recovery and ongoing run before installed opt-in.
Keep release migrations isolated and ordered. Base PR150 remains a prerequisite
and currently conflicts with main; do not fold its unrelated history into a new
release PR or change the installed SDK pin from these utility tests.

## Public Control Recovery Consumer Handoff

The verified-human original-key GET is implemented and covered by the fresh
551-case full gate. The local client is generated from Rust OpenAPI. See
[consumer contract](contracts/HERMES_RUN_CONTROL_V1.md#fleet-command-journal)
and [verification](CHAT_CLARIFICATION_VERIFICATION.md#original-key-control-lookup-7-october-2026).
The published Chats16b7516 consumer, now included in49c11f5, retains original
key/run/input, hashes the same Rust-normalized semantic payload and releases a pending hold only
from a fresh exact accepted receipt. Unknown,404 or terminal-without-ACK must not
become a second POST. Keep actual native reply-loss/restart acceptance separate
from these component tests; no installed runtime flag changes accompany this GET.

## Verified Native Credential And Input Slice

The fresh actual two-agent Rust/Base/Docker gate verifies six native model runs,
static provider rotation after drain, peer/restart isolation and real readiness
rollback. Six retained dotenv creation intents match original PostgreSQL fences.
This advances launch/input acceptance but does not enable a collector or replace
the coherent production flow below. All effective secret sources/reloads, bounded
retention, atomic private checkpoint/range commits, controller recovery and
predispatch/PM/Forge acceptance remain mandatory. See
[native evidence](CHAT_CLARIFICATION_VERIFICATION.md#native-provider-rotation-and-original-input-custody-7-october-2026).
No migration, API, SDK, UI or installed runtime change accompanies this test packet.

## Next Runtime Work After Private Log Transport

Base source [d0eedc1](https://github.com/FerrPOINT/services-base/commit/d0eedc16336386ca1a8827387d806d9c9a89b919)
now implements closed private `log_page`: per-stream byte offset and verified
SHA256 prefix,16 KiB pages, bounded64 MiB scan and double original readback.
All137 Linux runtime cases pass without skips; native Docker CLI reads all5000
synthetic records before/after stop and rejects a real rotating source. This is
source/transport evidence, not a Fleet collector or release. See
[evidence](CHAT_CLARIFICATION_VERIFICATION.md#verified-base-log-source-pages-7-october-2026).
The utility still depends on open Base PR150; SDK pin remainscbb4e99.

Fleet's private Rust client now decodes the same closed `log_page` envelope,
validates exact requested range/original receipt and retains raw binary bytes
without Debug/public serialization. This establishes the transport consumer,
not a DB collector or actual Rust-to-Base/Docker ingestion acceptance. The
generation-bound production flow below remains mandatory; do not wire pages
straight into generic `insert_log` or advance cursors in memory only.

Implement the remaining collector as one coherent generation-bound flow:

The original-input prerequisite now freezes guarded dotenv bytes/hash in each
new automatic private creation intent before create. Revision-bound launches
match the rendered file and retry/start checks reject drift. Historical intents
are not backfilled. This is not completion of item1: the pinned native loader
interpolates/sanitizes credentials and can apply external/managed sources. Prove
and freeze its actual effective values without storing delegated child credentials
or relying on the current environment; the collector remains disabled.

1. Freeze actual resolved launch credentials in private controller storage before
   execution; never reconstruct old secrets from the current rotated environment.
2. Validate original launch/controller/registration and retain partial line/UTF8/
   secret chunks privately. A staged raw checkpoint must be durable before the DB
   transaction references it; it must not become a public log or plaintext DB field.
3. Atomically commit redacted source ranges, durable cursor/version and delivery
   events under the original launch fence. Identical content is not dedup identity.
   A failed/unknown commit reconciles the original range; it does not skip forward.
4. Restore the exact referenced private checkpoint after crash. Preserve the old
   checkpoint until the new DB commit is confirmed; missing/tampered state holds
   collection instead of adopting a new generation or discarding backlog.
5. Define bounded disk/retention supervision before enabling this append-only
   source. The utility does not change runtime logging settings; overflow/rotation
   or unsupported effective driver is a hold, never an empty successful poll.
6. Prove actual Rust -> Base -> PostgreSQL ingestion, secret rotation/split handling,
   duplicate replay/crash recovery and authorized API/SSE. Keep migration releases
   in order; this transport packet adds no migration or public API.

The shared repository log-acknowledgement race is fixed and verified on both
main-based and runtime source. See
[the PostgreSQL evidence](CHAT_CLARIFICATION_VERIFICATION.md#atomic-process-log-acknowledgement-7-october-2026).
Do not implement collection by repeatedly inserting the current200-line tail:
timestamps/content hashes are not reliable source offsets, and identical lines
can be distinct real records. Freeze the original resolved per-launch credentials
before rotation, and atomically commit a verified source range with its cursor.
Missing prefix, overflow or rotation needs explicit gap/reconciliation semantics,
not silent deduplication or a successful empty poll.

Private Base Docker log readback now passes on four original exited generations
of two actual Hermes agents. Rust's typed client is implemented and passes the
fresh528-case Linux gate, but this is not production ingestion. Connect it only
with current launch custody, exact resolved per-launch secret redaction and
generation-bound durable cursor/deduplication/overflow/rotation semantics;
then prove actual Rust-to-Base ingestion and authorized API/stream output.
Complete interrupted activation/controller recovery and the unchanged
predispatch/first-step/PM/Forge acceptance next. Do not invent missing producer
authority or use a healthy runtime as SDLC readiness.

Keep Base control source69831aa separate from SDK pin cbb4e99. Its prerequisite
PR150 is ready/MERGEABLE at7d7323a with9 successful exact-head CI jobs, but still
open and not installed. Retain ordered release rather than
publishing a predecessor-heavy log-only main PR. Fleet's historical migrations
still require the agreed one-migration-per-release-PR order. Nothing is installed
or enabled from this packet. See
[verification](CHAT_CLARIFICATION_VERIFICATION.md#actual-private-base-docker-log-readback-6-october-2026).

## Current Actual Docker Proof

Real original-Engine/UID 999 Rust Fleet/Hermes/chat/config acceptance now passes
for two mapped-volume agents and five controlled-model runs. Additive `000019`
binds the actual Base endpoint to original launch/PID and removes the old
localhost-only container-dispatch assumption without an arbitrary endpoint setter.
Fresh broad/migration gates pass526 workspace cases (30 explicit opt-in ignores),
fmt/check/strict Clippy and19 migration cases without ignores. Publish this scoped
candidate and preserve the ordered one-migration-per-release-PR boundary, then
prove actual Docker readiness rollback, controller recovery/private-journal loss,
logs and the unchanged producer/PM/Forge requirements. No installed opt-in or
"full merge-ready" claim follows from this one gate. See
[verification](CHAT_CLARIFICATION_VERIFICATION.md#actual-docker-supervisor-chat-and-configuration-6-october-2026).

## Current SDLC Scope

The current focused-tested candidate implements container configuration replacement
through original namespace stop and fresh activation/rollback generations.
Eight focused regressions cover exact pre-create admission, stopped apply,
pending custody holds, normal replacement, readiness rollback and unknown or
foreign effects.31 focused cases and strict Clippy pass. The full exact-source
Linux gate passes523 cases with29 explicit ignores and fmt/check/Clippy.
Complete real mapped Hermes/model acceptance before claiming release readiness.
This is not controller takeover
or completed PM/Forge/SDLC acceptance.

The current work adds additive000018 pre-create DB authority: immutable
agent/history ordinal/controller/generation/operation/intent hash before both
private intent creation and Docker prepare. Exact replay, missing-file holds,
competing-controller fencing and blocked native fallback have new regressions;
fresh full/migration gates pass515 workspace cases (29 explicit ignores),
fmt/check/strict Clippy and19 migration cases without ignores on ten disposable
databases. Both gates capture the same290 unchanged backend/SDK files and owned
resources are cleaned. See [exact evidence](CHAT_CLARIFICATION_VERIFICATION.md#durable-container-pre-create-fence-6-october-2026).
This is not actual
mapped Hermes/model acceptance or complete private Base journal recovery.
Base PR150's main conflict was normally merged into424ad76 without rewriting
history. All9 exact-head CI37480593149 jobs pass; the candidate is ready/mergeable
after a fresh post-ready check, with no reviews/threads. Utility bytes are
unchanged. Earlier98a5bbd readiness below is historical; installed state and
producer permissions remain unchanged.

The current candidate additionally merges consumer `d592a0d` (HTTP408 remains
unknown). Fresh262 frontend tests and33 fixture browser cases pass, with
regenerated21-image preview evidence. All288 backend/SDK bytes match the
previous successful workspace/migration gates; no backend rerun is claimed.
The live acceptance requirements remain unchanged.

Chats through `0ecee7e` and runtime parent `7564e2e` are normally merged in the
integration candidate. Combined-tree gates pass511 workspace tests with29
explicit ignores,18 fully configured migration tests,259 frontend tests and33
fixture browser cases;21 preview screenshots are regenerated. See the
[combined evidence](CHAT_CLARIFICATION_VERIFICATION.md#combined-chats-and-runtime-candidate-6-october-2026).
This is not main/installed acceptance; later parallel Chats changes need another
scoped integration. The real runtime/producer/PM/Forge requirements below remain.

Fleet source `d924799be5ec77935b71decce20059f22b919e10` now consumes private
protocol2/boundary policy3 named-volume subpaths. Original mapping/file/hash are
bound in creation intent, prepared receipt, registration and DB launch provenance.
Fresh guards preserve local AgentPaths/marker ownership; protocol downgrade,
changed controller/proof, sibling mounts and unknown-start resend are denied.
The fresh Rust1.88/Linux/PG gate passes509 tests with29 explicit ignores and
fmt/check/strict Clippy; see [verification](CHAT_CLARIFICATION_VERIFICATION.md).
No SDK/public API/migration/deployment flag changes or installed rollout.

Next prove actual Rust Fleet -> named-volume Hermes -> model/chat with the real
controller UID, then safe new-generation config drain/activation/rollback, logs,
controller restart reconciliation and the remaining producer admission/PM/Forge
gates. Private-controller document/journal loss and restore must also be tested
before rollout; preserved-file replay is not proof of storage-loss recovery.
Base PR150 remains ready/mergeable on98a5bbd with9 SUCCESS jobs CI37465043730;
it is not merged/installed. The full SDLC goal remains open.

## Historical Runtime Gates

Base follow-up2bcf3d2 now proves read-only named-volume mapping with real UID999
controller files, original snapshot/Engine/volume hashes and drift rejection.
This is not yet a Fleet consumer: persist the original mapping in creation intent,
revalidate before all effects and extend Base guarded lifecycle validation to
use the corresponding local paths. Do not merely replace policy sources and
bypass existing path guards. CI37461023808 passes all nine jobs for this new
ready/mergeable candidate, reread after ready with no reviews/threads;
the ready368cfb8 observation below is historical. See current verification.

The current published runtime integration is cf4b08e, including trusted Fleet
bridge attachment. Its full Rust1.88/Linux/PG gate passes504 tests with29 ignored;
the earlier503/502 gates below are historical. The attachment fixture is not
actual Rust Fleet/container/model/chat acceptance.

Read-only installed-runtime inspection on6 October confirms a remaining blocker:
Fleet agents_root is `/var/lib/fleet-control/agents`, whereas the Docker Engine
bind source is `/var/lib/docker/volumes/sdlc1_fleet_agents/_data` (independently
`sdlc2_fleet_agents/_data` for workspace2). Both Fleet backends run UID/GID999;
the previous Base Hermes startup QA ran UID/GID10001. Do not pass the controller
path to the daemon, guess another volume, change protected ownership or accept
the previous host-bind QA as volume/permission proof. The next runtime packet
must seal the exact controller/Engine mount mapping, preserve local path/marker
guards and prove write/read access for the explicitly selected runtime UID.
See [current verification](CHAT_CLARIFICATION_VERIFICATION.md).

Base PR150 has been normally merged with accepted main475c694 without force push;
candidate368cfb8 is ready/mergeable with nine successful exact-head CI37458583952
jobs, reread after ready. Local377 Python cases contain366 PASS/11 skips;
26 Node runtime-contract tests and README/hub pass.
Utility executable bytes are unchanged by this history reconciliation.

Prepared-generation recipe checking is now implemented: original process/policy,
derived runtime API token and registration are revalidated before claim/start.
The new seven-case regression and fresh503-test Rust1.88/PG workspace gate pass
with29 ignored, fmt/check/strict Clippy and exact cleanup. The502-case source-loader
gate below is retained as prior evidence. Current fixture-backed verification does
not replace the next actual Fleet/container model/chat and config lifecycle gates.

The captured-source Base utility loader follow-up closes the file-hash/import
gap: fixed isolated Python compiles the captured pinned bytes, not cached code
or reread checkout files. Host behavioral coverage is available and added to CI;
all four Linux Rust regressions and the final502-case workspace gate pass.
This cannot replace controller attachment, filesystem mapping or actual Hermes
container/PM acceptance. Previous own QA cleanup is now verified on the original
Engine; the final full gate exits0 with exact own cleanup and288 matching inputs.

Docker consumer is now connected in the integration candidate: immutable
container registration/paths/config/context/source pins precede Base start;
start/health/stop and every Hermes endpoint use original container receipts.
Unknown start stays claimed and cannot resend or fall back to native Hermes.
Automatic first-generation preparation now saves a private original intent
before Base render/create/register and writes its matching prepared document.
Operator-prepared generations remain a compatibility path. Trusted controller
bridge attachment is now implemented; next implement daemon source mapping, UID/file access,
logs and namespace-based config rollback. The follow-up history-ordinal restart
candidate now preserves previous files and creates a new generation only after
confirmed original exit; its new full Rust/PG gate was interrupted by host disk
exhaustion/Docker unavailability. Own project3b96063dfdea is now verified clean;
the final502-case gate passes without relabelling the partial174-unit pass. Real Docker
restart/Fleet/Hermes acceptance remains required.
Do not enable installed Docker mode or label automatic provisioning complete.
Task admission, producer first step, PM tools/resume and real Forge/seven-agent
acceptance remain separate required gates.

The full fresh Rust1.88/PG candidate gate passes494 tests with29 explicitly
ignored profiles, all-target check/strict Clippy and fmt. See
[verification and retained failure](CHAT_CLARIFICATION_VERIFICATION.md).
This gate includes the initial preparation consumer and fake-Base PG fixtures,
not actual container/Hermes or full PM acceptance. Do not relabel the
ignored/live gates as complete. The prior490-case gate remains historical.

Historically, Base PR150 at6041530 was ready/mergeable with nine successful exact-head jobs in
CI37449729086; checks were reread after ready and reviews/threads are empty.
PR144 is merged at main63fff28. This certifies the shared utility, not installed
Fleet/Hermes acceptance. Native real-Hermes startup/stop and synthetic creation
controller exits/v1 regression are recorded separately from Rust fixtures.

## Historical Native Packet

The earlier Base release was d2c8ef6, preserving accepted maina3d6a79 with unchanged
boundary/bootstrap bytes and nine green jobs in CI37409952184. The a0f7044
observation below is historical. Fleet2293862 is published integration source;
its229 component/38 harness cases and fresh native lifecycle are scoped local
evidence, not readiness for PM, host-boundary consumption or automatic SDLC.

The reconciled foundational release PR47 now has five green exact-head CI jobs
at `5f20540`, but stays Draft for missing live PM acceptance. Base PR144 has nine
green jobs at `a0f7044`; its host primitive still is not a Fleet consumer.
The separate integration follow-up rejects dead retained-child dispatch and
foreign-controller runtime overwrites/false HTTP health-transition alerts.
Its fresh compilation, component and native evidence must be accepted separately;
neither foundational CI nor fixture screenshots certify the later runtime tail.

The pre-spawn native launch journal `000017` binds agent/config/controller before
execution and holds unknown outcomes across Fleet replicas. Component validation
is recorded separately; this does not complete the host-boundary integration,
loaded config, safe descendants or task admission. Release after `000016` as its
own one-migration packet, never by rewriting applied migration history.

The packet also pins native launch identity in private dispatch intents and
checks it before preparation, permit consumption and actual HTTP submission.
An old or legacy intent cannot follow a replacement managed gateway. This is
not loaded-generation attestation or full task admission.

Source reconciliation with accepted main `3c6b8ef` now preserves canonical and
legacy split foundations, verified central names and the accepted Base SDK.
The actual PostgreSQL lineage suite verifies both populated histories through
the shared eight-step tail (18 canonical/21 split entries), not just a clean
installation. Historical migration bytes are unchanged. Ordered single-migration
release packets, exact-head CI and review are still prerequisites. Keep PR47
and dependency packets unchanged until their owners perform that process.

The historical phases below are not completion evidence for the October SDLC plan.
Current implementation, boundaries, blockers and acceptance are maintained in
[SDLC_IMPLEMENTATION.md](SDLC_IMPLEMENTATION.md). Leaders are deferred. Java lifecycle
exists, but runtime chat/control still returns phase-2 errors and cannot run SDLC.

Original approval journal000016 and its opt-in consumer are implemented.
Original-mode HTTP reservation, exact-byte approval POST and bounded GET-only
recovery require their recorded component gates. Native positive/unknown decision
outcomes, separate Fleet SIGKILL recovery and combined run/command plugins are
verified in disposable native QA. Installed rollout and complete SDLC remain.
Journal component
checks do not close those gates or enable installed runtime flags. Ordered
one-migration release packets and exact-head CI/reviews still precede rollout.

Phase 0: pre-development hardening.

- RBAC and permissions endpoint. — done: `SystemRole = admin|operator|user` c бэкенд-энфорсментом (middleware + `GET /users/{id}/permissions`, `PATCH /users/{id}/role`), legacy `is_system_admin` alias; см. docs/AUTHORIZATION.md.
- Idempotent sessions/messages. — done: миграция 0004 (`idempotency_key` + `idempotency_payload_hash`, unique `(user_id, idempotency_key)`), replay возвращает исходную сессию/сообщение.
- Session participants, leader selection, handoff and delegation. — done: `leader_agent_id`, роли `leader|executor`, `/sessions/{id}/participants`, handoff/delegation-роуты (docs/API.md §sessions).
- Deployment jobs and settings surfaces. — done: `deployment_jobs` + bulk `POST /deployments/jobs/bulk`, settings API с per-key аудитом (docs/API.md).
- Product pages for leaders and executors. — done: `frontend/src/pages/{leaders,executors}` + карточки агентов/сессий.
- Technical pages for agents, deployments, logs and settings. — done: `frontend/src/pages/{agents,deployments,logs,settings,alerts}`.
- Screenshot manifest and evidence capture. — done: 82 desktop-файла в `docs/assets/screens/` + manifest.md (`1920x1080` и `2560x1440`).
- Documentation and ADR alignment. — done: полный док-паритет с task-tracker (55 файлов), ADR + ADR_INDEX синхронизированы.

Phase 1: Hermes MVP completion.

- Finish real Hermes API session open/send/stream integration. — done: Hermes `/v1/runs` адаптер (open/send/SSE mirroring/stop/steer/approval forwarding) — см. CURRENT_STATE.md.
- Expand fake Hermes lifecycle tests into real adapter contract tests. — done: контрактные тесты адаптера на фикстурах `backend/tests/fixtures` (Run-переходы, SSE, approvals).
- Add runtime reconciler tests for desired-state restart. — done: `reconcile_action(status, desired)` решает Restart/HealthCheck/None (unit-тесты переходов), reconciler-цикл перезапускает failed/stopped агентов с desired=running и health-checkает running.
- Add clean DB migration and seed workflows. — done: SeaORM-миграции 0001+ (idempotent IF NOT EXISTS), seed в тестах через фикстуры.
- Replace the local HMAC token validator with `sdlc-auth-core::Validator::hmac`
  after WSL/CI can fetch `services-base`. — done: `AuthService` валидирует через `sdlc_auth_core::Validator` (hmac-mode), локальный дубликат decode-логики удалён; см. docs/AUTHORIZATION.md.
- Add OIDC/JWKS validation mode and retire the compact-token legacy fallback
  after the transition window. — done: `auth.mode=oidc` — RS256/JWKS (кэш+refresh, kid-miss), строгие iss/aud, маппинг ролей, local login и HMAC-токены отклоняются fail-closed (см. docs/ENV.md, docs/OPERATIONS.md).

Phase 2: Java Agent runtime.

- Implement Spring Boot launch/provision adapter. — done: supervisor поднимает `java -jar <agents_root>/agentN/runtime/backend.jar --spring.profiles.active=noop`, readiness по `/actuator/health/readiness` (db-only).
- Wire health, capabilities, sessions and chat stream. — partial: health/readiness and lifecycle exist; chat/control remain phase 2 in runtime/mod.rs.
- Add Java Agent runtime tests and screenshots. — done: runtime-тесты provision/launch/readiness + скрин-свидетельства java-agent-страниц в evidence-сете.

Phase 3: fleet operations.

- Add operator retention policy thresholds and scheduled stale-folder review. — done: `fleet.retention.stale_archived_days` / `fleet.retention.review_interval_secs`, fleet-wide `GET /api/v1/agents/storage-review`, stale flag + archived days in storage report, scheduled review worker, `POST /api/v1/settings/retention/review`.
- Add richer monitoring and alerts. — done: `fleet_alerts` (миграция 6): авто-алерты переходов здоровья (agent_down/agent_recovered), `GET /fleet-alerts`, acknowledge (Operator+), авто-закрытие открытых и подтверждённых алертов после восстановления, аудит; событийная модель включает restart-loop/heartbeat-stale.
- Add bulk runtime updates and rollback. — done: `POST /api/v1/deployments/jobs/bulk` (см. docs/API.md); rollback помечает runtime_update jobs через `detail.rollback`.
- Add cross-project workflow health integration. — done: `refresh_workflow_bindings` сверяет биндинги с живым project-workflow каталогом (reconciler, runtime/mod.rs:344), `binding_status` в UI/API.
