# Traceability

Native control-outcome GET: [contract v1](contracts/HERMES_CONTROL_OUTCOME_V1.md),
`runtime/control_outcome_wire` and eleven protocol tests cover saved raw bytes,
epoch/scope/origin, closed capabilities/JSON, exact targeted ACKs and bounded
GET-only failures. This is wire-consumer evidence, not production journal,
positive native approval, safe-stop or full SDLC acceptance.

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
| Durable managed-file apply/rollback | `configuration_disk` Linux parent fsync for rename/unlink/new directory ancestors; test-only post-rename barrier failure retains journal/drain without effective head or runtime spawn. Power-loss/crash takeover and Windows directory durability remain unverified |
| Native control outcome witness | Base `hermes-control-plugin`; Fleet protocol `controls` native producer QA verifies actual lost steer/stop ACK, GET-only restart and unknown hold. Production consumer, positive native approval, combined plugins and installed acceptance remain required |
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
| Exact-action approvals without duplicate effect | Native `approvals` case: real Hermes terminal guard/request/effect, actual local JWT owner HTTP once/deny, one POST, unchanged transcript replay and lost real ACK held uncertain after terminal;21 host safety cases | Native unknown-decision lookup, crash while waiting, current config/task admission, central identity and installed/live UI |
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
