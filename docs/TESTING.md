# Testing

The central directory regression explicitly executes the real PostgreSQL query
with expanded and selected-foreign owner filters. It verifies that private rows,
counts and foreign cursors are denied together, while standalone legacy expanded
scope remains compatible. The API matrix covers all three historical roles and
both central service-write states; roles cannot bypass the private-owner filter.

The clarification checker resolves nested schemas and retains validation
constraints, including boolean schemas, maps, reference siblings and safe version
bounds. Its eight cases reject loosened schemas and refuse an incompatible
`--record` before changing the snapshot. After Rust generation, compare actual
published producer bytes with
`node frontend/scripts/verify-chat-contract.mjs --tracker <exported-Tracker-openapi>`
from the repository root. Record the producer commit and artifact hash; a local
snapshot match alone is not deployed contract or PM admission acceptance.

The screenshot helper mocks the current server-side directory, history,
task-context and chat-controls APIs. It refuses any unhandled fixture API route
and requires the list/transcript content before capturing Chats. A PNG count or
valid image dimensions alone must not turn a mock-error page into UI evidence.
The independent nine clarification/requirements controller images still use
their explicit fixture contract and `liveAcceptance=false` manifest.

Historical-lineage upgrade tests and central-profile preservation run explicitly
in CI after workspace tests, with `--include-ignored`/`--ignored` respectively.
Use an owned disposable PostgreSQL database; lineage cases create isolated
schemas. New task-chat upgrade acceptance exercises both accepted foundations
with populated runtime/config/outbox/transcript/event history and unchanged
users/deployment records. Populated down denial also covers a binding or creation
operation before its first message. The clock-rollback regression checks exact
bodies/allocated sequence after refusal, not just surviving message count.
Default ignored results do not prove this gate.

Delegated PM credential tests cover canonical bound-task operation allowlisting,
foreign/legacy paths, URL normalization and wrong method rejection, unsafe revision
numbers, owner/verifier actions, expiry, existing Authorization, scope mismatch and
no redirect/retry. The Base request retains its original five-field wire shape;
the private task restriction is not serialized. Client tests do not prove that
Tracker rejects direct bearer use; that requires separate receiving-service tests.

PM creation recovery tests cover persisted owner/key lookup after repository
restart, owner versus operator/machine/local identity, fresh project revocation,
unknown/invalid keys, strict continuation body (including array rejection before
HTTP), and unchanged chat/run state. Directory tests cover strict wire identity,
canonical nonnil UUIDs, sorted bounded pages, required null, foreign metadata,
keyset cursor and rollout-filtered empty pages. Client tests distinguish 404
from dependency/permission/conflict errors. The separate creation preview uses
fictional data and an isolated screenshot manifest; it is not live acceptance.

## PM Chat Slice

### Persisted Credentials (C11)

The extracted C11 candidate requires fresh exact-head Linux Rust1.88 locked
check/strict Clippy/workspace tests; prior combined branch results are not its
acceptance. Mandatory focused CI selects all7 `infra --lib pm_credentials::`
cases, the shared disabled/redacted config case and10 PostgreSQL
`infra --test sdlc_foundation pm_credential_creation::` cases with a required
owned `FLEET_TEST_DATABASE_URL`. Recovery cases include lost ACK, replay,
concurrency/payload conflict, parent/child revocation, expiry, strict journal
shapes, atomic audit rollback and exact named credential downgrade refusal.

Run `cargo test --locked -p migration --test pm_credentials -- --ignored
--test-threads=1` with separate empty
`FLEET_CREDENTIAL_MIGRATION_TEST_DATABASE_URL`. Run all10 lineage cases with
`--include-ignored`, historical message ordering and approval SSE with their
separate owned DBs. No missing env, ignored result or zero-match filter is PG
evidence. The complete foundation suite now includes the ten credential cases
as well as unchanged PR64 package/Workflow tests; preserve its exact package
checkout env and separate SDK pin.

