# Testing

## Tracked Runtime Stop

`cargo test --locked -p infra --lib runtime::process_stop` covers actual Linux
child termination/wait, repeated stop and already exited children. The PostgreSQL
`runtime_stop_untracked_never_fabricates_stopped_or_releases_run_capacity` case
calls production stop/restart for untracked running/starting/degraded runtimes:
errors leave PID/status/desired state and the pending run/capacity hold intact.
It does not prove descendant/container termination or cross-instance reconciliation.

With `FLEET_TEST_DATABASE_URL` pointing to disposable PostgreSQL,
`cargo test --locked -p infra --lib runtime::lifecycle_tests` proves delayed start
rechecks drain after lock acquisition and actual configuration writes/readback
remain serialized until event persistence completes. The latter test holds a
real PostgreSQL table lock after file changes; a competing start must wait, then
return conflict without spawning a child. Seed skills are explicitly disabled in
this filesystem fixture, not accepted as installed skills with missing content.
The Java missing-jar regression verifies two failed starts leave the ready agent
and its intent/PID/timestamps unchanged; command validation precedes publishing
starting state, so a known pre-spawn rejection cannot create an ownership hold.
The Linux stopped-child/failed-DB-update case injects a PostgreSQL trigger error
after actual child termination: old SOUL bytes, recorded PID, journal and drain
remain held rather than treating the metadata failure as reconciled activation.

The `runtime_purge_http_` PostgreSQL/HTTP cases call the real purge handler and
supervisor: untracked archived PID returns `503`, drain returns `409`; marker and
files, archived runtime metadata and absence of purge success event/audit are
verified. Atomic purge ownership and descendant quiescence are not proved.

`cargo test --locked -p infra --lib runtime::readiness` covers hung/slow probes,
absolute deadlines including polling sleeps, immediate success, elapsed deadline
and redacted diagnostics. The integration filter `runtime_readiness_java_`
uses actual TCP and PostgreSQL: hung headers/body with concurrent stop, oversized
Content-Length/chunked bodies, malformed/non-UP JSON and ordinary UP. HTTP UP
alone remains degraded/untracked; tests verify preserved PID/intent/timestamps
and capabilities. `runtime_health_failure_` separately checks the Hermes failure
path. These transport/lifecycle fixtures do not prove native SDLC admission.

## Configuration Activation Journal

Scoped Linux checks: `cargo test --locked -p infra --lib runtime::activation_journal`
and `cargo test --locked -p infra --test sdlc_foundation config_revision_`.
Journal tests cover exclusive reservation, backup/expected hashes, protected mode,
drop/partial-file retention, exact acknowledgement, foreign/duplicate/traversal
paths, links, non-regular files, bounded reads and absent/disabled-skill semantics.
Existing PostgreSQL tests cover draining, failed rollback hold, identity fencing
and exact effective-head readiness. These are not process-kill fault injection
or an installed-runtime restart recovery acceptance; both remain required.

## Credential Acknowledgement Timing

`cargo test --locked -p infra --lib pm_credentials::tests` includes an actual
HTTP acknowledgement delayed six seconds before issuance. A legitimate requested
TTL remains accepted even though it exceeds request-start + TTL + skew; an
overlong acknowledgement after the same delay remains rejected. Each command
issues one HTTP POST only. Existing tests retain exact scopes, replay/revocation,
expiry, invalid payload, no-store, redirect refusal and secret-safe diagnostics.
The mock issuer is transport regression evidence, not live Base runtime handoff.

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
`config_revision_readiness_http_uses_exact_heads_without_trusting_database_only_files`
checks the real readiness handler with a DB-active revision but no installed files.
It creates 100 newer drafts: readiness still finds the effective head and an older
draft can be validated by exact ID. Activation of that non-desired revision returns
409 (not a false 404); another agent's revision is 404 and regular-user writes are
403. Neither validation nor failed activation changes the effective head or grants
SDLC readiness. Without the database variable it skips.
`config_revision_identity_guard_fences_rebind_active_runs_and_unknown_dispatch`
checks the real PostgreSQL repository: pending/running/waiting/stopping runs and
pending/dispatching/uncertain outbox entries block identity edits and workflow
rebind. Unchanged identity plus a metadata edit is allowed outside drain. A rebind
waiting on the agent row lock observes a newly committed drain and fails without
changing the agent or binding. This is a source concurrency test, not a distributed
assignment/config lease or native runtime acceptance.
The filesystem regression also includes Hermes-owned category directories and
`.bundled_manifest`. These are preserved, not trusted as a provenance source;
the unverified runtime inventory blocker is separate from managed-file drift.

