# Traceability

## October SDLC Scope

| Requirement | Evidence / Remaining Gate |
| --- | --- |
| Seven independent specializations | `SdlcRole`, migration 000009, create/edit; no seven-agent live acceptance yet |
| Agent -> own chats | `/chats`, Chats unit + three-browser fixture E2E; immutable task binding pending |
| No new leader orchestration | Main nav and Chats controls exclude it; legacy routes/history preserved |
| Per-user visibility | Default backend filter, private message authorization regression, SSO stored-role tests |
| No duplicate unknown dispatch | Transactional outbox, agent capacity regression; crashed acceptance recovery pending |
| No completion from EOF | Fake Hermes non-terminal/terminal readback; interrupted is failure, never a fabricated reply |
| Durable cursor | Migration 000009, session cursor/Last-Event-ID, Base reconnect tests; expiry/reset pending |
| Config activation/drain | Migration 000009, desired/effective snapshots, DB drain/failed rollback regression |
| Safe automatic publication | Fail-closed readiness only; Tracker requirements and first-step workflow gate pending |
| SDLC receipt/deployment | Not implemented here; cross-service and real deploy evidence still required |

The following table describes the legacy baseline, not complete SDLC acceptance.

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