The `real-base-auth` job builds exact Auth01388 separately from SDK19a, exports
committed sources, binds binary SHA256 and runs the explicitly ignored real
issuance/replay/conflict/introspection/revoke consumer once. It uses synthetic
credentials and an owned loopback server, never an installed runtime. Only
source/binary hashes and a fixed consumer result are uploaded; the private log
and exact disposable source/tmp/target directories are cleaned even on failure.
Auth-source qualification is not deployed Auth/Tracker or model admission.
Rust OpenAPI byte parity remains mandatory; C11 adds no public DTO/route fields
and intentionally preserves PR64's generated API and client artifacts.

Run `FLEET_TEST_DATABASE_URL` against an isolated PostgreSQL instance for the 56
`infra/tests/sdlc_foundation.rs` tests. Without that variable the tests skip and must not
be counted as database acceptance. Binding tests cover concurrent replay, ownership,
duplicate task/agent pair, immutable payload, once-only audit/event, scoped pagination,
binding/prompt races and message creation/replay/dispatch/final mirroring after 500 messages.
Atomic PM Draft chat cases cover concurrent actor/key replay, repository recreation,
payload collision, duplicate binding rollback, exact participants/audit/event,
no prompt/run/outbox, foreign or disabled owner and non-PM agent rejection.
`FLEET_MIGRATION_TEST_DATABASE_URL` separately enables the central-subject migration test.

`FLEET_MESSAGE_ORDER_TEST_DATABASE_URL` must name a separate empty disposable
database. Run `cargo test -p migration --test message_order -- --ignored --test-threads=1`
to check historical backfill, backwards clock timestamps, immutable identity order
and pending migration down/reapply without losing messages. CI creates its own
database for this gate. Downgrade/reapply is a QA exercise, not an order-preserving
production rollback. Foundation pagination tests also check foreign cursors and
legacy listing order. Frontend tests cover overlapping pages and SSE reconnect
during previous-page loading, including catch-up of messages arriving mid-fetch;
browser fixtures are not real PM acceptance.

Frontend commands: `pnpm test -- --maxWorkers=2` and focused Playwright
`pnpm exec playwright test e2e/fleet-control.spec.ts --grep "PM chat clarification" --workers=1`.
The latter uses fixture APIs with production controllers and all three browsers; it is
not live PM evidence. Publish the verified fixture images with
`node scripts/publish-chat-controller-evidence.mjs`. Use the configured canonical browser
origin consistently through SSO. Live tests require compatible Tracker, Workflow,
scoped PM runtime and trusted readiness verifier; see the plan/gap register.

`pnpm chat:contract` checks generated Fleet wire schemas against the pinned Tracker contract
and runs the checker tests. `pnpm chat:evidence:verify` verifies the nine controller images,
route/view/viewport identity and content hashes. These gates also run in frontend CI.

## SDLC Foundation Checks

`cargo test -p infra --lib effective_configuration` checks actual temporary files:
fresh success followed by same-size drift in every managed file, missing files,
re-enabled disabled skills, wrong snapshot/revision/marker, missing or foreign
workspace and Unix symlink denial. It also verifies that readback does not repair
files or expose resolved secrets. These are controlled filesystem checks, not
runtime-loaded configuration or PM admission evidence. The PostgreSQL HTTP case
`readiness_http_does_not_trust_database_only_effective_revision` checks the real
readiness handler with a DB-active revision but no installed files, plus operator
access and regular-user denial. Without the database variable it skips.
The filesystem regression also includes Hermes-owned category directories and
`.bundled_manifest`. These are preserved, not trusted as a provenance source;
the unverified runtime inventory blocker is separate from managed-file drift.

New regression coverage: PostgreSQL concurrent session/message idempotency and
private authorization; configuration drain/rollback state; unknown dispatch
capacity; fake Hermes HTTP EOF versus terminal readback and single response
mirroring; split-secret stream redaction; seven specialization and Chats routing;
SSO backend-role preservation and fail-closed permission checks.

Run database tests explicitly against isolated disposable PostgreSQL databases:

```bash
export FLEET_TEST_DATABASE_URL=postgres://USER:PASSWORD@HOST:PORT/fleet_test
export FLEET_MIGRATION_TEST_DATABASE_URL=postgres://USER:PASSWORD@HOST:PORT/fleet_migration_test
cargo test --workspace -- --test-threads=1
```

