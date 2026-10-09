# Testing

## PM Tools Offline Conformance

The [offline runner](../scripts/pm_tools_conformance/README.md) requires an explicit
clean pinned Hermes checkout and executes 24 cases: 8 real Python extension/native
context probes plus 16 synthetic contract tests. It performs no model, network,
Docker, Cargo or activation; measured host Python dependencies are not a qualified
Hermes venv. The [handoff requirements](contracts/PM_TOOLS_HANDOFF_REQUIREMENTS.md)
retain producer pre-model barrier, identity and credential-custody blockers.
Offline PASS never grants task admission or proves production runtime readiness.

## Combined Steer Successor Inventory

The successor to `a26d8b35` with steer `71b17da7` requires all 130 default-ignored
cases explicitly, including 110 foundation cases. Default foundation retains
44 passed / 110 ignored / 0 failed. The shared `runtime_run_control::` CI selector
executes 25 cases: 18 original plus 7 `steer_transcript::` cases registered once
inside that module. The shared `runtime_terminal::` selector still executes14
once. All recovery, approval14 and 15/18 lineage cases remain required.
QA42 is frozen to its original source and is not acceptance for this successor;
new helper preparation waits for legitimate generation/readback and final source.
No Cargo, Docker or PostgreSQL acceptance was executed for this source merge.

## Combined Source Qualification

The controls/recovery plus approval14 integration must select all 123 ignored
cases explicitly (103 foundation), alongside 44 ordinary foundation cases.
Approval14 contributes 21 new cases: snapshot units 3, exact-pending unit 1,
approval recovery PG/HTTP 15, logical-clock PG 1 and migration14 PG 1.
The targeted-approval unit selector runs 2 cases including the inherited ACK
case. Journal runs 16; controls 18 and atomic terminal 14 run once each.
Migration14 requires isolated `FLEET_HERMES_TIME_MIGRATION_TEST_DATABASE_URL`.
Both 15/18 lineages, original-key recovery and strict generated OpenAPI cmp are
required. Source-only light checks do not imply compile, PG or native acceptance.

## Hermes Recovery Candidate

The [scoped release inventory](plans/2026-10-09-hermes-recovery-release.md) requires
14 terminal, 5 pinned-restart, 2 unknown-key, 2 PG-lock race and 7 framing cases,
plus 8 existing/extended runtime HTTP cases. They are ignored by default, require
`FLEET_TEST_DATABASE_URL` when selected and run serially against disposable PG.
CI gives each new family its own database and requires exact nonzero PASS counts
with no ignores; recovery-wire (3) and SSE-wire (10) unit selectors are explicit.
In the combined controls/recovery tree, the shared terminal selector runs once
in the controls database and requires 14 PASS, retaining all nine controls names
and five recovery additions. Recovery families retain their separate databases.
Existing journal (15), atomic ACK (11), GET readback (5), lineage and workspace
gates are retained. Local light checks are not Rust compilation, PG/HTTP/concurrency
execution or installed/native compatibility. No previous packet accepts this new tree.

## Approval Recovery Release Candidate

Unit14 has21 new named cases: snapshot unit3, exact-pending unit1, recovery
PG/HTTP15, logical-clock PG1 and migration1. Targeted unit group is2 including
its inherited ACK case; journal12 PG group is16. Required commands/envs and
remaining acceptance gaps are in [unit14 plan](plans/2026-10-09-approval-recovery-release.md).
CI explicitly selects ignored PG/migration cases, checks all exact new names
and counts, and runs the updated historical human/unknown-ACK HTTP case.
Lineage10, whole sorted SSE ledger including13/14, four-successor task-chat
rollback and all inherited gates are retained. These Rust/PG gates are prepared,
not executed locally. Loopback restarts do not qualify real Hermes processes.

## Steer Transcript Follow-Up

Seven new explicitly ignored PostgreSQL/HTTP regressions in
`runtime_run_control::steer_transcript::` cover original operator attribution,
redaction, exact session/run/receipt linkage, same-run distinct commands,
concurrent acknowledged replay/restart, legacy ACK repair, uncertain/rejected/
terminal-observed non-delivery, payload proof, audit/message rollback and
collision denial. The existing stop case also denies a control mirror.
CI now requires all 25 control cases by exact names/count, including these seven.
Only light checks ran at preparation; Rust compilation, these DB/HTTP tests,
strict Clippy, full ledger/SSE and exact-source Linux/native gates remain pending.
See [the follow-up gate commands](plans/2026-10-09-steer-transcript-release.md).

## Durable Runtime Controls Unit13

