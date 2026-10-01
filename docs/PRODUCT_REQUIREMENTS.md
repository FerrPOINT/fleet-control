# Product Requirements

## Goal

Fleet Control manages real agent runtimes and separate per-agent chats. Tracker,
Workflow and CI/CD own requirements, scheduling, workflow receipts and deployment.
The target is automatic SDLC after PM clarification and human confirmation of an
exact requirements revision. This target is not yet implemented end to end.

## Current SDLC Scope

- Seven independent specializations: PM, Analyst, Architect, Developer, Reviewer,
  Tester and DevOps; runtime kind and legacy product role remain separate.
- `/chats` groups a concrete agent's sessions. Free private chats do not advance
  stages. Immutable Tracker task/agent binding remains an implementation gap.
- Rework must continue the same task/agent chat; agent replacement must create
  a new chat and retain the old history without copying transcripts.
- Configuration revisions distinguish desired/effective state and use drain,
  validation, activation/readback and rollback.
- Runtime health is not SDLC readiness. Unverified assignment/workflow contracts
  keep automatic SDLC blocked.
- Leaders are deferred. Existing data and legacy routes are retained, but no new
  leader/team automation is required for this release.
- Hermes is the primary chat runtime. Existing Java jar lifecycle is retained;
  Java chat/control/config activation are phase 2.
- API ownership is authoritative. Local processes are not hostile-tenant OS
  sandboxes; see [Threat model](THREAT_MODEL.md).

Full stage requirements, owner boundaries and uncompleted acceptance are in
[SDLC implementation](SDLC_IMPLEMENTATION.md).

## Historical Baseline Scope

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
- Track deployment/provision jobs and expose the effective startup
  runtime/auth/integration configuration without pretending to apply runtime
  changes from the Fleet UI.
- Materialize guarded folders under the configured agents root.
- Expose operator UI for all management surfaces.

## Historical Baseline Criteria

These criteria describe legacy routes, not acceptance of the current SDLC plan.
New Chats must not use handoff to change an existing task-bound agent identity.

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
- `/settings` exposes read-only effective runtime roots, ports, integrations
  and legacy auth policy; user/RBAC changes are handed off to Admin Panel.
- Screenshot manifest covers all required page groups at all required
  viewports.