Without these variables, some legacy database test functions return early; a
green unit run alone is not PostgreSQL evidence. The `agent_logs` integration
suite instead fails if `FLEET_TEST_DATABASE_URL` is missing. It checks an
interleaved newer row using an agent-scoped PostgreSQL trigger, 64 concurrent
stdout/stderr writers, exact persisted/redacted acknowledgements and a failed
foreign-key insert without a phantom row. CI runs these tests in its ordinary
PostgreSQL workspace gate. The central-subject migration fixture uses a
fresh database. Fixture Playwright cases run on Chromium, Firefox and WebKit;
live cases require `SDLC_LIVE_QA=1`. Screenshots are fixture evidence, not a real
seven-agent PM/decomposition/Rework/deployment acceptance.

The managed-settings fixture changes themes through the shared account menu,
checks the selected radio item and preserves preview/apply/rollback assertions.
The removed standalone theme button is not an alternative control contract.

## Heartbeat Incident Regression

`backend/infra/tests/heartbeat_alerts.rs` requires `FLEET_TEST_DATABASE_URL`
pointing at a disposable PostgreSQL instance. An absent database URL fails the
fixture instead of returning a successful
test without executing PostgreSQL assertions.

Run explicitly:

```bash
cargo test --locked -p infra --test heartbeat_alerts -- --test-threads=1
```

Five cases cover actual canonical-kind persistence, acknowledged deduplication,
fresh recovery without a status transition, a new incident after recovery,
unknown/future/nonrunning retention, concurrent insertion identity, explicit
health recovery, and atomic rollback when the resolution audit fails. The audit
failure fixture installs a task-owned trigger restricted to its own agent; it
must never run against an accepted runtime database. UI tests check canonical
and legacy display labels. These are monitoring regressions, not Hermes/model,
PM workflow or full SDLC acceptance.

Chat/session acceptance scenarios `C-01` through `C-15` and their current
source-review gaps are defined in [CHAT.md](CHAT.md). Existing frontend unit
checks are not evidence of live runtime delivery or backend permission closure.

Backend checks:

```bash
cd backend
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --test-threads=1
```

Frontend checks:

```bash
cd frontend
pnpm generate:api
pnpm typecheck
pnpm lint
pnpm format:check
pnpm test
pnpm build
pnpm exec playwright test
pnpm screenshots:local
pnpm screenshots:verify
```

`e2e/platform-header-live.spec.ts` checks the installed Base header against real
Fleet/Central Auth: three routes, three themes and eleven widths (320-2560 px),
runtime services, keyboard/touch, native inert cleanup, drawer and central logout.
Run with `SDLC_LIVE_QA=1`, private `SDLC_QA_SESSION_FILE` and optional
`E2E_BASE_URL` (default `http://localhost:7742`) / `SDLC_HEADER_EVIDENCE_DIR`.
The dashboard localization smoke uses the same private session path. Neither
test publishes secrets, auth traces or browser video. Keep API mocks out of
this live gate; the component fixtures remain separate fast checks.

`e2e/leader-team-live.spec.ts` requires `SDLC_LIVE_QA=1` and the local
`.local/qa-session.json` from the workspace QA bootstrap. It creates an
isolated `qa-leader-team-*` agent, then archives it and purges only its own
files. Screenshots go to workspace `.local/screenshots` by default; set
`SDLC_CAPTURE_EVIDENCE=1` only when intentionally refreshing the committed
leader-team evidence images. `SDLC_CAPTURE_LEADERS_EVIDENCE=1` separately
refreshes the leaders directory evidence.

Required scenarios:

- create Developer Hermes and Tester Hermes
- create IT Lead Hermes and assign Developer/Tester executors
- verify `agent1` and `agent2` folder layout
- ensure distinct `HERMES_HOME` values
- reject path traversal
- reject absolute paths outside the configured agents root
- show storage totals and marker state before physical purge
- show fleet-wide storage review totals, purge candidates and marker/path issue
  states on the technical agents page
