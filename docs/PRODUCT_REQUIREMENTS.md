# Product Requirements

## Goal

Fleet Control manages multiple isolated agent runtimes and lets an operator move
task sessions between them.

## MVP

- Create sequential managed agents: `agent1`, `agent2`, and so on.
- Support two runtime kinds in the model: Hermes and Java Agent.
- Implement Hermes provisioning and process lifecycle first.
- Show Java Agent as a selectable runtime template with phase 2 capability
  status.
- Store agent identity, paths, config, skills, sessions, workflow bindings,
  logs and events in PostgreSQL.
- Materialize guarded folders under the configured agents root.
- Expose operator UI for all management surfaces.

## Success Criteria

- Fresh database seeds Developer Hermes and Tester Hermes.
- Each agent has distinct runtime, config, workspace and logs folders.
- Editing skills/config for one agent does not modify another.
- Sessions are visible globally and per agent.
- Handoff updates the session owner and target namespace.
