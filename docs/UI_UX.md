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
- workspace guard with storage/retention preview and explicit purge control
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
