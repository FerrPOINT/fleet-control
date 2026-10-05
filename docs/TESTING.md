# Testing

## Original Approval Journal

`runtime_approval_outcomes` in `sdlc_foundation` requires actual PostgreSQL via
`FLEET_TEST_DATABASE_URL`. Its seven cases cover concurrent single-use claim,
new repository readback, closed original/action matching, legacy denial, actor/
primary/terminal checks, direct SQL split-transaction/history denials, once/deny
concurrent completion, late cancelled history after revocation and injected
audit failure rollback. These are journal component tests, not native approval
sender/recovery acceptance. Existing targeted HTTP approval tests stay unchanged.

The ten `approval_outcome_http` cases in `runtime_control_outcome_http` exercise
the actual supervisor and exact-request API/middleware with authenticated HTTP
and PostgreSQL: once/deny concurrent sends, original body/headers before effect,
lost ACK/new repository, preflight/legacy denial, unknown/foreign witness, injected
DB ACK rollback, cancelled/terminal history after revocation, disabled/rotated
worker, normal ACK/readback races, ownership, public redaction and payload conflict.
Fixtures prove consumer behavior, not native plugin compatibility or an OS restart.

`approval_outcome_http_keyset` is separately ignored by the shared suite and must
run with its own empty `FLEET_APPROVAL_OUTCOME_KEYSET_TEST_DATABASE_URL`. Its 101
retained original contexts put the valid UUID strictly after the first 100 invalid
bearer scopes. The worker must settle it through original GET with zero POSTs.
CI creates that distinct database explicitly; the ordinary suite is not this gate.

`migration --test runtime_approval_outcomes` requires a separate empty disposable
database through `FLEET_APPROVAL_OUTCOME_MIGRATION_TEST_DATABASE_URL`. It checks
predecessor history/guard, up/down/reapply and refused nonempty downgrade. CI
explicitly creates that database; a missing URL/early return is not PG acceptance.

The separate canonical native `--scenario approval-outcomes` runs actual Hermes
terminal guards and owner-authenticated Fleet HTTP decisions with the committed
Base control plugin. It checks once/deny effects, one POST/native ACK/audit per
decision UUID, saved raw hash/store epoch, lost real HTTP ACK held through native
terminal completion, original GET recovery and unchanged context/history/replay.
This is not the synthetic HTTP suite, approval Fleet OS-death or a stage receipt.
Run legacy `approvals`, current `approval-recovery` and `control-restart` separately
as regressions when changing the common native observer/test helper. Exact
source/binary/log/cleanup evidence belongs in the verification ledger.

`--scenario approval-restart` runs three distinct Fleet OS processes with two
verified SIGKILLs and one surviving native gateway. The real tool effect pauses
at the model until both deaths; the second process must make a new original GET,
not use an old observation. The third mirrors terminal completion before the
held original witness is released. Assertions require one POST/native ACK/audit,
same gateway and immutable context/dispatch/terminal timestamps, one assistant,
owner replay and payload conflict. This is not safe descendant termination,
combined extensions, task admission or a PM/stage receipt.

## Combined Native Recovery Extensions

The separate `combined-controls` and `combined-recovery` scenarios install and
attest both committed Base plugins, lose the actual initial202 before control or
approval ACK loss, and retain three Fleet PIDs/two SIGKILLs/one gateway. Run lookup
is read-only POST; action outcome lookup GET. The controls QA roots are separate
so holding action lookup cannot block initial run recovery. One run POST and
original key/hash/native ID remain mandatory. Run `control-restart`,
`approval-restart` and `recovery` separately after changing their shared helpers
or the multi-plugin mounter. See the verification ledger for actual-source proof;
these tests do not enable task admission or installed flags.

## Exact Approval Context And Delivery Lock Order

