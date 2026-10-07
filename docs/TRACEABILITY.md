# Traceability

Controller recovery storage candidate: [ADR0032](adr/0032-fenced-controller-recovery-epochs.md),
[internal contract](contracts/CONTROLLER_RECOVERY_V1.md), additive000020 and
`infra/controller_recovery` reserve/readback/heartbeat/receipt operations.
Six PostgreSQL `container_lifecycle_tests::controller_recovery_*` cases cover
original identity, concurrent requests, version/expiry, held original effects
and immutable history. `migration/tests/controller_recovery` verifies additive
upgrade, empty down/up and retained-history refusal in an owned schema. Queue,
dispatch journal, runtime launch/endpoint and supervisor generation guards
consume the private recovery marker. This is not a native Base handover or
automatic worker. The fresh562-case Linux/PostgreSQL source gate passes; real
restart acceptance remains separate. See
[evidence](CHAT_CLARIFICATION_VERIFICATION.md#fenced-controller-recovery-storage-7-october-2026).

Integrated original-key UI consumer: normal merge49c11f5 combines runtime9a11bde
and published Chats16b7516. `api/runtime-control-lookup.ts` canonicalizes the
Rust-compatible semantic digest; `pages/chat-detail` fixes original command scope
and releases a hold only after authorized fresh ACK. Its API/unit/browser cases
cover lost initial receipt ID, stale reads, changed active run and preservation
of an unrelated composer draft, without another POST. The frontend tree matches
16b7516; backend tree matches9a11bde. See
[combined evidence](CHAT_CLARIFICATION_VERIFICATION.md#integrated-chats-and-runtime-candidate-7-october-2026).
Reload persistence, direct SSE denial, native public-route acceptance and actual
PM/admission/controller recovery remain separate requirements.

Controller restart observation:
[private contract](contracts/CONTAINER_CONTROL_V1.md#original-controller-restart-observation),
`container_control::observe_controller_restart`, closed witness decoder and
`container_lifecycle::health_container_locked` original DB ACK/PID checks.
Three client/subprocess cases and one PostgreSQL supervisor case distinguish
read-only degraded health from new ownership and retain lifecycle/generation
holds. Base source9171fe6 has separate actual same-container Compose restart
evidence with original synthetic agent/journal preservation. See
[exact evidence](CHAT_CLARIFICATION_VERIFICATION.md#original-controller-restart-observation-7-october-2026).
Full fenced transfer, interrupted activation and actual Fleet recovery remain open.

Public original-key control recovery:
[consumer contract](contracts/HERMES_RUN_CONTROL_V1.md#fleet-command-journal),
`sessions::lookup_control`, indexed `runtime_controls::lookup` and two domain
hash/query cases. Three PostgreSQL/HTTP `runtime_run_control` lookup cases cover
lost receipt identity, concurrent/reconstructed readers, original actor/payload,
terminal/uncertain state and access revocation without another native POST.
[Exact551-case evidence](CHAT_CLARIFICATION_VERIFICATION.md#original-key-control-lookup-7-october-2026)
is not UI wiring, native reply-loss or OS-restart acceptance of this public route.

Private Docker diagnostic logs: [ADR0031](adr/0031-private-bounded-container-log-readback.md),
[private contract](contracts/CONTAINER_CONTROL_V1.md), Rust
`runtime/container_control::log_tail` and its two binary/closed-receipt cases.
Base source69831aa contains eight focused request/pipe cases, including original
receipt drift, byte overflow, concurrent pipes and reader timeout/reaping.
The actual owned `--log-readback` probe checks four exited generations after
real Rust/Hermes chat/config acceptance; it invokes Base directly, not the Rust
client. Fourteen driver safety cases include extension failure-state retention.
[Exact evidence](CHAT_CLARIFICATION_VERIFICATION.md#actual-private-base-docker-log-readback-6-october-2026)
does not close production log ingestion/redaction/cursor/rotation or PM/SDLC.

Pre-spawn launch journal: [ADR0026](adr/0026-pre-spawn-runtime-launch-journal.md),
[internal contract](contracts/RUNTIME_LAUNCH_JOURNAL_V1.md), additive000017,
`runtime_launches` and `runtime/launch_journal`. PostgreSQL/controller cases in
`runtime/launch_journal_tests` cover concurrent claim, crash holds, immutable
identity/PID/history, config source revisions, retained-child ACK recovery,
atomic exit failure and stale metadata denial. Closed private dispatch binding
and real-child replacement are covered there; three foundation journal cases
verify original permit, changed generation and legacy-to-managed denial.
`migration/tests/runtime_launches` requires a dedicated empty database for actual
upgrade/down/reapply and retained-history guards. Native lifecycle compatibility
and final source fingerprints belong in the verification ledger, not inferred
from these component cases. No host boundary, producer admission, migration
lineage reconciliation or full SDLC completion is claimed.

Original approval journal: [ADR0025](adr/0025-original-approval-outcome-journal.md),
additive000016, `approval_outcomes` and seven PostgreSQL foundation cases plus
the isolated upgrade/down/reapply test. Coverage includes single-use claim,
closed action/original scope, concurrent completion, audit rollback, SQL history
guards and cancelled/terminal history after revocation. The opt-in approval HTTP
sender/GET worker is connected through `runtime/approval_outcome` and the actual
approval API middleware. HTTP/PG component coverage includes concurrent choices,
lost ACK/new repository, bad preflight/legacy, unknown/foreign witness, DB rollback,
terminal/revocation and disabled/rotated contexts. Component cases do not prove
native recovery; separate native `approval-outcomes` verifies real once/deny,
lost ACK held through terminal, original GET settlement, exact bytes/UUID/epoch,
one POST/ACK/audit and immutable history. Separate `approval-restart` verifies
two SIGKILLs/three Fleet PIDs, new original GET after restart and late settlement
without changing terminal history. Native `combined-controls` and
`combined-recovery` add both committed plugins, lost real initial202, original
read-only POST run lookup then GET action settlement and the same OS-death checks.
Installed release remains a gate; see the verification ledger.

Native control-outcome GET: [contract v1](contracts/HERMES_CONTROL_OUTCOME_V1.md),
`runtime/control_outcome_wire` and eleven protocol tests cover saved raw bytes,
epoch/scope/origin, closed capabilities/JSON, exact targeted ACKs and bounded
GET-only failures. Additive000015 and `runtime_controls` implement the internal
single-use context/ACK journal; seven PostgreSQL cases and isolated migration
upgrade/down/reapply/history-loss denial verify that boundary. Supervisor
dispatch/recovery is connected behind the default-false flag;
`runtime_control_outcome_http` exercises exact POST, original GET, race/rollback,
revocation/epoch/credential denial and bounded keyset behavior. The managed native
transport-loss/gateway-restart consumer case passes with real Hermes and a
loopback model. Approval has its separate native case above. This is not
installed consumer, safe-stop or full SDLC acceptance.

Journal clock-order repair: [ADR 0023](adr/0023-logical-journal-progress-time.md),
additive migration 000014, deterministic clock-regression PG test and isolated
upgrade/down/reapply. Current approval readback: [ADR 0022](adr/0022-current-native-approval-snapshot.md)
and native two-Fleet-process scenario. Logical timestamps do not attest clocks;
current snapshot recovery does not supply complete approval/tool history.

## October SDLC Scope

Original-key recovery candidate: [ADR 0019](adr/0019-native-original-key-recovery.md),
[wire v1](contracts/HERMES_RECOVERY_V1.md), source `recovery_wire` and atomic
`accept_recovered_hermes_run`, PG/HTTP `runtime_unknown_recovery`/`recovery_race`
and actual pinned native recovery harness. Installed end-to-end, native config/
OS/process-tree proof and task/PM admission remain gates. Prepared initial delivery
now has bounded restart recovery; see [ADR 0020](adr/0020-prepared-dispatch-restart-recovery.md)
and `runtime_prepared_recovery` PostgreSQL/HTTP tests. Managed native lost-ACK
recovery of an already-terminal free-chat run now has a separate two-Fleet-process
case; running/approval/native-crash and installed recovery remain gates.

| Requirement | Evidence / Remaining Gate |
| --- | --- |
| Docker named-volume isolation | Private protocol2/policy3 consumer, original mapping in creation/prepared/DB binding, canonical proof digest and exact per-agent subpaths. Three control units plus two Linux/PG lifecycle cases cover recipe/digest/Engine/downgrade/sibling guards, replay, private-file/controller drift, unknown start and witnessed stop. Actual Rust Fleet mapped Hermes/model/config/restart and installed acceptance remain; see [verification](CHAT_CLARIFICATION_VERIFICATION.md) |
| Seven independent specializations | `SdlcRole`, migration 000009, create/edit; no seven-agent live acceptance yet |
| Agent -> own chats | `/chats/:sessionId`, immutable task binding, owner-only persisted Draft/reservation/chat coordinator; actual admitted PM start pending |
| No new leader orchestration | Main nav and Chats controls exclude it; legacy routes/history preserved |
| Per-user visibility | Default backend filter, private message authorization regression, SSO stored-role tests |
| No duplicate unknown dispatch | Immutable exact-request journal (000012), single durable submission permit, atomic ACK, original-key positive readback and late-error classification; submitted unknown outcomes never reset. Native lost-ACK/two Fleet processes require one POST/inference/assistant and immutable context; installed/running recovery remains |
| Prepared initial delivery after restart | `prepared_dispatch` bounded scan, fresh protocol and shared submission path; `runtime_prepared_recovery` concurrent supervisor/uncertain-claim/malformed-ACK/stale-context/exclusion tests. No task/PM authority or public operator repair |
| No completion from EOF | Fake Hermes non-terminal/terminal readback; interrupted is failure, never a fabricated reply |
| Bounded native stream without false release | `sse_wire` incremental byte framing, strict JSON/run identity, traffic/frame/text/snapshot/deadline limits; `runtime_stream_bounds` PG/HTTP failures retain capacity. See consumer profile and verification ledger; upstream missed-event replay remains open |
| Native controls cannot fabricate success | `run_control` original context/fresh status/bounded ACK; migration 000013 and `runtime_controls` immutable actor/key/context, single claim, ACK transaction, terminal-only reconciliation. Final367-case Linux/PG component gate, separate real AIAgent ACK/replay case and three-browser unknown/reload fixtures pass. Exact published-head/release gates, HTTP/live identity, native unknown-command acceptance, approvals and task admission remain |
| Atomic free-chat terminal and pin-to-worker recovery | `runtime_terminal` PG rollback/concurrency/late-event guards; `runtime_pinned_recovery` fresh-supervisor GET-only fixtures; native/PM live acceptance remains separate |
| Durable cursor | Migration 000009, session cursor/Last-Event-ID, Base reconnect tests; expiry/reset pending |
| Config activation/drain | Migration 000009, desired/effective snapshots, DB drain/failed rollback regression |
| Private activation backups | Version 2 controller-side journal, canonical source paths, Linux owner/mode/link guards and preserved legacy hold; filesystem and PostgreSQL lifecycle regressions. OS identity/mount isolation, crash takeover and Windows ACL support remain separate gates |
| Durable managed-file apply/rollback | `configuration_disk` Linux parent fsync for rename/unlink/new directory ancestors; test-only post-rename barrier failure retains journal/drain without effective head or runtime spawn. Power-loss/crash takeover and Windows directory durability remain unverified |
| Native control outcome witness | Base `hermes-control-plugin`; Fleet `run_control` and `control_outcome_readback` use original context/claim/GET only. PG/HTTP and wire fixtures cover failure/race/bounds. Native `control-outcomes`/`control-restart` and separate `approval-outcomes`/`approval-restart` verify actual transport loss, original GET recovery and OS-process deaths without second POST/history change; see the exact-source verification ledger. Combined plugins and installed acceptance remain gates |
| Managed native free-chat lifecycle | `native_supervisor_live` explicit opt-in cases: actual Fleet activation/outbox, two native gateways, loaded SOUL/token isolation, idempotent mirrors and restart identity. Separate lost-ACK recovery uses the real Base witness plugin and distinct Fleet processes; separate steer/stop uses a real AIAgent and terminal readback. Model is local; installed/control-command recovery/approvals/process-tree/PM gates remain |
| Native Hermes listener rendering | Server-selected snapshot renderer v2 seals listener/env/CORS; v1 history remains reproducible. Native loader and installed-runtime attestation are separate gates; see ADR 0017 |
| Safe automatic publication | Tracker exact owner/revision/hash gate implemented; trusted prerequisite verifier and real first-step PM integration pending |
| SDLC receipt/deployment | Not implemented here; cross-service and real deploy evidence still required |

## PM Clarification Trace

| Requirement | Implementation / evidence | Remaining |
| --- | --- | --- |
| Immutable instance/task/concrete-agent chat | migration 000010, atomic binding and owner/key creation ledger, restart/concurrency/lost-response tests | Admitted runtime dispatch and live saga acceptance |
| Owner-only answer and exact consent | Central-subject gateway and strict Tracker SDLC commands, read-only and exact-hash UI tests | Real identity/project live denial acceptance |
| Questions, versions, no preselection | Generated DTOs, single/multiple/text validators, stale draft tests | Structured PM tools and real question publication |
| No unknown-command reinterpretation | Frozen message/answer payloads and keys, uncertain-steer regression | PM delivery readback/rebind |
| Requirements revision changes revoke UI consent | Revision/hash form identity, full document and comparison, regression test | Trusted exact-revision prerequisite evidence |
| Wire drift | Rust OpenAPI, generated client, seven-schema Tracker snapshot check | Compatible deployed versions and CI head verification |
| Human-only stop/steer, private command receipts | VerifiedHumanSession gate before lookup; actual local JWT HTTP owner/replay/revocation, foreign session/run denial and operator/admin reads; sessionless admin denied without native POST | Live central session/PAT identity, assignment-scoped machine control and release CI |
| Exact-action approvals without duplicate effect | Native `approvals`, `approval-recovery`, `approval-outcomes`, `approval-restart` and `combined-recovery`: real Hermes terminal guard/request/effect, owner HTTP once/deny, one POST, current waiting GET recovery, original decision witness and recovery after two SIGKILLs/three Fleet PIDs without rewriting terminal history; mixed case also loses initial202 and verifies original run lookup with both plugins | Loaded config/task admission, central identity and installed/live UI |
| Approval targets original current context | Shared accepted journal/run/session/origin/credential guard, fresh native approval capabilities and waiting exact request readback; bounded HTTP200 JSON ACK. Four journaled component cases and renewed actual native approval/control checks | Loaded configuration generation, fresh distributed authorization/fencing and task admission remain |
| Unknown delivery avoids session/message lock inversion | Explicit PG holder, pg_blocking_pids barrier and message NOWAIT regression: fails before session-first delivery and passes after; unknown acceptance HTTP still requires one POST and held pending capacity | No claim that all DB deadlocks or Docker VM clock regressions are fixed |
| Keyboard/mobile/desktop | Three-browser controller fixtures, axe, Escape focus and tab arrows, generated image hashes | Live production acceptance, not fixture promotion |

The table below describes the legacy baseline, not complete SDLC acceptance.

| Requirement               | Implementation                                                             |
| ------------------------- | -------------------------------------------------------------------------- |
| Two runtime kinds         | `AgentKind`, runtime templates, create wizard                              |
| Start with Hermes         | `Hermes` template implemented, Java Agent phase 2                          |
| Leaders and executors     | `AgentProductRole`, `/leaders`, `/executors`, `leader_executors`           |
| RBAC                      | `SystemRole`, `/users/me/permissions`, protected backend routes            |
| Sequential agent folders  | DB ordinal, `agentN` path derivation                                       |
| Isolated config/workspace | per-agent `config` and `workspace` paths                                   |
| Per-agent skills          | `agent_skills`, skills tab                                                 |
| Sessions as tasks         | `agent_sessions`, `session_messages`, `session_agent_runs`, sessions pages |
| Private by default        | nullable `leader_agent_id`, `visibility = private`                         |
| Sessions per user         | backend current-user default filter, user avatars in sessions/agents lists |
| Leader-scoped tasks       | `leader_agent_id`, `/sessions/{id}/leader`, leader sessions UI             |
| Leader delegation         | `/sessions/{id}/delegations`, parent/child sessions, managed executor check |
| Agent switching/handoff   | session handoff API, participants and runtime runs                         |
| Idempotency               | session/message idempotency keys, payload hashes and conflict handling      |
| Settings                  | Read-only effective `/settings/runtime`, `/ports`, `/integrations`, `/auth`; users handoff |
| Deployments               | deployment job model, list/detail/create/cancel UI and API                 |
| Logs                      | process logs, events and audit tabs                                        |
| Operator audit            | `audit_log` writes for mutating agent/session/runtime/config/skill actions |
| Screenshots               | generated 135-file three-viewport fixture screenshot manifest               |
| Full SDLC docs            | docs index, contracts, ADRs and pre-development gate docs                  |
| Current native approval recovery | `approval_snapshot`, atomic `hermes_approval_recovery`, six PG cases, real two-Fleet-process native gate; not historical queue or unknown decision recovery |
