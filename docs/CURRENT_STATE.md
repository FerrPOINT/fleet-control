# Current State

Status (2026-10-02): SDLC foundation is partially implemented. Automatic SDLC is
blocked until cross-service assignment/workflow/deployment contracts are verified.
See [SDLC implementation](SDLC_IMPLEMENTATION.md). The baseline feature/gate lists
below are historical, not acceptance evidence for the new SDLC plan.

New implementation: seven specializations, agent-grouped `/chats`, read-only
directory, persistent bearer SSE replay, transactional prompt outbox, concurrent
idempotency, no effective-admin central bypass, versioned configuration activation
with drain/readback/rollback and fail-closed SDLC readiness.

Native Windows Rust commands still require MSVC `link.exe`.

The 2026-10-02 transcript follow-up adds immutable internal database allocation
order to legacy listing and paginated chat history. New messages do not reorder
when the clock moves backwards; overlapping pages preserve server order. Historical
records retain their former timestamp/UUID order, not recovered insertion order.
SSE reconnect no longer cancels an in-flight older-history fetch. A catch-up read
includes messages arriving during that fetch. This does not complete PM creation,
delivery, resume or live Backlog acceptance.

## PM Clarification Work In Progress

The creation recovery follow-up adds owner/key readback for a lost initial
acknowledgement, persisted-operation continuation without prompt resubmission,
and a rollout-filtered strict Tracker project directory. It adds no runtime
dispatch and does not close admission/resume/live acceptance. The new creation
form is an isolated [design proposal](design/PM_DRAFT_CREATION_PREVIEW.md), not
production UI; approval and controller integration remain pending.

The effective-configuration follow-up verifies actual managed files against the
active database snapshot on every readiness request. Same-size drift, missing
files, re-enabled disabled skills, foreign markers and symlinked paths block
readiness. Planning/readback is read-only; activation alone creates skill paths.
This is not a fenced admission, a runtime-loaded-config proof or task-workspace
claim. The preceding WSL Rust 1.88 workspace gate passed 118 library and 39 actual
PostgreSQL tests. The corrected bundled-inventory tree passed 118 library tests,
including oversized/non-file marker denial; its broader local rerun was
interrupted when WSL became unavailable. Exact-tree Linux CI subsequently passed
all five jobs; see the [merge gate evidence](CHAT_CLARIFICATION_VERIFICATION.md).
This does not close actual PM delivery/resume or live acceptance. Frontend
passed 211 tests including Russian/English readback warnings, typecheck, lint,
format and build. OpenAPI regenerated from source is unchanged. Three separate
directory/SSE/historical tests remain ignored in this local workspace run.
Controlled filesystem/HTTP evidence does not replace live PM acceptance.
The compatibility correction preserves Hermes-owned category directories and
`.bundled_manifest`; their presence is not managed-file drift or a native-skill
attestation. A distinct unverified runtime inventory blocker remains. The
controlled regression includes this layout without trusting its manifest.

The internal PM Draft chat repository operation now commits private chat, exact
binding, two participants and one audit/event atomically. It creates no prompt or
runtime run; a bound chat cannot use ordinary message dispatch. An opt-in public
Draft creation coordinator now persists an owner/key operation, reconciles Tracker
creation and initial PM reservation, validates the immutable original input and
creates that atomic chat. It ends at `awaiting_admission`, not a runtime launch.
PM admission, initial delivery and real Backlog acceptance are not connected yet.
The namespace follow-up now checks fresh trusted Workflow project ownership on
every creation continuation before external writes, including completed replay.
The dedicated read PAT stays server-only; exact issuer/provisioner/project checks,
bounded body/deadline and no redirect/retry/proxy fallback fail closed. This is
not a workspace/execution lease or admission receipt. The follow-up passed 115
library and 38 actual PostgreSQL cases with both DB variables configured; three
separately gated directory/SSE/historical tests remain ignored in this local run.
No new UI composition, public DTO or database migration was introduced here.
The coordinator follow-up passed 111 library tests and 37 real PostgreSQL cases;
the separate fresh-DB migration/backfill/down-up test also passed. These are
controlled repository/HTTP checks,
not a real human/PM/Tracker/Workflow acceptance.

The working branch is reconciled with accepted Fleet main `11a22c1` and pinned
Base `c083783a37791e277db796361203884b87828a7d`. On Rust 1.88.0,
Node 22.20.0 and pnpm 10.28.1, 111 library/37 real PostgreSQL tests and the
separate historical migration test passed. Frontend now has 209 passing tests
after receiving the accepted shared-library cleanup; 36 three-browser fixture
cases passed and 27 live cases were skipped. Nine controller captures were
regenerated. Package-consumer, effective-theme and generated OpenAPI/client checks
passed. The compatibility gate documents one intentional security retirement:
legacy run-wide approval now returns 409; other contract changes remain checked.
These checks do not complete PM admission, initial delivery or live Backlog flow.

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
The authenticated background metadata poller is connected behind an explicit
deployment flag (disabled by default). PM answer delivery remains unwired.

Fleet now implements strict bounded `metadata_v1` page decoding and transactional
format/version pinning. It validates all nine supported resource shapes, required
nulls, canonical non-nil UUIDs, safe versions, UTC source timestamps, source
digests and lossless decimal bigint cursors. Empty pages pin the format; changed
replay, stale cursors and implicit legacy conversion fail closed. The full WSL
gate passed 102 library tests and 29 actual PostgreSQL 17.11 cases. This storage
follow-up did not itself connect a poller or prove live PM delivery.

The polling follow-up checks the pinned machine subject and exact read-only
Tracker scope through Base on every cycle, then current Tracker project access.
Bounded GETs refuse redirects; failed or corrupt pages leave durable cursors
unchanged. Restart/replay cannot duplicate events or create a runtime dispatch.
The WSL gate passed 106 library tests and 31 actual PostgreSQL 17.11 cases.
HTTP authorization/dependency fault coverage uses test endpoints, not a live
Central issuer or PM. No accepted runtime or deployment secrets were changed.

The 2026-10-02 follow-up implements the server-only Base delegation client.
Commands derive the actual Tracker task/assignment/execution/agent/version grant;
responses must have exact scopes, a live bounded expiry and no-store protection.
Secrets are not serializable or debug-visible, and child authorization is bound
to the configured Tracker API origin. This does not yet issue credentials from
the PM coordinator or hand them to real runtime tools. The follow-up passed 100
Rust library tests, format, all-target check and strict all-target Clippy.
No new PostgreSQL or live provider acceptance is implied by that library gate.

Terminal readback now reconciles the matching Fleet run atomically, releasing
capacity without depending on a surviving SSE worker. PM cache updates cannot
invent terminal proof, change the accepted runtime ID or reopen a verified run.
This follow-up passed the full WSL suite with 100 library tests and 27 actual
PostgreSQL 17.11 cases, plus focused final-tree PM regressions. The two separately
ignored directory/approval-stream gates remain separate CI evidence. Workflow
PR #90 is reconciled with accepted master, but real PM admission remains blocked
until the actual pinned native-skills source and compatible build are available.

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
The old CI evidence used Base `af1bdd4746dfda331d0c32741af3f7c502fad816`.
Current CI/builds use the accepted SHA in [the Base pin](../.base-revision).
GitHub CI status must be checked on the current PR head;
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