`runtime_targeted_approval` uses journaled accepted runs, authenticated bounded
native status/capability fixtures and actual local JWT middleware. It checks
changed origin/credential/run/session context, legacy or terminal runs, unavailable
approval capability, foreign pending requests and exact HTTP200 JSON ACKs. An
uncertain durable decision cannot send again after capabilities recover. These
are component checks, not machine/task or loaded-generation admission.

`runtime_acceptance_readback_http::delivery_failure_locks_session_before_message`
uses an explicit PostgreSQL session-row holder and `pg_blocking_pids`, not elapsed
time as a concurrency barrier. While delivery actually waits for that session,
the holder must still lock its message with NOWAIT. The pre-fix candidate failed
with55P03; session-first delivery removes the inversion without retries or relaxed
unknown-acceptance assertions. The unknown acceptance HTTP test separately
requires one submission, immutable original journal and pending held capacity.

## Durable Runtime Controls

`runtime_control_outcome_http` exercises the opt-in actual supervisor sender and
GET worker against PostgreSQL and a bounded fake native server. It asserts that
context is committed before an effect, one POST survives caller replay, lost ACK
and DB commit failure recover by GET only, unknown/foreign/rotated context cannot
be adopted, and late ACK preserves terminal history after actor revocation.
These component cases do not certify real Hermes or Fleet OS-process restart.

The 101-record keyset case deliberately retains 100 invalid immutable contexts.
Run it separately with `--ignored` and its own empty disposable database via
`FLEET_CONTROL_OUTCOME_KEYSET_TEST_DATABASE_URL`; sharing a DB with timed HTTP
fixtures can delay unrelated workers. CI creates that database explicitly and
requires the named case to pass. Do not delete history or weaken timeouts to
make the shared suite pass. The actual native `control-outcomes` scenario and
its cleanup/source hashes are recorded separately in the verification ledger.

