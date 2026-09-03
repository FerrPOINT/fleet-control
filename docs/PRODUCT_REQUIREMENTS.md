# Product Requirements

## Goal

Fleet Control manages multiple isolated agent runtimes and lets an operator
coordinate executor task sessions directly or through leader agents.

## MVP

- Create sequential managed agents: `agent1`, `agent2`, and so on.
- Support two runtime kinds in the model: Hermes and Java Agent.
- Implement Hermes provisioning and process lifecycle first.
- Show Java Agent as a selectable runtime template with phase 2 capability
  status.
- Store agent identity, paths, config, skills, sessions, workflow bindings,
  logs and events in PostgreSQL.
- Separate runtime kind (`hermes`, `java_agent`) from product role (`leader`,
  `executor`) and profile (`developer`, `tester`, `it_lead`, `custom`).
- Let leaders manage selected executors and write into sessions where they are
  explicitly selected.
- Store each session under the authenticated user who created it.
- Enforce `admin`, `operator` and `user` permissions in the backend.
- Keep human-created executor chats private by default.
- Make session and message creation idempotent.
- Store Fleet transcript mirror messages and per-agent runtime run links.
- Track deployment/provision jobs and editable runtime/auth/integration
  settings.
- Materialize guarded folders under the configured agents root.
- Expose operator UI for all management surfaces.

## Success Criteria

- Fresh database seeds Developer Hermes and Tester Hermes.
- Each agent has distinct runtime, config, workspace and logs folders.
- Editing skills/config for one agent does not modify another.
- Session lists default to the current user's sessions and can be expanded to
  all users or narrowed to multiple selected users by admin/operator.
- Normal users can create and use only their own sessions and see safe agent
  directory data.
- Creating a direct executor session leaves it private.
- Creating a direct leader session selects that leader by default.
- Creating/delegating a child executor session from a leader chat records
  `parent_session_id` and the selected leader.
- Selecting a leader for an executor session is allowed only when that leader
  manages the executor.
- Handoff updates the target agent and namespace while preserving the session
  user.
- Replaying the same session/message idempotency key returns the previous result
  without duplicate runtime writes; a changed payload returns `409`.
- `/deployments` exposes job create/detail/cancel states.
- `/settings` exposes runtime roots, ports, integrations, auth and users/RBAC.
- Screenshot manifest covers all required page groups at all required
  viewports.
