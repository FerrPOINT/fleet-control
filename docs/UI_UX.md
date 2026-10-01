# UI/UX

Fleet Control is an operational application. The UI should be dense, calm and
scan-friendly.

The complete chat/session behavior, current polling implementation, target
streaming UX and acceptance scenarios are documented in [CHAT.md](CHAT.md).

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
- The global header uses the Base `PlatformHeader`, above the sidebar content offset.
  Brand and services belong to the header; the sidebar contains only navigation.
  Identity and central sign-out appear once in the bounded account menu.
  Mobile controls/navigation are at least 44 px; desktop controls are 40 px.
  The drawer closes after navigation, Escape and the 768 px desktop breakpoint.
  Existing permission filtering and page-specific create actions are preserved.

Detail geometry (2026-10-01):

| Route | Mode | Primary / contextual rail |
|---|---|---|
| `/agents/:id`, `/executors/:id` | detail-with-aside | Identity / runtime snapshot |
| `/agents/:id/workspace`, executor alias | detail-with-aside | Managed paths / storage and purge controls |
| `/sessions/:id` | detail-with-aside | Transcript and runs / session controls and participants |
| `/leaders/:id` | detail-with-aside | Sessions / team editor |
| Agent runtime, skills and config tabs | wide | Two working panels, not a metadata rail |

Detail layouts use `page-split` from Base, not a locally copied grid contract.
The contextual rail is 320 px at viewport widths >= 1024 px and follows the
primary content below that breakpoint. Grid children have `min-width: 0` and
align to the start so a long rail does not stretch a short primary tool into a
large empty card. The server API and action behavior are unchanged.
