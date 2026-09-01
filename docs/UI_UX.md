# UI/UX

Fleet Control is an operational application. The UI should be dense, calm and
scan-friendly.

Screens:

- dashboard
- agents
- create agent wizard
- agent overview
- runtime controls
- skills editor
- config/SOUL editor
- workspace guard
- agent sessions
- global sessions
- session detail and handoff
- workflow bindings
- deployments
- logs
- settings
- login and register

Use local shadcn-style primitives, lucide icons, theme tokens and fixed control
dimensions. Avoid marketing-style hero pages.

Session and agent session lists show a user avatar for every session. The user
filter defaults to the current user, supports removing users with an inline
close control, and supports adding multiple users from the users list. An empty
selection means all users.