- start/stop/restart Hermes through a fake runtime command
- reconcile a tracked Hermes process that exits unexpectedly
- keep Java chat/control/config activation typed as phase 2 while preserving
  the existing jar lifecycle
- edit one agent's skills without changing another
- enforce `admin`, `operator` and `user` RBAC at backend routes
- direct executor session is private by default
- direct leader session selects itself as leader
- child executor session from a leader chat records parent and leader
- default session API filter returns the current user's sessions
- admin/operator multi-user filter can expand to all users
- normal users cannot read all users or expand session filters
- newly issued access tokens carry `aud`, `iss`, `role`, `scopes` and `sid`
- legacy compact access tokens without `aud`/`iss` remain accepted during the
  transition window
- access tokens with wrong issuer or audience are rejected without legacy
  fallback
- selecting a leader for an executor session requires `leader_executors`
- session and message idempotency replay returns the original row
- session and message idempotency conflict returns `409`
- create a mirrored session message and dispatch through the runtime boundary
- create a session and hand it off to another agent
- create a leader delegation and verify parent/child linkage
- list session participants from `/sessions/{id}/participants`
- create/list/cancel deployment jobs
- load/update runtime, ports, integrations and auth settings with redaction
- mutating runtime, leader, session, skill and config actions create redacted
  audit entries
- logs UI separates process logs, events and audit trail
- settings UI supports user role changes
- screenshot manifest is generated and contains the required viewports/routes

## Общая база

Подключение версий, границы контрактов и проверки описаны в [BASE_INTEGRATION](BASE_INTEGRATION.md).

## Agent Detail Live Acceptance

From `frontend`, run against an already running QA platform:

```powershell
$env:SDLC_LIVE_QA = '1'
$env:SDLC_QA_SESSION_FILE = (Resolve-Path ../../services-base/deploy/.local/qa-session.json).Path
$env:PLAYWRIGHT_BASE_URL = 'http://localhost:7742'
pnpm exec playwright test e2e/agent-detail-live.spec.ts --project chromium --workers 1 --retries 0
```

The default session-file location is the same Base bootstrap path; passwords
are never committed. The test uses real Central Auth and Fleet APIs, creates a
uniquely prefixed QA executor, never starts its runtime and archives only that
record through the normal API in `finally`. Archived history and managed files
are retained by the product contract; this test does not delete volumes.

All six tabs are checked at 375, 768, 1280, 1920 and 2560 px in light, gray and
dark themes (90 combinations), with full-page screenshots, keyboard navigation,
40 px tab targets, no document/tab overflow and no serious/critical axe issues.
Screenshots default to workspace `.local/screenshots/fleet-agent-detail` and
must be inspected before publishing selected evidence. Error/retry, pending
locks, failed-draft retention and invalid JSON are covered by the agent-detail
unit suite; live API failures are not simulated by this acceptance test.

`e2e/detail-layout-live.spec.ts` uses the same live flags and session path. It
creates only its own QA executor, leader and an empty session; no agent process
or message dispatch is started. Agent records are archived in `finally` through
the API. Session history has no delete API and is retained in the isolated QA
project until that project's explicit teardown.

The layout test measures actual rail width, position, grid gap and stacking
order on overview, workspace, leader detail and session detail at
375/768/1023/1024/1279/1280/1920 px, in all three themes (84 combinations).
It also runs axe and checks document overflow. DOM unit tests check semantic
landmarks and primary-before-rail order; they are not CSS geometry evidence.

## Browser Authentication Regression Gate

Run the frontend test, typecheck, lint, format and build gates before release.
The focused tests are `src/api/client.test.ts`,
`src/app/auth-boundary.test.tsx`, `src/shared/auth/store.test.ts`,
`src/widgets/app-shell.test.tsx` and `src/pages/sso-callback/index.test.tsx`.
They cover late successful/error responses, response-body races, concurrent
expiration, same-subject reauthentication, cache/draft removal, StrictMode,
pending/failed sign-out, permission subject mismatch and obsolete SSO completion.

Run `pnpm exec playwright test e2e/fleet-control.spec.ts` in Chromium, Firefox
and WebKit against the built frontend. These browser flows use mocked API and
signed SSO responses: they verify UI integration, not live Central Auth,
Hermes, workflow resume or autonomous SDLC acceptance. Live specifications
require the separately documented QA setup and are not covered by fixture runs.