The [control release plan](plans/2026-10-09-runtime-controls-release.md)
originally specifies 32 focused cases: API1, native ACK3, control PG/HTTP18,
atomic terminal9 and migration1. Combined recovery integration expands that
same terminal selector to 14, not a second module or an extra nine tests.
New PG cases are explicitly ignored by default;
CI requires each name and exact success counts using separate disposable DBs.
Whole-ledger lineage/SSE, parent guards, strict Linux all-targets Clippy/check,
workspace, real Auth and generated OpenAPI parity remain required.
Formatting/static checks are not execution; all new Rust tests remain pending.
Current generated contracts/UI have not been promoted for the new API.

## Hermes Journal Unit12

The [isolated release plan](plans/2026-10-09-hermes-journal-release.md) names
mandatory journal, single-submission, atomic ACK, GET-only recovery, terminal
evidence and both-lineage migration gates. New DB cases are explicit opt-in;
default ignored results are not coverage. All exact-source Linux execution is
pending at source preparation, without any runtime-ready claim.

## Real Base Delegation Consumer Gate

The separate `infra` integration target `pm_credentials_real_auth` requires a
source-qualified disposable Auth built from Base01388df, not the in-process HTTP
stub. It exercises actual Fleet issuance/replay/conflict, strict introspection
wire and revoke. It is ignored by default; a default workspace PASS does not
accept this release prerequisite. Explicit execution fails for absent inputs:

```bash
cargo test --locked -p infra --test pm_credentials_real_auth -- --ignored --test-threads=1
```

The owned launcher supplies `FLEET_REAL_AUTH_TEST_OWNED=source-qualified-disposable`,
`FLEET_REAL_AUTH_TEST_SOURCE_SHA=01388dfb43332cbe5837fd5e1fadccf09cb8886d`,
`FLEET_REAL_AUTH_TEST_BINARY`, its `FLEET_REAL_AUTH_TEST_BINARY_SHA256`, and
`FLEET_REAL_AUTH_TEST_DATABASE_URL=postgres://fleet_test@postgres:5432/fleet_real_auth_test`.
The test owns the Auth process and an unexposed loopback port40000+, registers
a synthetic user through real HTTP, issues a disposable parent PAT, restarts Auth
with an exact-subject PM policy, and verifies Fleet against that process. It
accepts no external parent PAT or subject; credentials stay in test memory.
Do not use installed runtime secrets, publish ports or retain PAT values in
evidence. Pin/source/binary identity and cleanup belong to the launcher evidence;
an environment string alone does not attest the server. No Tracker request,
assignment claim, workflow receipt or full SDLC completion is asserted here.
The twenty-stage source-exported backend gate explicitly builds this Auth binary,
records its digest/source commit and requires this otherwise-ignored consumer case.
Packet4ed36ce828ea explicitly passed this real consumer against Auth01388df.
This is disposable source/binary compatibility, not installed-runtime acceptance.

The mandatory `real-base-auth` CI job uses a GitHub-managed Rust container and
isolated PostgreSQL service with no published host port. It exports committed
Fleet/SDK/Auth sources, builds Auth01388df separately from SDK19a7, records source
and binary hashes, and requires the otherwise-ignored consumer case with exactly
one PASS and zero ignores. It is not a local Compose project or installed-runtime
acceptance. This CI job depends on Base126 publishing the pinned Auth commit;
the workflow itself has not run yet.

## Persisted Credential Candidate

The [credential release](plans/2026-10-09-pm-credentials-release.md) requires
explicit PostgreSQL `sdlc_foundation` credential creation cases, ignored lineage
tests, `migration --test pm_credentials -- --ignored --test-threads=1` with its dedicated empty
`FLEET_CREDENTIAL_MIGRATION_TEST_DATABASE_URL`, historical message-order tests,
locked Linux workspace/check/strict Clippy and generated OpenAPI parity.
CI creates the dedicated database and runs the credential migration case.
Source preparation and formatting do not certify these pending gates.

The same packet passed15 of20 stages, including7 credential unit cases,10 real-PG
credential cases,47 foundation cases, workspace tests and10 lineage cases. Stage16
failed before SSE exercise because its inherited fixture expected11 migrations
after the additive credentials migration made12. The fixture now compares the
exact sorted ledger against registered canonical versions and requires the
credentials migration. This correction is formatted, not yet Linux/PG verified.
The credential-migration, smoke, OpenAPI and final parity stages did not run;
all10 independent cleanup/input parity checks passed and permanent runtime was
unchanged. Full acceptance waits for current-main reconciliation and a fresh gate.

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