`cargo test -p infra --lib base_package` also covers pinned effective readback
against the real local Git object cache (`FLEET_TEST_BASE_PACKAGE_CHECKOUT`),
extra native HOME skill denial, forged proof with matching disk files and missing
cache. `effective_configuration` checks bounded depth, unlisted/nested/case-aliased
files, symlinks and Unix sockets. Legacy extra categories remain preserved.
Backend CI checks out this exact private package into a separate cache and sets
the test variable; these tests must not silently skip there. SDK `.base-revision`
is independent and unchanged. Local environments must provide the same authorized
object cache to run the actual-pin cases; mock evidence is not a replacement.
Pinned Git reads use asynchronous subprocess IO with a five-second process
deadline, a ten-second whole-package deadline and bounded stdout. Batch stdin is
closed explicitly; stderr is discarded and timeout/overflow kills the child.
Three additional scoped cases check real Git stdin/size handling, overflow and
exit-wait timeout, and sanitization of failed process output. Process fixtures
use only synthetic text; these tests do not prove native runtime attestation.

`package_mapping_rejects_name_as_id_and_profile_workflow_or_catalog_drift` checks
canonical numeric IDs independently from Base namespace symbols and profile
declarations. The PostgreSQL/HTTP case
`base_package_workflow_mapping_requires_fresh_owner_readback_and_exact_frozen_fields`
uses controlled Workflow metadata and the actual pinned Git package to prepare a
draft through the real API. Changed profiles create no draft and cannot activate;
validation stores a specific blocker, restored mapping validates. Readback is
fresh and compares catalog/profile/version/ID fields; legacy catalog credentials
do not substitute for a missing dedicated PAT. This is source evidence, not a
live installed v3 catalog or native runtime admission.

`base_package_workflow_preflight_failure_releases_drain_without_changing_files`
starts the real background activator after a controlled owner outage. It verifies
failed/no-drain state, preservation of the previous DB-effective head and SOUL,
and new draft preparation after restoring the dependency. An unverified rollback
still remains drained in the separate config lifecycle regression. Transport
negatives cover redirect, encoding, oversized and duplicate-field replies;
automatic protocol retries are explicitly disabled. These tests do not attest
the fixture's legacy effective head as a loaded native runtime.

`cargo test -p api sdlc_configuration` checks fresh Base HTTP introspection using
controlled servers: exact subject/agent-specific scopes, revocation, broad or
duplicate grants, browser/legacy denial before IO, disabled configuration,
redirect/encoding/malformed/oversized response denial and sanitized errors.
These are source contract tests, not live Base acceptance or native admission.
The OpenAPI regression checks global operation-ID uniqueness, including the
configuration read and the distinct existing PM runtime callback.
The PostgreSQL case
`base_package_machine_readback_denies_database_only_effective_config_and_human_fallback`
calls the real machine route with a pinned DB-active revision but no installed
files: local admin cannot substitute for the PAT, another agent is forbidden,
and metadata alone returns `503` without leaking paths/content/credentials.
It creates 100 newer drafts and verifies that direct effective-head/pinned-revision
lookup still finds the active revision outside the bounded history window.

New regression coverage: PostgreSQL concurrent session/message idempotency and
private authorization; configuration drain/rollback state; unknown dispatch
capacity; fake Hermes HTTP EOF versus terminal readback and single response
mirroring; split-secret stream redaction; seven specialization and Chats routing;
SSO backend-role preservation and fail-closed permission checks.

`runtime::hermes_wire` and the scoped `sdlc_foundation::runtime_http_` group
verify exact terminal event/status identity and native completion flags. Actual
HTTP/PostgreSQL fixtures check foreign run readback, contradictory partial output,
subagent completion, cancellation requests and unsupported response aliases:
none may fabricate a reply or free the waiting agent's capacity. A valid terminal
SSE event and a valid EOF status read each persist one reply without a second
prompt POST. These fixtures are not installed Hermes, OS process-tree stop or
Workflow/Tracker business-completion evidence.

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
