# Current State

Status (2026-10-01): SDLC foundation is partially implemented. Automatic SDLC is
blocked until cross-service assignment/workflow/deployment contracts are verified.
See [SDLC implementation](SDLC_IMPLEMENTATION.md). The baseline feature/gate lists
below are historical, not acceptance evidence for the new SDLC plan.

New implementation: seven specializations, agent-grouped `/chats`, read-only
directory, persistent bearer SSE replay, transactional prompt outbox, concurrent
idempotency, no effective-admin central bypass, versioned configuration activation
with drain/readback/rollback and fail-closed SDLC readiness.

Native Windows Rust commands still require MSVC `link.exe`.

## PM Clarification Work In Progress

Production `/chats/:sessionId` now has dialogue/clarification/requirements controllers,
paginated transcript, draft preservation, read-only/dependency/unknown-outcome states and
owner exact-revision confirmation. Gateway validates immutable Tracker identity; additive
migration 000010 prevents task-chat reassignment. Normal prompts/steer cannot bypass SDLC.
Tracker backend and Workflow continuation are developed in separate repositories.

The follow-up adds server-scoped directory counts/search/cursors, immutable PM run
proof and machine-only fresh readback, and integrated exact-request tool approvals.
Readback/replay does not redispatch; fresh decisions reject stale PM assignments.
Provider JWT validation alone never grants human approval capability.
Task-bound lists/counts and all transcript/run/control reads now require current
Tracker project access; streams recheck before emitting queued events. A reservation
lock wait is followed by fresh assignment authorization before approval dispatch.

Transactional Tracker inbox storage is now implemented: exact replay deduplication,
immutable receipts, per-binding source cursor and safe transcript/stream projection.
The authenticated background poller and PM answer delivery remain unwired.

Verified on the refreshed working branch on 2026-10-01: 96 Fleet Rust library tests
and strict Clippy, 26 actual PostgreSQL 17.11 integration cases, 212 frontend tests and source/client
OpenAPI plus seven Tracker wire contracts. Three-browser fixture checks cover the
chat tabs at all three viewports, targeted approvals and the server directory;
chat checks include axe, tablet context and keyboard focus. These fixtures
do not prove live PM delivery. Current gaps and rollout block are listed in
[Gap Register](GAP_REGISTER.md). Existing production screenshot manifest remains historical
until live integration acceptance; new controller captures are UI fixture evidence only.
Nine controller screenshots have generated route/view/viewport/hash verification. See
[verification ledger](CHAT_CLARIFICATION_VERIFICATION.md) for service boundaries and blockers.
The merged eleven-file schema passed clean up, pending migration 000010 down,
reapply and status on an isolated PostgreSQL 17 database. This feature owns only
one new migration. Disposable QA database ownership was verified before cleanup;
accepted runtimes, images and volumes were not changed.

## October Foundation Evidence

Verified on 2026-10-01, separate from the historical baseline below:

- WSL workspace tests: 73 passed with both PostgreSQL test URLs configured,
  including six foundation HTTP/DB tests, `interrupted` failure handling and
  three provisioning preservation/ownership/active-runtime regressions, and
  legacy configuration secret masking with safe-reference preservation.
- Strict WSL Clippy and Rust formatting passed.
- Clean PostgreSQL migration up, new foundation migration down (one step),
  up/status passed. Unpublished schema changes are consolidated into 000009.
- Rust-source OpenAPI regeneration is byte-identical to the current spec;
  frontend API generation and drift check passed.
- Frontend: 125 tests in 21 files; typecheck/build, lint and format check passed
  after integrating the latest upstream header, contextual rails and safe editors.
- Shared bearer SSE: four regression tests passed.
- Installed Base UI contract: 38 route patterns; README structure and its three
  validator tests passed; Markdown links checked across 84 documents.
- Playwright: 18 fixture checks passed across Chromium/Firefox/WebKit;
  27 opt-in live checks require a running backend and are not acceptance evidence.
- Screenshot manifest verifies 135 fixture images at 375x812, 1920x1080,
  2560x1440. Desktop config and mobile Chats/transcript inspected visually.
- Standalone Rust 1.88 release and frontend Docker images built successfully
  with locked dependencies and explicit sibling Base contexts. A disposable
  clean PostgreSQL/Redis/backend/Nginx stack passed health, authenticated RBAC,
  private-chat denial, idempotent replay/conflict, immediate proxied session SSE
  and session persistence after backend restart. The disposable stack was removed.

CI now includes compile, migration rollback/reapply, frontend formatting,
route/link/screenshot gates, three-browser fixture acceptance and browser evidence
artifacts plus disposable authenticated container acceptance. Shared Base changes are published in
[services-base #121](https://github.com/FerrPOINT/services-base/pull/121).
Fleet CI pins Base to `af1bdd4746dfda331d0c32741af3f7c502fad816` until the
dependency is merged. GitHub CI status must be checked on the current PR head;
local checks alone do not prove CI acceptance.

The real seven-agent PM/requirements/decomposition/Rework/deployment scenario,
cross-service CI and production receipts remain unverified. Container acceptance
proves the standalone Fleet foundation, not actual Hermes/provider execution,
Central Auth integration or a complete automatic SDLC environment.

## Historical Baseline

Implemented:

- new `fleet-control` repository scaffolded from the React/Rust stack
- fresh fleet-control backend domain, migration and API skeleton
- `AgentKind = hermes | java_agent`
- separate agent product role: `leader | executor`
- agent profiles: `developer | tester | it_lead | custom`
- `SystemRole = admin | operator | user` with backend RBAC enforcement and a
  legacy `is_system_admin` alias
- local HMAC access tokens now carry fleet-compatible `aud`, `iss`, `role`,
  `scopes` and `sid` claims with strict issuer/audience validation for new
  tokens and a transition fallback for legacy compact tokens
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
- Java Agent template and existing externally provisioned jar lifecycle;
  chat/control/config activation remain phase 2
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
- read-only agent storage/retention reports for `runtime`, `config`,
  `workspace` and `logs` with marker status and purge eligibility
- fleet-wide storage/retention review for technical agent inventory with total
  managed bytes, archived bytes, purge-ready agents and marker/path issues
- full documentation baseline
- historical 82-file desktop screenshot set, superseded by the October manifest

Known local limitation:

- Native Windows Rust check is blocked until MSVC Build Tools provide
  `link.exe`.
- The earlier Docker engine hang is no longer the migration blocker: October
  tests ran against an isolated PostgreSQL 17 container. Full compose smoke has
  not been rerun.
- WSL/Linux backend check, clippy, tests and OpenAPI source regeneration pass.

Historical verified gates:

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
- `pnpm screenshots:local` with 82 generated desktop screenshots.
- `pnpm screenshots:verify`.
- `pnpm markdown:check` for `README.md` and `docs/**/*.md`.
