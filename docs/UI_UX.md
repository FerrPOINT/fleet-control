# UI/UX

Fleet Control is an operational application. The UI should be dense, calm and
scan-friendly.

Screens:

- dashboard
- leaders
- leader create, detail, edit and team editor
- executors
- executor create, detail and edit
- agents
- create agent wizard
- agent overview and edit
- runtime controls
- skills editor
- config/SOUL editor
- technical agents inventory storage review, workspace guard with
  storage/retention preview and explicit purge control
- agent sessions
- global sessions
- session detail, transcript mirror, leader selector, runtime runs and handoff
- session delegation flow and parent/child sessions
- workflow bindings
- deployments overview, jobs and job detail/cancel
- logs process, events and audit tabs
- settings runtime, ports, integrations, auth and users/RBAC tabs
- access denied and not found states
- login and register

Use local shadcn-style primitives, lucide icons, theme tokens and fixed control
dimensions. Avoid marketing-style hero pages.

Session and agent session lists show a user avatar for every session. The user
filter defaults to the current user, supports removing users with an inline
close control, and supports adding multiple users from the users list. An empty
selection means all users for admin/operator users; normal users remain scoped
to themselves by the backend.

Leader UX rules:

- `/leaders` is the main team-coordination entry point.
- `/executors` is the main delivery-agent entry point.
- `/agents` is technical inventory for runtime/config/process inspection.
- Creating a leader exposes managed executor selection.
- Creating a direct executor session leaves the leader selector empty and shows
  it as private.
- Creating or opening a leader-scoped session shows the selected leader badge
  and enables the leader author option in the composer.
- Delegation from a leader-scoped session creates a child executor session and
  does not auto-add the entire team as participants.

Quality states:

- Every route needs loading, empty, error and access denied behavior where
  applicable.
- Mutating buttons are disabled while saving.
- Permission-gated navigation uses `/api/v1/users/me/permissions`.

Agent detail (2026-10-01):

- Overview, runtime, skills, config, workspace and sessions use the active
  Russian/English locale. Agent names, file paths, runtime diagnostics, logs and JSON remain data,
  not translated API identifiers.
- Section links wrap without a horizontal scroller and retain a 40 px target.
- Agent, log, skill, config, storage and session loading failures have a retry
  action and are distinct from an empty result.
- Config and skill editors lock during mutation. Failed saves retain their
  drafts; success appears after a successful response and clears on new edits.
- Both JSON editors require an object; malformed JSON, arrays and scalar values
  disable config submission. File purge also requires a verified storage marker.
- This change does not close the separate detail-column geometry/header audit.
