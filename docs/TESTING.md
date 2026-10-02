# Testing

## PM Chat Slice

Run `FLEET_TEST_DATABASE_URL` against an isolated PostgreSQL instance for the 37
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

Without these variables, database test functions return early; a green unit run
alone is not PostgreSQL evidence. The central-subject migration fixture uses a
fresh database. Fixture Playwright cases run on Chromium, Firefox and WebKit;
live cases require `SDLC_LIVE_QA=1`. Screenshots are fixture evidence, not a real
seven-agent PM/decomposition/Rework/deployment acceptance.

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
