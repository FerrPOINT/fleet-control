# Traceability

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
| Settings                  | `/settings/runtime`, `/ports`, `/integrations`, `/auth`, users/RBAC UI      |
| Deployments               | deployment job model, list/detail/create/cancel UI and API                 |
| Logs                      | process logs, events and audit tabs                                        |
| Operator audit            | `audit_log` writes for mutating agent/session/runtime/config/skill actions |
| Screenshots               | generated 128-file screenshot manifest                                     |
| Full SDLC docs            | docs index, contracts, ADRs and pre-development gate docs                  |