`scripts/native_supervisor_live/run.py --scenario control-restart` is the
separate real-Hermes gate for persisted control outcomes after Fleet process
death. It SIGKILLs the first two of three Fleet processes after real steer/stop
ACK loss; the next process must recover by GET only. It validates the actual
model barrier, original contexts/dispatch, one POST and audit per command,
same gateway PID and late stop ACK without rewriting terminal history. Run it
only in the harness's owned disposable Compose namespace; see
[process-death evidence](CHAT_CLARIFICATION_VERIFICATION.md#control-outcomes-after-fleet-sigkill-5-october-2026).
An ignored case or a PASS without plugin preflight/cleanup is not acceptance.

`runtime_run_control` PostgreSQL/HTTP tests exercise identical concurrent/restarted
replay with one native POST, semantic conflicts, unknown ACK hold, one submitted
claim, current actor revocation, foreign-session readback, atomic ACK/audit rollback
and terminal-only reconciliation. A raw run flag is insufficient proof. The
actual human HTTP group uses issued local HMAC JWTs and `require_auth`, real
handlers, an owned PostgreSQL DB and authenticated fake Hermes. It checks missing/
invalid auth, unrelated user, wrong session/run, missing keys, owner replay with
one POST per command, payload conflict, operator/admin receipt reads and revoked
user denial. A separate injected sessionless admin tests only the proof boundary:
no HTTP human header can create `VerifiedHumanSession`. It is not Central Auth
JWKS/session/PAT live evidence or assignment-scoped machine-control admission.
The isolated migration `runtime_controls` test needs its own empty disposable DB via
`FLEET_RUNTIME_CONTROL_MIGRATION_TEST_DATABASE_URL`; without it no PG evidence is
provided. It exercises additive upgrade, preserved legacy rows, empty down/re-up,
nonempty refusal and immutable identity/state guards.

The opt-in actual native `controls` case verifies real AIAgent ACK/status/interrupt,
stable command receipt replay/history and terminal readback. It is not native
per-command unknown acceptance, targeted approval, process-tree or PM evidence.
Playwright's unknown-control/reload scenario runs Chromium/Firefox/WebKit; its
three screenshots are published by `publish-runtime-control-evidence.mjs` and
verified with `pnpm controls:evidence:verify`. Fixtures never count as live runtime
acceptance. PM controller nine-image evidence stays separate.

The full 135-screen capture rejects unhandled mock API requests before saving an
image. JSON/Markdown manifests must agree on fixture scope, count, route, CSS
viewport, actual PNG size and SHA-256. Structural/hash verification does not
replace visual inspection; the earlier capture exposed missing chat endpoints
despite passing the former count-only verifier. See the verification ledger for
failed broad gates, clock observations and current exact-source evidence.

`pnpm screenshots:verify` also runs nine isolated Node tests: valid inventory,
changed bytes/hash/dimensions, duplicate paths, count drift, false live scope,
missing mobile route and path traversal. Their synthetic PNG headers test the
verifier only; they are not captured images or browser acceptance. Every required
route must exist at each required viewport. Temporary fixture directories are
removed after each test.

## Bounded Native Stream Profile

`runtime::sse_wire` exercises incremental UTF-8, BOM, LF/CRLF/CR, empty event
reset, incomplete EOF, malformed bytes, frame/input/event counters, transcript/
snapshot budgets and deadlines that traffic cannot extend. The PostgreSQL/HTTP
`runtime_stream_bounds` group covers wrong MIME/encoding, malformed/foreign
control payloads, chunk-split Unicode, truncated terminal frames, oversized
frames/text and actual30s partial/60s idle timers. Assertions retain one original
run/session/key and held capacity, never a fabricated reply or another POST.
With no owned `FLEET_TEST_DATABASE_URL`, these tests skip rather than prove PG.
Full regression, native compatibility and release evidence are kept separately.
The [profile](contracts/HERMES_EVENT_STREAM_V1.md) lists limits and recovery scope;
native missed tools/approvals and upstream durable replay remain open.

## Managed Native Supervisor

The opt-in [owned Compose harness](../scripts/native_supervisor_live/README.md)
runs `infra/tests/native_supervisor_live.rs` against two real pinned Hermes
gateway CLI/API/AIAgent processes, disposable PostgreSQL and a deterministic
loopback model. It uses Fleet's provisioner, installed skill content, versioned
configuration activation, supervisor and prompt outbox, not a fake Hermes server.
Assertions cover distinct HOME/workspace/ports/SOUL, non-root private dotenv,
foreign-token denial, one original prompt/assistant mirror, native restart
readback and tracked parent stop. Native source files are compared with exact
Git archive hashes before inference; compiler JSON pins the executed test binary.

This ignored target must explicitly run one named test. Ordinary workspace tests
do not execute it. Host CI runs twenty-one harness/QA-fault unit tests without Docker;
these are not native acceptance. Build/native logs, exact hashes and independent
post-cleanup container readback are saved under ignored `tmp/`. There are no
installed runtime, accepted image, migration, Base SDK pin or production UI changes.
The separate `--scenario recovery` case uses two actual Fleet subprocesses and a
QA platform plugin that drops only the real native `202` acknowledgement. Lookup
is held until the first Fleet process exits and native terminal GET succeeds.
The second process must restore the same run through the committed Base witness
plugin, retaining request bytes/hash/key/context/horizon and one assistant mirror.
Native observations require one POST/inference and no SSE. This does not certify
native gateway crash, running/approval recovery or orphan safe-stop transfer.
Exact bytes/logs and the initial fixture failure are in the verification ledger.
Waiting tools/approvals crash recovery, complete loaded-config inventory,
descendant quiescence, central Fleet auth/UI and task/PM admission remain distinct
gates. Never substitute this chat happy path for a Workflow stage receipt.

`--scenario approvals` runs real terminal approval guards and exact-action
decisions through the actual local JWT middleware/HTTP routes. Three separate
chats exercise `once`, `deny` and losing a real successful native ACK. Check the
owned target's actual file mode, one decision POST, original run/dispatch identity,
one final assistant message (tool events are distinct), immutable replay transcript,
same-key replay and changed-payload409. The lost ACK must remain `uncertain` after
terminal and must never resend. Owner/stranger denial and unsupported `always`
choice are checked before runtime IO. The QA observer never manufactures a
request, approval, native result or ACK. This does not prove central credentials,
task-specific approvals, native outcome lookup or a Fleet restart while waiting.
The exact named ignored case and all21 host safety units are documented in the
harness README; historical failures and passing binary hashes stay in the ledger.

Prepared restart recovery coverage lives in
`backend/infra/tests/support/runtime_prepared_recovery.rs`: two fresh supervisors,
one original body/model/options/key/run, atomic uncertain-outbox permit, unknown
ACK hold without resend, changed credential/protocol denial and queue exclusion
for drain/task/failed/expired/submitted records. Native/Fleet managed acceptance
remains separate from these PostgreSQL/HTTP fixtures.

## Original-Key Recovery

`runtime::recovery_wire` unit HTTP cases verify closed facts/receipts, original
scope/epoch/deadline denial and zero native submissions during lookup. Disposable
PostgreSQL `runtime_unknown_recovery` verifies original epoch header and bytes
before the first POST, lost acknowledgement recovery, atomic mapping/mirror and
expired/changed context denial. `recovery_race` adds actual lock barriers for
concurrent ACK/lookup acceptance and deadline expiry while waiting on the journal.
Run with `FLEET_TEST_DATABASE_URL` using `--test-threads=1`; no env means skip,
not PostgreSQL evidence. The separate native
[recovery harness](../scripts/hermes_protocol_live/README.md) uses real pinned
hook/API/AIAgent/SQLite and a deterministic loopback model, not fake Hermes.
Base's 56 stdlib store/boundary tests include Linux symlink checks. These scopes
do not prove installed managed gateway/Fleet end-to-end or PM admission.

## Atomic Terminal And Pinned Recovery

With disposable `FLEET_TEST_DATABASE_URL`, run these serial PostgreSQL targets:

```bash
cargo test --locked -p infra --test sdlc_foundation runtime_terminal -- --test-threads=1
cargo test --locked -p infra --test sdlc_foundation runtime_pinned_recovery -- --test-threads=1
```

Terminal tests cover concurrent first commit/replay, immutable identity/outcome,
assistant/prompt/run database faults with full event-cursor rollback, historical
partial mirrors, empty output, failed/cancelled, drain and task/PM boundaries.
Late delta/tool/approval tests verify no writes after terminal state. Pinned
recovery starts fresh production supervisors after the journal/ACK/pin commit,
with actual PostgreSQL and controlled authenticated HTTP: zero POST/SSE, held
capacity on invalid status, subsequent terminal readback and UUID-keyset fairness.
These are source fixtures, not authentic Hermes or PM admission acceptance.
Results are recorded separately in the verification ledger.

Three deterministic PostgreSQL blocking barriers cover run/prompt progress and
the actual approval reservation FK path while terminal commit waits. Root-only
terminal output is tested against absent/empty/whitespace responses with nested
tool metadata. Recovery fixtures enumerate all bounded keyset pages and use
fresh ordered UUID ranges; the full shared-database suite must pass after the
targeted subsets, not just when each test starts with an empty database.
The SSE/status fixture reserves only its own pending prompt while its agent is
Ready, restores Running and calls production `send_message` once. Exact bearer
checks protect all run routes before counters. Background dequeue is covered by
the separate production dispatch and journal HTTP scenarios; no terminal,
capacity or ten-second SSE-state assertion is removed by this setup isolation.

## Versioned Native Hermes Renderer

`configuration_snapshot_tests` preserve omitted/explicit v1 serialization,
reject unknown versions, malformed native objects and conflicting API aliases,
without changing legacy validation. Infra verifies legacy bytes/readback remain
unchanged while v2 seals API/env/CORS and writes versioned hash markers. The
PostgreSQL foundation case proves server-selected Hermes2/Java1 and immutable
historical snapshot shape, including failed-draft validation.

For an actual native loader check, explicitly run the ignored Rust exporter and
the [renderer scenario](../scripts/hermes_protocol_live/README.md#actual-rust-renderer-scenario).
It consumes real generated files in two independent native processes; no installed
runtime is touched and no inference is performed. Host tests prove YAML-layer
exceptions cannot silently become an env-fallback PASS. This is narrower than
loaded effective-config attestation, gateway lifecycle or automatic SDLC acceptance.

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

`cargo test --locked -p infra --lib configuration_disk::tests` covers nested
directory creation, replacement/deletion, Unix0600, missing paths, non-files,
outside-root and symlink denials. Task-local test-only fsync failure injection
checks that visible rename/unlink still returns unavailable when the directory
barrier fails. The PG lifecycle case
`failed_directory_barrier_preserves_activation_drain_journal_and_effective_head`
injects failure after visible config rename, then verifies retained journal/drain,
no effective head, no runtime spawn and no second activation claim. These are
component failures, not a physical power-loss simulation or OS-safe-stop proof.

## Credential Acknowledgement Timing

The delayed issuer test measures its six-second wait with `Instant`, then
compares the returned expiry to the fixture's exact post-delay issuance/expiry.
It does not infer elapsed time by subtracting two Docker VM wall-clock samples.
The extracted production expiry predicate preserves the same receipt-time +
requested TTL +5s ceiling and fresh expired-token denial. A deterministic UTC
boundary test checks the exact ceiling, +1ns rejection, expired/equal rejection
and that using request-start rather than receipt time incorrectly rejects the
delayed valid ACK. No expiry/clock-security policy or retry is relaxed.

For actual producer interoperability, use the opt-in
[locked Base/Tracker harness](../scripts/pm_credentials_live/README.md).
It runs a separate ignored target against real service binaries; it does not
replace the persisted coordinator/fault-injection suite below or native PM
acceptance. Exact refs, hashes, cleanup and limits are in the verification ledger.

With disposable `FLEET_TEST_DATABASE_URL`, acceptance recovery is covered by
`cargo test --locked -p infra --test sdlc_foundation runtime_acceptance -- --test-threads=1`.
These cases require actual PostgreSQL plus controlled HTTP runtimes: atomic ACK
rollback/replay, immutable session pin, one concurrent winner, held capacity,
task/PM boundary, restart GET-only recovery and advancement past over 20 rejected
ACK readbacks. `runtime_task_protocol` also rejects an unadmitted task despite
valid durable capabilities before prepare/POST. They are not live Hermes tests.

With disposable `FLEET_TEST_DATABASE_URL`, run
`cargo test --locked -p infra --test sdlc_foundation pm_credentials_pg_ -- --test-threads=1`.
The audit-failure regression injects failures at both intent and ACK audit
INSERTs: the same transaction must roll back the corresponding journal update.
After ACK failure, original-key replay must recover the same issued child and
commit exactly one intent and one acknowledgement audit event.
Tests call the production optional creation port/coordinator, persist real PG
intent/ACK/audit and use actual HTTP fixtures for Base/Tracker. They cover lost
ACK and coordinator recreation, partial success, concurrency, original-parent/
origin/TTL conflict, stale/foreign/noncanonical context, revoked parent/child,
real short TTL expiry, first-write strict SQL shapes, immutable records, once-only
redacted audit and refused downgrade. No model/run/lease/Workflow mutation occurs.
They do not run real Base/Tracker servers or prove native admission/tool handoff.

`FLEET_CREDENTIAL_MIGRATION_TEST_DATABASE_URL` names a separate empty disposable
database for `cargo test --locked -p migration --test pm_credentials`:
predecessor up, populated legacy operation, additive up, unchanged bytes,
empty-journal down and reapply. CI explicitly creates that database and runs the
test. Historical transcript regression now rolls back two migrations to retain
its original 000010 coverage; neither test is production downgrade guidance.
Without the corresponding DB variables these cases return early and are not
PostgreSQL evidence.

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

The `sdlc_foundation::hermes_dispatch_journal` group exercises exact model/options
bytes, atomic reservation/fault rollback, concurrent one-winner submission,
immutability, finite horizon, original scope, drain/capacity/task-PM denials and
sanitized SQL diagnostics. Its late-error regression observes prepared, commits
submitted, then applies the stale failed update: delivery stays pending and the
real ACK can still atomically commit. A still later error cannot erase that ACK.
The production-adapter HTTP regression inspects the submitted journal before
receiving POST, corrupts 202 and verifies that another send creates no second POST.
Known-ID worker fixtures require original journal; legacy, changed origin and
rotated credentials perform no status HTTP. These are controlled HTTP producers,
not native Fleet/model/PM acceptance.

Migration target `hermes_dispatch_journal` requires a separate empty disposable
database via `FLEET_DISPATCH_JOURNAL_MIGRATION_TEST_DATABASE_URL`. It snapshots
pending, accepted-unpinned and completed legacy runs/messages/outboxes across
upgrade, empty downgrade and reapply; a populated journal refuses downgrade.
CI creates this database and explicitly executes the target. This does not
prove unknown-key lookup or store continuity in native Hermes.

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

## Native Hermes Protocol Gate

The opt-in [native acceptance harness](../scripts/hermes_protocol_live/README.md)
runs the pinned upstream API adapter and real AIAgent in separate processes,
with its native auth/profile middleware and SQLite. Only the upstream model is
a deterministic local fixture; the Hermes HTTP server is not mocked. No paid
provider, installed agent, runtime snapshot or persistent volume is used.

```powershell
python -B -m unittest discover -s scripts/hermes_protocol_live -p test_harness.py -v
python -B scripts/hermes_protocol_live/run.py `
  --hermes C:/git/azhukov/sdlc/прототипы/hermes `
  --image sdlc-fleet-canonical-runtime:20261003-r1
```

Host safety tests are independent of Docker and run in the docs CI job. Native
acceptance additionally requires the exact clean Hermes source and an existing
Base-packaged dependency image for its pinned lockfile. Source/image/log/harness
hashes and exact cleanup results are saved under ignored `tmp/`. A failed or
timed-out run is not acceptance. Native protocol evidence does not prove Fleet
unknown-acceptance journaling, managed gateway CLI lifecycle, model-provider
quality, tool approval, native configuration attestation, PM admission/resume,
OS quiescence or autonomous SDLC. Current results belong in CURRENT_STATE and
the clarification verification ledger, not in fixture screenshot manifests.

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

## Current Approval Recovery

`runtime_pinned_recovery` covers concurrent supervisors, stable request/event
replay, transaction rollback, redaction, resolved/stopping/terminal preservation,
missing capability, foreign identity and legacy/task/PM denials. The GET snapshot
parser has three bounded/exact-shape units. Native `--scenario approval-recovery`
uses two distinct Fleet processes, one real gateway and its real terminal approval,
with only the model loopback fixture and explicit transport loss. It verifies
original run/session/request, one owner POST/tool effect and one final answer.
See [execution and limits](CHAT_CLARIFICATION_VERIFICATION.md#current-approval-snapshot-recovery).
This is not complete historical replay, unknown decision resolution, central auth,
PM publication/resume, OS-descendant or seven-agent acceptance.

## Journal Clock Ordering

`journal_clock_regression_keeps_logical_progress_without_renewing_horizon`
injects a persisted future creation time with the original recovery horizon.
Submission/ACK must preserve logical order, exact replay and a single run/permit;
nonempty downgrade must fail without removing the journal or trigger.
Migration target `hermes_journal_time_order` requires its own empty database via
`FLEET_HERMES_TIME_MIGRATION_TEST_DATABASE_URL`. It verifies original history/
guard preservation, actual trigger order and empty down/reapply. Running without
that variable is not PostgreSQL migration evidence. These checks do not certify
host clock stability or safe unknown redispatch.
