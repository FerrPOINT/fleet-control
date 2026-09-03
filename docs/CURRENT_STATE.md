# Current State

Status: pre-development hardening implemented and validated through WSL/Linux
backend gates plus Windows frontend gates. Native Windows Rust commands still
require MSVC `link.exe`.

Implemented:

- new `fleet-control` repository scaffolded from the React/Rust stack
- fresh fleet-control backend domain, migration and API skeleton
- `AgentKind = hermes | java_agent`
- separate agent product role: `leader | executor`
- agent profiles: `developer | tester | it_lead | custom`
- `SystemRole = admin | operator | user` with backend RBAC enforcement and a
  legacy `is_system_admin` alias
- race-safe `agentN` ordinal allocation through a PostgreSQL sequence
- Hermes provisioning layout and local process supervisor
- Hermes launch switched to `hermes serve` for the programmatic control-plane
  surface
- Hermes `/v1/runs` adapter path for message dispatch, SSE event mirroring,
  run stop/steer and approval forwarding
- `services-base` telemetry-compatible bridge through shared tracing
  initialization and `x-request-id` middleware
- derived per-agent runtime tokens from
  `FLEET_CONTROL_FLEET__RUNTIME_TOKEN_SECRET`; raw runtime tokens are written
  only into the managed agent env/config surface
- Java Agent runtime template and phase 2 contract placeholder
- React application pages for fleet dashboard, leaders, executors, technical
  agents, sessions, workflows, deployments, logs and settings
- permission-aware navigation, access denied and not found states
- user-owned task sessions with default current-user filtering and multi-user
  session visibility controls
- private-by-default sessions with optional selected leader
- leader-to-executor team bindings
- delegation API for leader-created child executor sessions
- session participants and per-agent runtime runs
- runtime approval mirror records with resolved state after successful approval
  forwarding
- idempotent session and message creation guards
- deployment/provision job model and UI
- settings API/UI for runtime roots, ports, integrations, auth and user roles
- Fleet transcript mirror messages and per-agent runtime run links
- redacted audit log writes for mutating control-plane actions
- audit-log route with filters
- path marker guard for existing agent folders
- explicit physical folder purge for archived agents with confirmation, marker
  validation, event and audit trail
- full documentation baseline
- generated 128-file screenshot set for all required pages at required viewports

Known local limitation:

- Native Windows Rust check is blocked until MSVC Build Tools provide
  `link.exe`.
- Final local Docker migration smoke rerun is blocked because `docker compose`
  hangs before returning even `ps`; this is tracked as an environment gap.
- WSL/Linux backend check, clippy, tests and OpenAPI source regeneration pass.

Latest verified gates:

- `cargo fmt --all --check` through WSL/Linux.
- `cargo check --workspace --all-targets` through WSL/Linux.
- `cargo clippy --workspace --all-targets -- -D warnings` through WSL/Linux.
- `cargo test --workspace -- --test-threads=1` through WSL/Linux.
- clean PostgreSQL migration `up` and `status` on temporary WSL database before
  the final environment-level Docker hang; migration files were not changed
  after that pass.
- Rust-source OpenAPI regeneration through `cargo run -p api --bin gen-openapi`.
- `pnpm generate:api`, `pnpm typecheck`, `pnpm lint`, `pnpm format:check`,
  `pnpm test`, `pnpm build`.
- `pnpm exec playwright test` across Chromium, Firefox and WebKit.
- `pnpm screenshots:local` with 128 generated screenshots.
- `pnpm screenshots:verify`.
- `pnpm markdown:check` for `README.md` and `docs/**/*.md`.
