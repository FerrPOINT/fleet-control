# Traceability

## October SDLC Scope

Original-key recovery candidate: [ADR 0019](adr/0019-native-original-key-recovery.md),
[wire v1](contracts/HERMES_RECOVERY_V1.md), source `recovery_wire` and atomic
`accept_recovered_hermes_run`, PG/HTTP `runtime_unknown_recovery`/`recovery_race`
and actual pinned native recovery harness. Installed end-to-end, native config/
OS/process-tree proof and task/PM admission remain gates. Prepared initial delivery
now has bounded restart recovery; see [ADR 0020](adr/0020-prepared-dispatch-restart-recovery.md)
and `runtime_prepared_recovery` PostgreSQL/HTTP tests. Managed native/Fleet recovery
acceptance remains separate.

| Requirement | Evidence / Remaining Gate |
| --- | --- |
| Seven independent specializations | `SdlcRole`, migration 000009, create/edit; no seven-agent live acceptance yet |
| Agent -> own chats | `/chats/:sessionId`, immutable task binding, owner-only persisted Draft/reservation/chat coordinator; actual admitted PM start pending |
| No new leader orchestration | Main nav and Chats controls exclude it; legacy routes/history preserved |
| Per-user visibility | Default backend filter, private message authorization regression, SSO stored-role tests |
| No duplicate unknown dispatch | Immutable exact-request journal (000012), single durable submission permit, atomic ACK, original-key positive readback and late-error classification; submitted unknown outcomes never reset. Managed native/Fleet acceptance remains required |
| Prepared initial delivery after restart | `prepared_dispatch` bounded scan, fresh protocol and shared submission path; `runtime_prepared_recovery` concurrent supervisor/uncertain-claim/malformed-ACK/stale-context/exclusion tests. No task/PM authority or public operator repair |
| No completion from EOF | Fake Hermes non-terminal/terminal readback; interrupted is failure, never a fabricated reply |
| Atomic free-chat terminal and pin-to-worker recovery | `runtime_terminal` PG rollback/concurrency/late-event guards; `runtime_pinned_recovery` fresh-supervisor GET-only fixtures; native/PM live acceptance remains separate |
| Durable cursor | Migration 000009, session cursor/Last-Event-ID, Base reconnect tests; expiry/reset pending |
| Config activation/drain | Migration 000009, desired/effective snapshots, DB drain/failed rollback regression |
| Managed native free-chat lifecycle | `native_supervisor_live` explicit opt-in PASS: actual Fleet activation/outbox, two native gateways, loaded SOUL/token isolation, idempotent mirrors and restart identity. Model is local; installed/control/lost-ACK/process-tree/PM gates remain |
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
