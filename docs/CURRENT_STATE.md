# Current State

Status (2026-10-05): SDLC foundation is partially implemented. Automatic SDLC is
blocked until cross-service assignment/workflow/deployment contracts are verified.
See [SDLC implementation](SDLC_IMPLEMENTATION.md). The baseline feature/gate lists
below are historical, not acceptance evidence for the new SDLC plan.

Base now has an opt-in native control-outcome producer with durable single-send
reservations and GET-only exact ACK lookup. Two real API/AIAgent cases verify
lost steer/interrupt replies, gateway restart and unknown-command hold;21 Linux
component cases pass. A strict Rust GET wire-consumer now validates saved
epoch/scope/origin/body identity and exact action ACKs, but is not yet wired
into the supervisor or command journal. Production Fleet does not yet persist/send its epoch/raw
command context or reconcile these receipts. Keep this plugin disabled on
installed agents. See [outcome contract](contracts/HERMES_CONTROL_OUTCOME_V1.md)
and the verification ledger; producer-only evidence is not consumer acceptance.

Managed configuration apply/rollback now persist Linux directory entries after
rename/unlink and new ancestors before acknowledging an effective revision.
Injected persistence failure retains activation journal/drain without spawning
a runtime. The current candidate passes397 Linux/PG component cases, strict
all-target checks, migration CLI and OpenAPI equality;235 frontend cases and
five actual Hermes scenarios pass. Native source/test/harness fingerprints match
the current bytes. Windows durability, physical power loss, loaded generation
and safe descendants are not certified. Exact remote Tracker schemas and
Workflow predispatch authority remain incompatible/unavailable; local snapshot
parity is not producer release acceptance. See
[persistence evidence](CHAT_CLARIFICATION_VERIFICATION.md#linux-managed-configuration-persistence-5-october-2026).

Current pending approval recovery is implemented for accepted pinned free chats.
Actual Hermes acceptance uses two distinct Fleet processes and one surviving
gateway: GET-only request restore, exact owner decision, one tool effect and one
final answer. This does not recover historical questions or unknown decision
outcomes. See [current evidence](CHAT_CLARIFICATION_VERIFICATION.md#current-approval-snapshot-recovery).

Additive000014 now protects logical journal submission/ACK order against observed
clock regression without renewing deadline/key/permit. The deterministic test
fails on the old schema and passes on the new; isolated upgrade/down/reapply
preserves history/original guard. The preceding candidate passed389 Linux/PG cases,
strict all-target Clippy/check/fmt, OpenAPI equality and clean migration CLI;
Node22 frontend gates pass235 tests and build. Clock integrity,
ordered release and exact-head CI remain prerequisites. See
[clock evidence](CHAT_CLARIFICATION_VERIFICATION.md#journal-clock-order-repair).

Targeted approval now requires original accepted free-chat journal context,
fresh native capabilities and the pinned currently waiting exact request. Legacy
or unadmitted task context cannot authorize the POST; bounded exact HTTP200 JSON
ACKs are mandatory. Steer/stop share that context guard. Separate actual native
approval and control runs pass with real Hermes and a loopback model. A PostgreSQL
delivery lock-order regression first failed and then passed after session-first
locking; unknown acceptance remains pending and is never redispatched. See
[current scope/evidence](CHAT_CLARIFICATION_VERIFICATION.md#exact-approval-context-and-delivery-lock-order-5-october2026).
Loaded config generation, task/PM admission, central identity, unknown-decision
lookup and safe OS descendants are not certified by these checks.
The preceding approval/lock-order packet passed378 Linux/PG component cases, strict
all-target checks/Clippy/fmt, OpenAPI equality and migration CLI. VM-clock jumps
and incomplete runner audit remain recorded; local QA is not release-head CI.

Hermes free-chat steer/stop now require original accepted journal context, fresh
capabilities and pinned native GET before a bounded exact ACK. Guidance does not
reset concurrent run state; interrupt only requests stopping. Legacy run-wide
approval is denied at the adapter too. Linux/PG358 component cases and a separate
real AIAgent control case pass; see the
[control profile](contracts/HERMES_RUN_CONTROL_V1.md). The new migration 000013
adds durable authenticated actor/key command receipts, a single-send claim, scoped history,
atomic ACK/audit/events and terminal-only reconciliation of unknown outcomes.
Production chat/legacy controls preserve input and keys, hold unknown effects
after reload and distinguish ACK from terminal state. New component/native/UI
evidence is recorded separately in the verification ledger; the broad backend
component gate now passes367 distinct Linux/PG tests, all-target check, strict
workspace Clippy, fmt and OpenAPI equality. Previous failed attempts and ongoing
VM clock-regression observations remain recorded, not reclassified as successes.
Task admission, historical approval replay and installed approval acceptance,
safe OS descendant stop, release partition and exact-head CI remain requirements.

Human free-chat stop/steer now require `VerifiedHumanSession` before lookup,
matching targeted approvals. Actual local JWT middleware/HTTP tests verify owner
commands, replay, foreign-session/run denials, operator/admin receipt reads and
active-user revocation. A separate sessionless-principal test denies even an
admin and a forged human header. The renewed Linux/PG gate passes372 distinct
component cases plus migration CLI and strict all-target checks. This is not
live central JWKS, scoped machine/task admission or a release-head CI result;
see [exact evidence](CHAT_CLARIFICATION_VERIFICATION.md#human-runtime-control-http-boundary-5-october-2026).

Native targeted approvals now pass a separate opt-in owned Linux test against
real Hermes terminal guards, approval requests and tool execution. Three chats
verify owner once/deny, one POST per decision, immutable transcript on replay,
foreign-user denial and a dropped real ACK retained as uncertain after terminal.
The model is a loopback fixture; HTTP uses actual local JWT middleware. This
does not certify central identity, task/PM admission, waiting-approval crash
recovery, native outcome lookup, installed agents or safe descendants. The
21 host harness tests and exact binary/source/log evidence are recorded in
[native approval evidence](CHAT_CLARIFICATION_VERIFICATION.md#managed-native-exact-action-approvals-5-october2026).

The native Hermes stream consumer now has bounded incremental byte framing,
strict JSON and original run identity, fixed assembly/idle/lifetime deadlines,
traffic/text/snapshot budgets and no incomplete EOF dispatch. Empty transport
chunks cannot extend idle time. Consumer failure preserves accepted identity
and capacity for independent GET-only recovery; it is not safe stop, upstream
tool/approval replay or a stage receipt. The
[consumer profile](contracts/HERMES_EVENT_STREAM_V1.md) and verification ledger
separate source/component/native evidence from release and remaining gates.
The final source passes348 distinct Linux/PG component cases and one separately
executed managed native compatibility case. Exact source/log/binary hashes,
preliminary failures and remaining acceptance scope are recorded in the
[bounded stream evidence](CHAT_CLARIFICATION_VERIFICATION.md#bounded-native-stream-consumer-4-october-2026).

The managed native supervisor happy path now passes one explicit opt-in Linux
test: two real Hermes gateway CLI/API/AIAgent processes, actual Fleet activation
and prompt outbox, a deterministic loopback model, private per-agent dotenv,
distinct loaded SOUL and restart-preserved run/transcript identity. Every13770
tracked source file matches the pinned archive; temporary QA source image and
containers are cleaned up. Two host-bind readiness failures remain failed
evidence, not PASS. No production readiness deadline or installed image changed.
Native waiting-tool/approval crash recovery, complete config/plugin inventory,
descendant quiescence, central auth/UI and task/PM admission still remain gates.
See [scope and exact evidence](CHAT_CLARIFICATION_VERIFICATION.md#managed-native-supervisor-4-october-2026).

Managed lost-ACK recovery now passes a separate opt-in native case. A QA-only
platform plugin discards the real accepted response after Hermes reservation;
the first Fleet subprocess exits with a submitted journal and unknown run ID.
A different Fleet subprocess restores the original already-terminal run through
the exact committed Base witness plugin and authenticated GET, without a second
POST/inference or native SSE consumer. The original request/context/horizon remain
unchanged and one assistant mirror is stored. This is disposable source QA, not
installed rollout, native crash/running recovery, safe orphan stop or task/PM
admission. See [exact evidence and boundaries](CHAT_CLARIFICATION_VERIFICATION.md#managed-native-lost-ack-recovery-4-october-2026).

New implementation: seven specializations, agent-grouped `/chats`, read-only
directory, persistent bearer SSE replay, transactional prompt outbox, concurrent
idempotency, no effective-admin central bypass, versioned configuration activation
with drain/readback/rollback and fail-closed SDLC readiness.

The B-SDLC-02 source slice prepares a pinned Base package draft through the
operator API without changing effective runtime files. Git schema/hash/inventory,
snapshot proof/content, concurrent desired/identity and drain guards are verified
by scoped tests. Pinned effective readback additionally rechecks Git provenance
and a bounded closed HOME skill-file inventory; an opt-in agent-scoped Base PAT
read exposes metadata only, never dispatch rights. Runtime/plugin/external
inventory, loaded settings and general assignment dispatch remain blockers.
Details: [scope and evidence](SDLC_IMPLEMENTATION.md#b-sdlc-02-подготовка-закреплённого-base-draft).

The configuration-observation follow-up passed 28 distinct scoped Rust 1.88 cases
(31 executions, including repeated inventory checks):
11 pinned-package/filesystem unit, 5 PostgreSQL/HTTP, 3 effective-file regression,
1 drain/rollback, 2 config compatibility and 6 machine-auth/OpenAPI cases.
Scoped infra/API/integration clippy and fmt passed; regenerated OpenAPI/client,
frontend typecheck/drift and documentation links passed. This is not a full
integrated gate or installed-runtime acceptance. Native admission/dispatch remain
blocked, and no accepted images, volumes or skills pins were changed.

Native Windows Rust commands still require MSVC `link.exe`.

The 2026-10-02 transcript follow-up adds immutable internal database allocation
order to legacy listing and paginated chat history. New messages do not reorder
when the clock moves backwards; overlapping pages preserve server order. Historical
records retain their former timestamp/UUID order, not recovered insertion order.
SSE reconnect no longer cancels an in-flight older-history fetch. A catch-up read
includes messages arriving during that fetch. This does not complete PM creation,
delivery, resume or live Backlog acceptance.

## PM Clarification Work In Progress

The credential confinement follow-up limits the server-only delegated client to
enumerated PM GET/POST operations on its canonical assigned task at the configured
Tracker origin. Base's five-field delegation wire is unchanged. Tracker separately
denies direct PM bearer use on legacy/global/owner/verifier operations and checks
current assignment authority. Client allowlisting is not server authorization or
runtime admission. Opt-in creation now persists original command/parent/origin
intent before Base POST and immutable ACK metadata before fresh child/context
readback. Audit and journal updates are atomic; replay cannot rotate identity or
renew an expired child. The 33-case Linux/PostgreSQL component gate passed.
A separate actual Base/Tracker issuer-to-context test now verifies exact replay,
scoped denials and parent revocation with distinct central/local IDs. The complete
persisted creation saga, renewal administration, admission and runtime tool
handoff remain open. See the
[current credential evidence](CHAT_CLARIFICATION_VERIFICATION.md#persisted-credential-preparation-4-october-2026).
The broader Linux regression passed 174 workspace library and 70 foundation
PostgreSQL/HTTP tests, all-target check/strict Clippy and formatting. Node22
frontend passed 229 tests, lint and format; fixture screenshot manifests verify.
These checks do not include the opt-in real-producer credential run, all other
integration targets, release build or end-to-end runtime acceptance.

Free-chat verified acceptance now commits run/prompt/outbox atomically before
status GET. Accepted-but-unpinned pending runs recover effective session identity
by GET only after restart, with one concurrent stream-start winner and immutable
runtime mapping. The subsequent atomic-terminal follow-up below handles pinned
restart recovery; opt-in unknown-ID recovery is described below. Assignment
admission and native configuration attestation remain open. See
[ADR 0015](adr/0015-accepted-run-session-readback.md) and the verification ledger.

The atomic-terminal follow-up now commits optional assistant, prompt delivery,
run outcome and durable events together. Accepted pinned free-chat runs recover
after restart by authenticated original-context GET, with no second POST/SSE
worker. Late delta/tool/approval writes are fenced by the committed terminal
state; empty output creates no fabricated reply. Its Linux/Rust1.88/PostgreSQL17.6
gate passed 192 library +123 foundation +1 approval SSE +3 migration cases
(319 distinct tests), all-target check/strict Clippy/fmt and Rust OpenAPI equality.
Independent review closed the FK lock inversion and fixture setup races.
No migration, public API, installed runtime or producer changed. See
[ADR 0018](adr/0018-atomic-terminal-pinned-recovery.md) and
[current evidence](CHAT_CLARIFICATION_VERIFICATION.md#atomic-terminal-and-pinned-recovery-4-october-2026).
Installed unknown-ID/store continuity acceptance, missed native tool/approval replay, stream frame/
multibyte bounds, OS isolation, admission/PM resume and exact-head release CI
remain open. These component results are not full SDLC acceptance.

The final acceptance-readback source gate passed on Linux/Rust 1.88/PostgreSQL
17.6: all 176 workspace library tests, all 85 foundation PG/HTTP cases and the
separate authenticated approval SSE test (262 distinct cases). All-target check,
strict all-target Clippy, formatting and regenerated OpenAPI equality passed.
The 98-file Markdown link check and README validation passed. These include a
real post-commit database permission fault, concurrent ACK/pin/delivery races,
GET outage/foreign identity/restart recovery, a 22-ACK keyset and stale control
snapshot denial. The final Compose project was removed with caches preserved.
Hermes HTTP producers here are controlled fixtures, not authentic gateway/model
acceptance; this gate does not prove unknown POST recovery, PM admission/resume,
process-tree quiescence, release build, remaining integration targets or UI/live
screenshots. Accepted-but-unpinned controls remain temporarily unavailable.

The separate native protocol gate now passed against clean pinned Hermes
`bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3`: four actual API/AIAgent/SQLite
scenarios with a deterministic local model, plus eight host harness safety
tests. It proves dropped-202 original-key replay, eight concurrent replays,
terminal and in-flight process-crash recovery without reexecution, exact session/
native transcript, parsed SSE success flags and token isolation across two homes.
Credential rotation demonstrably creates a different idempotency scope; Fleet
must preserve and verify the original scope before enabling recovery. Unknown
profile rejection is not multiplex isolation. Managed gateway CLI lifecycle,
native tools/config attestation, unknown-ID recovery and PM admission/resume remain
open. Exact hashes/cleanup are in the
[verification ledger](CHAT_CLARIFICATION_VERIFICATION.md#native-hermes-protocol-acceptance-4-october-2026).

The versioned native renderer is now source-verified: new Hermes snapshots use
v2; missing/1 keeps historical config/env/marker bytes and serialization. V2 seals
native loopback host, assigned port, enablement, derived key, HOME and CORS and
rejects conflicting API aliases/malformed platform extra. An actual pinned Hermes
loader read two Rust-exported homes, checking the YAML layer without fallback,
then dotenv/config precedence against stale shell settings. No model or listener
was started, and no installed runtime was upgraded. The scoped final Linux gate
passed 300 tests, all-target check/strict Clippy/fmt/OpenAPI; 13 host harness tests
and the native loader case passed. Loaded effective revision, plugins, process
ownership and SDLC admission remain open. See [ADR 0017](adr/0017-versioned-native-hermes-renderer.md)
and [renderer evidence](CHAT_CLARIFICATION_VERIFICATION.md#versioned-native-hermes-renderer-4-october-2026).

The new source journal atomically reserves an exact free-chat run/request before
POST. It freezes bytes/hash/key, original origin/default-profile credential
fingerprint, bounded verified protocol facts and DB-clock recovery horizon.
One durable submission permit precedes IO; native ACK commits journal/run/message/
outbox together. Error classification observes current journal under the message
lock, so a stale prepared read cannot erase a concurrent submission. Known-ID
restart recovery now requires accepted original journal context; legacy history
is preserved but cannot be attested with current credentials. Migration 000012
refuses nonempty downgrade. See [ADR 0016](adr/0016-hermes-original-request-journal.md).
Prepared initial delivery is implemented by the follow-up below. Installed
unknown-key recovery acceptance, missed native tool/approval replay and full live
admission/deployment remain open.

The opt-in original-key extension now has a Base native producer and Fleet
consumer. A native SQLite witness commits with the original reservation before
inference and survives pruning; epoch, source, scope, request and DB-clock horizon
must match. Positive non-dispatch lookup restores only the original run ID;
negative/conflicting/expired/reset proof never repeats POST or frees capacity.
The feature defaults off and cannot backfill historical intents. The Linux Fleet
gate passes 195 library +127 PG/HTTP +1 approval SSE +3 migration cases (326
distinct); two actual pinned native cases and 56 Linux plugin cases pass
separately. Race tests prove concurrent recovered/normal ACK convergence and full
rollback when the horizon expires while waiting for a journal lock.
No new migration/public API, installed config, image or dependency pin changed.
The Base producer is [PR #140](https://github.com/FerrPOINT/services-base/pull/140),
ready for review with six green exact-head CI jobs, not merged;
Fleet release ordering and managed Fleet/native acceptance remain required.
See [ADR 0019](adr/0019-native-original-key-recovery.md) and the
[verification ledger](CHAT_CLARIFICATION_VERIFICATION.md#original-key-non-dispatch-recovery-4-october-2026).

The prepared-dispatch follow-up resumes only exact unconsumed free-chat journals,
using bounded keyset scans and fresh health/protocol checks. The atomic claim
rechecks identity, drain, capacity and original DB deadline; its sole winner
submits the frozen original bytes/key through the normal ACK/readback path.
Prepared uncertain outboxes reset only inside that successful permit transaction.
Submitted unknown outcomes never reset or re-send. The final Linux/Rust1.88/PG17.6
gate passed 195 library +132 foundation +1 approval SSE +3 migration cases
(331 distinct), all-target check/strict Clippy/fmt and Rust OpenAPI equality.
No new schema/public route, runtime installation, dependency pin or task/PM
authority is introduced. Managed native/Fleet acceptance and the remaining
SDLC gates stay open. See [ADR 0020](adr/0020-prepared-dispatch-restart-recovery.md)
and [evidence](CHAT_CLARIFICATION_VERIFICATION.md#prepared-dispatch-restart-recovery-4-october-2026).

Its final Linux/Rust 1.88/PostgreSQL 17.6 gate passed 179 workspace library,
102 foundation PG/HTTP, one authenticated approval SSE and three isolated
migration cases (285 distinct tests), all-target check/strict Clippy/fmt and
exact Rust OpenAPI equality. The new late prepared-error race and populated
accepted/terminal migration history pass. Node22 frontend passed 230 tests,
typecheck/lint/format/build, API compatibility and seven chat wire contracts;
135 screenshot and nine controller-image fixture verifiers passed without
recapture. README and 99 Markdown files passed. The exact Compose project was
removed; no accepted runtime, producer source or pins were changed. This is
not full release/CI or actual Fleet/native/PM acceptance. See the
[journal evidence](CHAT_CLARIFICATION_VERIFICATION.md#original-hermes-dispatch-journal-4-october-2026).

Frontend revalidation on unchanged production sources passed 229 tests,
typecheck, lint, format, build, API drift/compatibility and seven chat wire
contracts. The 135 screenshot and nine controller-image manifest/hash verifiers
passed; no images were regenerated or relabeled as live. Vite still reports a
large production chunk warning; this is not a completed performance gate.

The creation recovery follow-up adds owner/key readback for a lost initial
acknowledgement, persisted-operation continuation without prompt resubmission,
and a rollout-filtered strict Tracker project directory. It adds no runtime
dispatch and does not close admission/resume/live acceptance. The new creation
form is an isolated [design proposal](design/PM_DRAFT_CREATION_PREVIEW.md), not
production UI; approval and controller integration remain pending.

The effective-configuration follow-up verifies actual managed files against the
active database snapshot on every readiness request. Same-size drift, missing
files, re-enabled disabled skills, foreign markers and symlinked paths block
readiness. Planning/readback is read-only; activation alone creates skill paths.
This is not a fenced admission, a runtime-loaded-config proof or task-workspace
claim. The preceding WSL Rust 1.88 workspace gate passed 118 library and 39 actual
PostgreSQL tests. The corrected bundled-inventory tree passed 118 library tests,
including oversized/non-file marker denial; its broader local rerun was
interrupted when WSL became unavailable. Exact-tree Linux CI subsequently passed
all five jobs; see the [merge gate evidence](CHAT_CLARIFICATION_VERIFICATION.md).
This does not close actual PM delivery/resume or live acceptance. Frontend
passed 211 tests including Russian/English readback warnings, typecheck, lint,
format and build. OpenAPI regenerated from source is unchanged. Three separate
directory/SSE/historical tests remain ignored in this local workspace run.
Controlled filesystem/HTTP evidence does not replace live PM acceptance.
The compatibility correction preserves Hermes-owned category directories and
`.bundled_manifest`; their presence is not managed-file drift or a native-skill
attestation. A distinct unverified runtime inventory blocker remains. The
controlled regression includes this layout without trusting its manifest.

The internal PM Draft chat repository operation now commits private chat, exact
binding, two participants and one audit/event atomically. It creates no prompt or
runtime run; a bound chat cannot use ordinary message dispatch. An opt-in public
Draft creation coordinator now persists an owner/key operation, reconciles Tracker
creation and initial PM reservation, validates the immutable original input and
creates that atomic chat. It ends at `awaiting_admission`, not a runtime launch.
PM admission, initial delivery and real Backlog acceptance are not connected yet.
The namespace follow-up now checks fresh trusted Workflow project ownership on
every creation continuation before external writes, including completed replay.
The dedicated read PAT stays server-only; exact issuer/provisioner/project checks,
bounded body/deadline and no redirect/retry/proxy fallback fail closed. This is
not a workspace/execution lease or admission receipt. The follow-up passed 115
library and 38 actual PostgreSQL cases with both DB variables configured; three
separately gated directory/SSE/historical tests remain ignored in this local run.
No new UI composition, public DTO or database migration was introduced here.
The coordinator follow-up passed 111 library tests and 37 real PostgreSQL cases;
the separate fresh-DB migration/backfill/down-up test also passed. These are
controlled repository/HTTP checks,
not a real human/PM/Tracker/Workflow acceptance.

The working branch is reconciled with accepted Fleet main `11a22c1` and pinned
Base `c083783a37791e277db796361203884b87828a7d`. On Rust 1.88.0,
Node 22.20.0 and pnpm 10.28.1, 111 library/37 real PostgreSQL tests and the
separate historical migration test passed. Frontend now has 209 passing tests
after receiving the accepted shared-library cleanup; 36 three-browser fixture
cases passed and 27 live cases were skipped. Nine controller captures were
regenerated. Package-consumer, effective-theme and generated OpenAPI/client checks
passed. The compatibility gate documents one intentional security retirement:
legacy run-wide approval now returns 409; other contract changes remain checked.
These checks do not complete PM admission, initial delivery or live Backlog flow.

Production `/chats/:sessionId` now has dialogue/clarification/requirements controllers,
paginated transcript, draft preservation, read-only/dependency/unknown-outcome states and
owner exact-revision confirmation. Gateway validates immutable Tracker identity; additive
migration 000010 prevents task-chat reassignment. Normal prompts/steer cannot bypass SDLC.
Tracker backend and Workflow continuation are developed in separate repositories.

The follow-up adds server-scoped directory counts/search/cursors, immutable PM run
proof and machine-only fresh readback, and integrated exact-request tool approvals.
Readback/replay does not redispatch; fresh decisions reject stale PM assignments.
Provider JWT validation alone never grants human approval capability.
Task-bound lists/counts and all transcript/run/control reads now require current
Tracker project access; streams recheck before emitting queued events. A reservation
lock wait is followed by fresh assignment authorization before approval dispatch.

Transactional Tracker inbox storage is now implemented: exact replay deduplication,
immutable receipts, per-binding source cursor and safe transcript/stream projection.
The authenticated background metadata poller is connected behind an explicit
deployment flag (disabled by default). PM answer delivery remains unwired.

Fleet now implements strict bounded `metadata_v1` page decoding and transactional
format/version pinning. It validates all nine supported resource shapes, required
nulls, canonical non-nil UUIDs, safe versions, UTC source timestamps, source
digests and lossless decimal bigint cursors. Empty pages pin the format; changed
replay, stale cursors and implicit legacy conversion fail closed. The full WSL
gate passed 102 library tests and 29 actual PostgreSQL 17.11 cases. This storage
follow-up did not itself connect a poller or prove live PM delivery.

The polling follow-up checks the pinned machine subject and exact read-only
Tracker scope through Base on every cycle, then current Tracker project access.
Bounded GETs refuse redirects; failed or corrupt pages leave durable cursors
unchanged. Restart/replay cannot duplicate events or create a runtime dispatch.
The WSL gate passed 106 library tests and 31 actual PostgreSQL 17.11 cases.
HTTP authorization/dependency fault coverage uses test endpoints, not a live
Central issuer or PM. No accepted runtime or deployment secrets were changed.

The 2026-10-02 follow-up implements the server-only Base delegation client.
Commands derive the actual Tracker task/assignment/execution/agent/version grant;
responses must have exact scopes, a live bounded expiry and no-store protection.
Secrets are not serializable or debug-visible, and child authorization is bound
to the configured Tracker API origin. This does not yet issue credentials from
the PM coordinator or hand them to real runtime tools. The follow-up passed 100
Rust library tests, format, all-target check and strict all-target Clippy.
No new PostgreSQL or live provider acceptance is implied by that library gate.

Terminal readback now reconciles the matching Fleet run atomically, releasing
capacity without depending on a surviving SSE worker. PM cache updates cannot
invent terminal proof, change the accepted runtime ID or reopen a verified run.
This follow-up passed the full WSL suite with 100 library tests and 27 actual
PostgreSQL 17.11 cases, plus focused final-tree PM regressions. The two separately
ignored directory/approval-stream gates remain separate CI evidence. Workflow
PR #90 is reconciled with accepted master, but real PM admission remains blocked
until the actual pinned native-skills source and compatible build are available.

Verified on the refreshed working branch on 2026-10-01: 96 Fleet Rust library tests
and strict Clippy, 26 actual PostgreSQL 17.11 integration cases, 212 frontend tests and source/client
OpenAPI plus seven Tracker wire contracts. Three-browser fixture checks cover the
chat tabs at all three viewports, targeted approvals and the server directory;
chat checks include axe, tablet context and keyboard focus. These fixtures
do not prove live PM delivery. Current gaps and rollout block are listed in
[Gap Register](GAP_REGISTER.md). Existing production screenshot manifest remains historical
until live integration acceptance; new controller captures are UI fixture evidence only.
Nine controller screenshots have generated route/view/viewport/hash verification. See
[verification ledger](CHAT_CLARIFICATION_VERIFICATION.md) for service boundaries and blockers.
The merged eleven-file schema passed clean up, pending migration 000010 down,
reapply and status on an isolated PostgreSQL 17 database. This feature owns only
one new migration. Disposable QA database ownership was verified before cleanup;
accepted runtimes, images and volumes were not changed.

## October Foundation Evidence

Verified on 2026-10-01, separate from the historical baseline below:

- WSL workspace tests: 73 passed with both PostgreSQL test URLs configured,
  including six foundation HTTP/DB tests, `interrupted` failure handling and
  three provisioning preservation/ownership/active-runtime regressions, and
  legacy configuration secret masking with safe-reference preservation.
- Strict WSL Clippy and Rust formatting passed.
- Clean PostgreSQL migration up, new foundation migration down (one step),
  up/status passed. Unpublished schema changes are consolidated into 000009.
- Rust-source OpenAPI regeneration is byte-identical to the current spec;
  frontend API generation and drift check passed.
- Frontend: 125 tests in 21 files; typecheck/build, lint and format check passed
  after integrating the latest upstream header, contextual rails and safe editors.
- Shared bearer SSE: four regression tests passed.
- Installed Base UI contract: 38 route patterns; README structure and its three
  validator tests passed; Markdown links checked across 84 documents.
- Playwright: 18 fixture checks passed across Chromium/Firefox/WebKit;
  27 opt-in live checks require a running backend and are not acceptance evidence.
- Screenshot manifest verifies 135 fixture images at 375x812, 1920x1080,
  2560x1440. Desktop config and mobile Chats/transcript inspected visually.
- Standalone Rust 1.88 release and frontend Docker images built successfully
  with locked dependencies and explicit sibling Base contexts. A disposable
  clean PostgreSQL/Redis/backend/Nginx stack passed health, authenticated RBAC,
  private-chat denial, idempotent replay/conflict, immediate proxied session SSE
  and session persistence after backend restart. The disposable stack was removed.

CI now includes compile, migration rollback/reapply, frontend formatting,
route/link/screenshot gates, three-browser fixture acceptance and browser evidence
artifacts plus disposable authenticated container acceptance. Shared Base changes are published in
[services-base #121](https://github.com/FerrPOINT/services-base/pull/121).
The old CI evidence used Base `af1bdd4746dfda331d0c32741af3f7c502fad816`.
Current CI/builds use the accepted SHA in [the Base pin](../.base-revision).
GitHub CI status must be checked on the current PR head;
local checks alone do not prove CI acceptance.

The real seven-agent PM/requirements/decomposition/Rework/deployment scenario,
cross-service CI and production receipts remain unverified. Container acceptance
proves the standalone Fleet foundation, not actual Hermes/provider execution,
Central Auth integration or a complete automatic SDLC environment.

## Historical Baseline

Implemented:

- new `fleet-control` repository scaffolded from the React/Rust stack
- fresh fleet-control backend domain, migration and API skeleton
- `AgentKind = hermes | java_agent`
- separate agent product role: `leader | executor`
- agent profiles: `developer | tester | it_lead | custom`
- `SystemRole = admin | operator | user` with backend RBAC enforcement and a
  legacy `is_system_admin` alias
- local HMAC access tokens now carry fleet-compatible `aud`, `iss`, `role`,
  `scopes` and `sid` claims with strict issuer/audience validation for new
  tokens and a transition fallback for legacy compact tokens
- race-safe `agentN` ordinal allocation through a PostgreSQL sequence
- Hermes provisioning layout and local process supervisor
- Hermes launch switched to `hermes serve` for the programmatic control-plane
  surface
- Hermes `/v1/runs` adapter path for message dispatch, SSE event mirroring,
  run stop/steer and approval forwarding
- `services-base` telemetry-compatible bridge through shared tracing
  initialization and `x-request-id` middleware
- derived per-agent runtime tokens from
  `FLEET_CONTROL_FLEET__RUNTIME_TOKEN_SECRET`; raw runtime tokens are written
  only into the managed agent env/config surface
- Java Agent template and existing externally provisioned jar lifecycle;
  chat/control/config activation remain phase 2
- React application pages for fleet dashboard, leaders, executors, technical
  agents, sessions, workflows, deployments, logs and settings
- permission-aware navigation, access denied and not found states
- user-owned task sessions with default current-user filtering and multi-user
  session visibility controls
- private-by-default sessions with optional selected leader
- leader-to-executor team bindings
- delegation API for leader-created child executor sessions
- session participants and per-agent runtime runs
- runtime approval mirror records with resolved state after successful approval
  forwarding
- idempotent session and message creation guards
- deployment/provision job model and UI
- settings API/UI for runtime roots, ports, integrations, auth and user roles
- Fleet transcript mirror messages and per-agent runtime run links
- redacted audit log writes for mutating control-plane actions
- audit-log route with filters
- path marker guard for existing agent folders
- explicit physical folder purge for archived agents with confirmation, marker
  validation, event and audit trail
- read-only agent storage/retention reports for `runtime`, `config`,
  `workspace` and `logs` with marker status and purge eligibility
- fleet-wide storage/retention review for technical agent inventory with total
  managed bytes, archived bytes, purge-ready agents and marker/path issues
- full documentation baseline
- historical 82-file desktop screenshot set, superseded by the October manifest

Known local limitation:

- Native Windows Rust check is blocked until MSVC Build Tools provide
  `link.exe`.
- The earlier Docker engine hang is no longer the migration blocker: October
  tests ran against an isolated PostgreSQL 17 container. Full compose smoke has
  not been rerun.
- WSL/Linux backend check, clippy, tests and OpenAPI source regeneration pass.

Historical verified gates:

- `cargo fmt --all --check` through WSL/Linux.
- `cargo check --workspace --all-targets` through WSL/Linux.
- `cargo clippy --workspace --all-targets -- -D warnings` through WSL/Linux.
- `cargo test --workspace -- --test-threads=1` through WSL/Linux.
- clean PostgreSQL migration `up` and `status` on temporary WSL database before
  the final environment-level Docker hang; migration files were not changed
  after that pass.
- Rust-source OpenAPI regeneration through `cargo run -p api --bin gen-openapi`.
- `pnpm generate:api`, `pnpm typecheck`, `pnpm lint`, `pnpm format:check`,
  `pnpm test`, `pnpm build`.
- `pnpm exec playwright test` across Chromium, Firefox and WebKit.
- `pnpm screenshots:local` with 82 generated desktop screenshots.
- `pnpm screenshots:verify`.
- `pnpm markdown:check` for `README.md` and `docs/**/*.md`.
