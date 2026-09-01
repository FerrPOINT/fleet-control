# Current State

Status: initial implementation scaffold.

Implemented:

- new `fleet-control` repository scaffolded from the React/Rust stack
- fresh fleet-control backend domain, migration and API skeleton
- `AgentKind = hermes | java_agent`
- Hermes provisioning layout and local process supervisor
- Java Agent runtime template and phase 2 contract placeholder
- React application pages for fleet dashboard, agents, sessions, workflows,
  deployments, logs and settings
- full documentation baseline

Known local limitation:

- Windows Rust check is blocked until MSVC Build Tools provide `link.exe`, or a
  GNU/Linux toolchain is used.
