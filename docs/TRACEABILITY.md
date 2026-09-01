# Traceability

| Requirement | Implementation |
| --- | --- |
| Two runtime kinds | `AgentKind`, runtime templates, create wizard |
| Start with Hermes | `Hermes` template implemented, Java Agent phase 2 |
| Sequential agent folders | DB ordinal, `agentN` path derivation |
| Isolated config/workspace | per-agent `config` and `workspace` paths |
| Per-agent skills | `agent_skills`, skills tab |
| Sessions as tasks | `agent_sessions`, sessions pages |
| Sessions per user | `agent_sessions.user_id`, sessions/agents current-user filter |
| Agent switching/handoff | session handoff API and UI |
| Full SDLC docs | docs index, contracts and ADRs |