Backend ownership and SSE revocation tests remain independently required.
Discarding an obsolete browser result must never be treated as permission to
resend an uncertain mutation. Recovery tests must preserve original command
identity and verify authoritative readback before allowing any new dispatch.

## Configuration Foundation Candidate Gates

Exact code `011afd9151c828279976aec5e6cf0a28b78c1f69` passed all 18 Linux
backend gates in packet `ba43ca138e39` (2026-10-09), with all nine parity checks
and independently empty cleanup. The publication follow-up is documentation
only, not a rerun on a new code SHA. Foundation47's prior evidence is not
validation of this delta. See [scope, counts and evidence](plans/2026-10-09-runtime-config-release.md#verified-backend-evidence).

After scope review and a task-owned commit, freeze/export sources from that exact
Git SHA (never copy `.local`, `target`, `node_modules`, credentials or backups).
Use SDK `19a7a381ae6dbea61a643bb96189e483fa64df5c` as the sibling dependency and
set `FLEET_TEST_BASE_PACKAGE_CHECKOUT` to a separately verified Git checkout/cache
containing `4b9b4c9297a13fb28a6ba2039af2f7cb719f2f58` and canonical Base
`remote.origin.url` (HTTPS or SSH as accepted by the production reader).
Set `FLEET_TEST_DATABASE_URL` and `FLEET_MIGRATION_TEST_DATABASE_URL` to the
owned disposable PostgreSQL database. Missing either DB or package input skips
acceptance cases and is not PASS. CI supplies both inputs explicitly.

Required Linux Rust 1.88.0 commands from `backend`:

```sh
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p api --lib routes::pm_runtime::tests:: -- --test-threads=1
cargo test --locked -p api --lib routes::sdlc_configuration::tests:: -- --test-threads=1
cargo test --locked -p infra --lib base_package -- --test-threads=1
cargo test --locked -p infra --lib effective_configuration -- --test-threads=1
cargo test --locked -p infra --test sdlc_foundation -- --test-threads=1
cargo test --locked --workspace -- --test-threads=1
cargo test --locked -p migration --lib lineage_tests -- --include-ignored --test-threads=1
cargo test --locked -p infra --test central_profile -- --ignored --test-threads=1
cargo test --locked -p migration --test message_order -- --ignored --test-threads=1
cargo test --locked -p infra --test chats_directory -- --ignored --test-threads=1
cargo test --locked -p infra --test runtime_approval_events -- --ignored --test-threads=1
cargo run --locked -p migration -- up
cargo run --locked -p migration -- status
cargo run --locked -p migration -- down -n 1
cargo run --locked -p migration -- up
cargo run --locked -p migration -- status
cargo run --locked -p api --bin gen-openapi
```

Compare generated OpenAPI byte-for-byte with the checked-in candidate; this
comparison passed for the exact code SHA above, including its two new paths/schemas.
Keep foundation47's existing isolated directory/approval, clean migration
up/down/reapply, transcript-clock and frontend compatibility/strict gates.
No new migration or lockfile change is expected. New tests include duplicate
Authorization zero-IO denial, exact scopes/current introspection, Workflow drift
and transport refusal, >100-revision exact-head reads, identity/drain/outbox fences,
concurrent preparation CAS, closed skill inventory and unchanged effective files.

Local QA must wait for the exclusive heavy slot and use Base ComposeHelper journal
v2 from the verified `SDLC_MAINTENANCE_BASE` User environment SDK, loaded first on
`sys.path`, with an owned temporary project and `with`/finally cleanup. Require
30 GiB free, exact disposable volume inventory and retained terminal/source/gate
and independently empty cleanup evidence; preserve external caches, immutable
sources and runtime/rollback data. Product SDK pin remains separate from this
maintenance-helper installation. The completed packet used a private owned helper;
the documentation-only follow-up does not rerun containers or backend gates.
Controlled owner fixtures do not prove live cross-service
credentials, installed Workflow v3 or physical Hermes admission.
