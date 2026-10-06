# Chat Clarification Verification

Date: 2026-10-01. Status: verified foundation, incomplete approved vertical slice.
No real PM publication/resume or live Backlog acceptance is claimed.

## Accepted-Main Reconciliation And Safe Downgrade (2026-10-06)

Release source `5240107596ce9b645a4e30cd9b9506ecfc739905` is reconciled by a
normal merge with accepted main `3c6b8ef7bdb08f799ca30e0a5d7537914ce40ab6`.
The accepted Base SDK is `cbb4e99230420dc2659431b1c9fb5090e5c940f0`.
Verified display names retain both human-session and central-subject proofs.
All fourteen historical main migration blobs are preserved. Only pending task
chats000010 is appended, giving eleven canonical or fourteen split entries;
later runtime integration migrations are not part of this release.

A real PostgreSQL regression reproduced destructive populated downgrade:
the old `down` succeeded, deleting task metadata and allowing transcript
allocation order to be rebuilt from a clock-regressed timestamp. The pending
migration now locks its history tables and refuses downgrade before changing
schema or ledger if any transcript, binding, creation operation, approval,
PM run, inbox event or projection cursor exists. Empty downgrade/reapply still
works. Populated upgrade tests cover both accepted foundations, preserve
runtime/config/outbox/event/user/deployment records and do not bind legacy
chats by title or task key. Separate tests retain an unmessaged binding and
a creation operation. The migration's `up` SQL is unchanged.

Final Linux Rust1.88/PostgreSQL17.6 gate: **179 distinct component cases passed**.
The serial workspace suite passed164 cases. Explicit opt-in gates passed
nine additional lineage cases, three central-profile cases, and one each for
historical transcript ordering, scoped directory and authenticated approval
SSE. The lineage registry unit test ran twice and is counted only once.
No opt-in case is claimed from an ignored test or missing database URL.
Locked all-target check, strict Clippy, fmt, clean migration CLI up/status/
empty down/reapply/status and byte-exact Rust OpenAPI regeneration passed.
CI now explicitly executes the lineage and central-profile opt-in gates.

| Owned evidence | SHA256 |
| --- | --- |
| Final backend project `sdlc-qa-fleet-chat-release-615ca6a67d74` log | `5293d3bf18dc0844bbb8914ba7dfab8e26a40fafd33fcf69e9948e8c511f4145` |
| Pre-fix populated-down regression `d4f2cc6d199c`, expected failure | `96c55b19ed6b778b89b155bdd44f650f32dbe7bbe54dec8d306a3d1e065d9805` |
| Earlier baseline gate `3a86122ea4f1`, before guard | `e58a519f9e3f2e1ccf7ec094894b5683f760a00cba1143e674a187e30e6fbe40` |
| Preliminary compile failure `eddcf3faa07e`, corrected borrowed DB helper | `63f5b29ef77a30fc9541976f4fc7aaa43b8a7186ffa5e3306f4b29da6833a9d0` |
| Final canonical-origin three-browser fixture log | `361ff4734254ff34866206ce71987cbb5f57e92f054213dbdd26929f173ea307` |

Logs are retained in the owning local release QA directory; the final run,
not preliminary successes, attests the guarded migration bytes. Every listed
Compose project was removed in `finally` and independently read back empty.
Docker grouping audit returned `complete=true`, desktop42, both runners0,
and no violations. Accepted runtime images, volumes, pins and flags were not
changed. These counts describe that observation, not a permanent inventory.

Node22.20.0/pnpm10.28.1 frozen installation, accepted SDK shared-UI verification
(38 route patterns), generation/drift/compatibility, typecheck, **228 Vitest
cases**, lint/semantic classes, formatting and production build passed.
The same release-generated seven PM wire schemas also match actual Tracker
PR114 source `8c80a41fae3bf1c10439ddb7e536b05bf320340d`, read directly from its
Git OpenAPI blob, without changing the accepted snapshot or that repository.
This is schema compatibility, not deployed producer/admission authority.

The final canonical `http://localhost:4187` preview passed **36 fixture cases**
in Chromium, Firefox and WebKit; **27 opt-in live cases were skipped**. The
earlier QA wrapper used `127.0.0.1`, while central login canonicalizes to
localhost, losing fixture local-storage identity after navigation. Correcting
only that wrapper origin made the gate pass; product/test assertions were
not weakened. The fixture logs still contain unmatched legacy API proxy
failures because no live backend was started; they are not a no-network-error
or live integration acceptance. Preview exited with the test process.

Desktop clarification and mobile dialogue captures were visually inspected;
no obvious overlap was seen. Existing135 screenshot entries and nine controller
fixture hashes were verified, not relabeled as live screenshots. New browser
captures remain local fixture artifacts. Markdown links and README validation
are rerun after these documentation edits. Exact published-head CI must be
checked independently; older green checks do not attest this merge.

The PR remains Draft: real fenced admission/first-step, PM structured tools,
answer delivery/checkpoint/rebind, owner-confirmation-to-Backlog, producer
readiness and seven-agent/deployment acceptance remain open. The newer runtime
integration branch has separate evidence and is not certified by this gate.

## Directory Query Review Follow-Up (2026-10-03)

Owner IDs retain their existing JSON SQL parameter, but membership converts
the list to a UUID array once and uses `user_id = ANY(...)`. This avoids the
set-subquery estimate that selected two full session scans for the normal
owner scope. No dependency feature, HTTP field or database index changed.

Rust 1.88.0 with the exact pinned Base `9408802` passed fmt, strict workspace
Clippy, locked workspace tests and the separate real PostgreSQL directory/SSE
gates. Directory regression covers mine, multiple owners, read-all, literal
search, count consistency, cursor pagination and fresh project ACL changes.

A disposable PostgreSQL 17.6 database copied the actual 11-migration schema
and inserted 500000 synthetic unbound sessions, 1000 owners and 20 agents.
The exact fixed query used the existing user/session index and page PK lookups
with zero session Seq Scans for mine and multiple-owner cases. Before/after:
143.445/8.506 ms for mine and 188.951/10.234 ms for multiple owners.
All-users count retained its full-scope scans. These are controlled query-plan
measurements, not production endpoint latency or live bound-task acceptance.
See [plan summary](evidence/chat-directory-scale.json).

The full feature remains Draft. This component fix does not close PM
admission, runtime handoff, resume or owner-confirmation acceptance.

## Credential Confinement Follow-Up (2026-10-02, Component Verified)

The local branch is reconciled with accepted main `d5697cd`; its Base pin is
`9408802dfa978cba2f67162a49adca6f65851b01`. The client change restricts
delegated PM credentials to enumerated operations for the canonical assigned
task, without changing the five-field Base delegation wire. It is not connected
to runtime handoff, admission or dispatch.

Completed after environment recovery: fresh-target Rust 1.88 locked all-targets
check and strict Clippy, format checks, full serial workspace tests, and byte-exact
Rust OpenAPI regeneration. Passed: 121 library tests and 38 actual PostgreSQL
17.6 cases. The central-subject migration test returned early without its separate
DB URL; it is not included in the PostgreSQL count. Three separately gated
directory/SSE/historical migration tests were ignored locally; CI executes them.
The passing QA run is `credential-qa-20261002-04c0cf46`.

Exact pinned Base frozen frontend passed typecheck, 220 tests, lint, format,
source-client contract/compatibility checks and production build. Source comparison
matched all 128 tracked frontend/OpenAPI files in the isolated consumer, excluding
deliberately omitted env files. Base frontend has no source changes between the
previous and new pin. README validator, its three tests and links across 91
Markdown documents passed. No frontend composition or public schema changed.

The earlier disk/daemon failure is superseded by these local gates. Old owned
containers were confirmed absent after recovery; both successful-run containers
were removed in the QA finally block. No runtime, shared volume, secret, snapshot
or image prune was performed; evidence and target artifacts are retained.

The previous CI run
[37019857586](https://github.com/FerrPOINT/fleet-control/actions/runs/37019857586)
attests only `250457540bc961ab7c07463448734593a2ecc3ab`; the follow-up requires its
own exact-head CI. Client HTTP fixtures do not prove direct bearer enforcement
in Tracker or genuine Base delegation/runtime handoff. The full PM slice remains
incomplete and Draft; admission/tools/resume/verifier/live acceptance are not
closed by this component verification.

## Creation Recovery Follow-Up (2026-10-02)

Owner/key readback recovers the original creation UUID after a lost response.
Persisted-operation continuation accepts only an empty object, rechecks current
human/project/namespace/rollout authority and does not accept edited input or
dispatch a run. Tracker's separate strict project directory provides UUID-keyset
pages of actual ID/key/name metadata; Fleet retains the source cursor after its
rollout filter, including empty pages. There is no legacy directory fallback.

Fresh Cargo target, Rust 1.88.0 locked fmt/check/strict Clippy and full workspace
tests passed: 120 library cases and 39 actual PostgreSQL 17.6 cases. Three
separately gated directory/SSE/historical migration cases were ignored in this
local run. The final empty-object OpenAPI schema annotation also passed a
separate fresh-target fmt/strict API Clippy/34-case API suite and source
generation. The generated object forbids additional properties; arrays and
fields are rejected by the actual handler. The client was regenerated from it.

Node 22.20.0/pnpm 10.28.1 typecheck, lint, format, build, OpenAPI compatibility
and 220 unit tests passed. All 36 Chromium/Firefox/WebKit fixture cases passed;
27 opt-in live cases were skipped, not accepted. The separate
[creation proposal](design/PM_DRAFT_CREATION_PREVIEW.md) has 16 generated screens
across mobile/tablet/desktop/wide sizes; mobile form and desktop recovery were
opened for visual inspection. Capture detected no API/external requests or page
errors and no document-level horizontal overflow. This is an unapproved design
proposal, not live production screenshots. Browser fixtures do not close actual
admission, assigned PM tools, checkpoint/resume or exact-revision Backlog acceptance.

## Merge Gate Review (2026-10-02)

Exact-tree Linux CI [37000005352](https://github.com/FerrPOINT/fleet-control/actions/runs/37000005352)
passed all five jobs for `65c2f7469f9a4cca05e4d037cdee55c8a3cca410`,
including backend/migrations/historical ordering, generated OpenAPI, minimum
Rust, container smoke, frontend and three-browser fixtures/screenshot manifest.
This closes the corrected-tree CI verification left pending by the interrupted
local WSL run. It does not turn controlled runtime/browser fixtures into live PM
acceptance. Documentation-only follow-ups must also pass their own CI.

Tracker [37005023205](https://github.com/FerrPOINT/task-tracker/actions/runs/37005023205)
passed all four jobs for `c09af5a0803bbeea0eb5f4e975917ce74ac2ab4d`.
Its backend explicitly executes the isolated PostgreSQL Draft/reservation/lease
and clarification suites. The lease is implemented, but Fleet has not consumed
it for runtime admission: its receipt remains `dispatch_allowed=false`.
Workflow [36969920135](https://github.com/FerrPOINT/project-workflow/actions/runs/36969920135)
passed both jobs for `0401af1635ad8af5e7a2b32b7dcdc659c05cc65d`.
Base's previous green result does not validate a later rebase; the updated
delegation branch requires fresh checks against accepted main.

All four PR bodies, conversation/review/inline/commit comments and review
threads were inspected. No review comments, pending reviews or unresolved
threads existed at this review snapshot. CI annotations were runner notices
and existing Workflow action deprecation warnings, not failed product checks.

The complete approved slice is **not merge-ready**. Remaining blocking work:

1. Actual creation UI, task workspace/config/native admission and fresh fenced
   ownership/first-step authority, not namespace or health observations alone.
2. Durable scoped credential issuance, real initial Hermes delivery and assigned
   structured PM tools; no duplicate run after unknown acceptance.
3. Answer delivery with safe checkpoint/rebind and independent exact-revision
   prerequisite verification before owner confirmation can reach Backlog.
4. Genuine pinned native-skills source/build and live authenticated PM/owner
   acceptance, restart/denial scenarios and production screenshot evidence.

Keep the feature disabled and PRs Draft until these gates pass. An independent
foundation may only be released as an explicitly approved separate scope;
green CI alone cannot narrow the user's acceptance criteria. See
[GAP_REGISTER](GAP_REGISTER.md) for exit criteria.

## Fresh Namespace Guard (2026-10-02)

The creation coordinator now requires uncached Workflow ownership readback before
any Tracker read/write or chat continuation, including completed operation replay.
The dedicated server PAT has no human/catalog/callback fallback and is redacted
from config Debug/serialization. The callback's credential separation also
rejects this PAT as a readback secret. Exact issuer spelling (including explicit
default ports), original provisioner and Tracker instance/project are checked.
The guard creates neither ownership mapping nor admission/lease/runtime receipt.

Rust 1.88.0 locked workspace tests passed 115 library and 38 actual PostgreSQL
17.11 cases with both DB variables configured. Four client cases cover strict
wire/authority/UUID/namespace validation, fresh reads and refusal after revocation,
redirect/no retry, invalid/oversized/encoded bodies, chunked size limits, stalled
body deadline and exact 128-byte multibyte instance boundary. Coordinator cases
check actual call order: namespace denial prevents subsequent calls; recovered
and completed replay do not repeat create/reserve POSTs; stale current assignment
immediately after successful reserve acknowledgement prevents chat creation.

These are controlled HTTP/identity fixtures and owned disposable PostgreSQL,
not live Base/Workflow/Hermes acceptance. Three separately gated directory/SSE/
historical migration cases were ignored in this run; earlier and CI evidence is
recorded separately. Namespace mapping time is not lease freshness; full fenced
predispatch admission, workspace/native readiness and first-step evidence remain
open. No frontend composition, public API schema or migration changed.

## Accepted Base Reconciliation (2026-10-02)

The candidate includes accepted Fleet main `11a22c1` and Base pin
`c083783a37791e277db796361203884b87828a7d`. Frozen pnpm installation, Rust 1.88.0
locked fmt/check/Clippy/tests and generated source/client OpenAPI checks passed.
The rerun has 111 library tests, 37 actual PostgreSQL 17.11 cases, plus the
separate empty-DB historical migration test. Node 22.20.0/pnpm 10.28.1 gates pass
typecheck, 209 unit tests, lint, format and production build. Shared library
cleanup removed the old local time tests; this count replaces the pre-reconciliation
215 total below. Package-consumer and effective three-theme contrast checks pass.

All 36 three-browser fixture scenarios passed; 27 opt-in live checks were skipped.
Nine controller screenshots were regenerated and verified; mobile clarification
and desktop dialogue were opened for visual inspection. The historical 135-image
manifest still verifies, and links were checked across 90 documents.

The API compatibility gate retains an explicit product security migration for the
legacy run-wide approval `200` -> `409`; it does not pretend this is backwards
compatible. Four wrapper regressions verify exact refusal/no numeric or wildcard
success even after baseline retirement, while official Base comparison still
rejects sibling removals and request/other-response/schema changes. CI's own
temporary theme preview is terminated before the separate browser/capture gates.
These are source/repository/fixture checks, not live PM or rollout acceptance.

## Persisted PM Draft Creation (2026-10-02)

Owner/key-unique ledger, strict Tracker creation/input/reservation readback and
atomic private task-bound chat are implemented behind a disabled-by-default
project allowlist. Creation returns awaiting_admission with dispatch_allowed=false;
there is no initial prompt or Hermes run. No human credentials are persisted.

The WSL workspace suite passed 111 library tests and 37 actual PostgreSQL 17.11
cases with both DB variables configured. Four new PG cases cover lost creation
and reservation responses, independent repository recreation, concurrent replay,
changed payload, immutable receipts, stale current assignment, foreign-owner read
and no generic prompt/run escape. Actual Fleet TCP HTTP handlers additionally
reject machine/local-without-central consent, foreign-admin receipt access and
revoked project access. Upstream Tracker and identity markers are controlled test
fixtures in these cases, not a live Central Auth/Tracker/Hermes acceptance.

Exact captured Tracker HTTP reservation/readback bytes from source
`e8ba23b1a19c3527f4b14bd8d530f0db9870d40b` decode without hash rewriting. Tests pin
their body digests, the reserve_pm_draft command envelope hash, explicit nulls,
canonical UUIDs, opaque machine subjects, fixed origin/no retry/no redirect and
safe rejection classes. The captured requests used synthetic identities.
Generated OpenAPI operation IDs are also checked for uniqueness. Formatting,
all-target check and strict Clippy passed; generated client/typecheck/lint/format,
215 frontend tests, build, seven-shape contract check, 135-screen manifest and
89-document link check passed. No production screen composition changed here;
fixture image evidence remains separate from actual PM delivery.
The separate fresh-DB migration/backfill/down-up test also passed; production
down/up is not a recovery method and old preview migration records are not parity.

## History Catch-Up Follow-Up (2026-10-02)

History invalidation now waits for an in-flight page without cancelling it, then
performs a catch-up read. Reconnect and message events arriving during older-page
loading cannot silently lose the newly appended message until another event.
Both deterministic component cases keep the older page and display the new
message in server order after the subsequent read. The full frontend unit gate
passed 215 tests; typecheck/build passed. Public API and database schema did not
change. Lint/format and all 36 Chromium/Firefox/WebKit fixture cases also passed;
27 opt-in live cases were skipped. The nine controller images were regenerated
and verified from the passed run. These remain controller tests, not live PM
delivery evidence.

## Atomic PM Draft Chat Follow-Up (2026-10-02)

The internal repository operation creates private session, immutable binding,
owner/primary participants and one audit/durable event in a single transaction.
It does not create a system message, pending run or prompt outbox. Ordinary prompt
dispatch is rejected on the new binding. It is not exposed as a public creation
API and does not attest to an actual Tracker assignment or Workflow admission.

WSL workspace tests passed 106 library and 33 actual PostgreSQL 17.11 cases.
The final focused two-case rerun additionally checked distinct command-key races
for one task/agent: one winner, no orphan free chat. Cases also cover duplicate
actor/key replay, independent repository recreation, changed payload, duplicate
binding rollback, exact audit/event/participants, foreign/disabled owner and
non-PM rejection. The common PM test owner now uses a canonical central UUID.
Strict all-target Clippy and formatting passed; public OpenAPI remains unchanged.
No human authentication or runtime delivery is inferred from repository tests.

## Transcript Ordering Follow-Up (2026-10-02)

The backend now uses immutable internal database identity allocation order for
new messages, with unchanged public UUID cursor/message wire. The WSL workspace
gate passed 106 library and 31 actual PostgreSQL 17.11 tests; a separate empty
database passed the historical backfill, clock rollback and down/reapply test.
The regression checks both paginated history and the legacy message listing.
The backfill test also verifies that existing durable events are not duplicated.
Frontend coverage checks server page order, overlapping message deduplication and
event-stream reconnect during previous-page loading. Browser verification uses
fixture APIs, not a real PM run. CI independently executes the migration test.
Final UI gates passed 214 unit tests and 36 fixture Playwright cases in Chromium,
Firefox and WebKit; 27 opt-in live cases were skipped, not accepted. Typecheck,
lint, format and build passed. Nine controller images were regenerated from the
passed run, with their route/viewport/content-hash manifest; the existing full
135-screen manifest and generated API/client drift checks also passed.

Historical rows are backfilled in their previous timestamp/UUID order. Allocation
is not commit order, and its bigint is not a durable SSE cursor. Down/reapply
retains messages but cannot preserve the new order; production recovery needs a
verified backup or forward migration. No accepted deployments or secrets changed.

## Metadata Inbox Follow-Up

### Authenticated Poller Follow-Up (2026-10-02)

The opt-in server worker is connected, disabled by default. WSL workspace gates
passed 106 library tests and 31 actual PostgreSQL 17.11 cases with both database
variables configured; format, all-target check and strict Clippy passed.
Two new PostgreSQL/HTTP cases verify exact machine identity/read scope, fresh
project access, active owner/keyset selection, concurrent duplicate pages,
repository recreation/replay, revoked authorization, oversized/forged pages and
refused redirects. Failed pages preserve the durable source cursor. Projection
creates neither a pending prompt nor a runtime run. Config debug/serialization
redacts the PAT; warnings use fixed safe diagnostic codes.
Final-tree workspace rerun passed after making the existing history pagination
fixture's timestamps explicit. A preceding rerun exposed host clock rollback;
this was a limitation of that poller baseline, addressed for new messages by the
separate transcript-order follow-up above.
Pending migration rollback/reapply, source OpenAPI comparison and Markdown links
also passed. The two ignored directory/SSE tests remain separate CI gates.

Authorization endpoints in these component tests are synthetic. Actual Base/
Tracker issuance and live PM publication/resume/Backlog acceptance remain open.
Accepted deployments, secrets, pinned images and runtime volumes were not changed.

The 2026-10-02 WSL gate passed 102 library tests and 29 actual PostgreSQL 17.11
tests with both required database variables configured. Two new domain tests
cover all nine typed event resources, maximum bigint cursor strings, required
nulls, noncanonical/nil IDs, hashes, unsafe versions and page forgery. Two new
database tests cover empty-page pinning, concurrent replay, reconnect, changed
receipts, legacy-format rejection and complete rollback on a middle-page error.
Database triggers reject cursor deletion/regression and format changes.

No public API, UI, runtime images or accepted migrations changed. The existing
single pending task migration owns the new projection/version columns and guard.
That storage-only follow-up did not connect the poller; its contract/DB checks
are not live Tracker publication, PM delivery or Backlog acceptance.

Two contract snapshots in `backend/domain/tests/fixtures` were captured from
Tracker's actual PostgreSQL-backed `drafts` and `sdlc` HTTP integration tests,
not generated by Fleet. The consumer regression decodes all nine event types
and verifies their original source digests, including UTF-8/escaping and sequence
gaps. The test data and Central issuer are synthetic; this is producer/consumer
wire evidence, not live PM or owner acceptance.
The snapshot follow-up passed all 103 library tests, format and strict all-target
Clippy. The preceding storage implementation passed all four CI jobs on
`218af39`; that CI is not evidence for a later snapshot-test commit.
Snapshot commit `b0b472a` subsequently passed all four CI jobs. Poller evidence
above is a separate implementation/gate and must not inherit an older CI result.

## Source Baseline

Fleet foundation: `470f2856735bde5eff5bc969b6973a6d38044b12`.
Tracker implementation: `24f0f1e` (includes `0cffd7d8` clarification foundation).
Workflow implementation: `a99a5d8`.
Existing unrelated sibling workspaces were not changed. Each service has its own branch.

## Verified Fleet Gates

- Linux Rust 1.88: format, all-target check, strict Clippy, workspace tests and source
  OpenAPI regeneration/diff. 69 library tests; 11 actual PostgreSQL foundation cases.
- Isolated PostgreSQL 17.6: clean ten-migration up, central-subject regression with its
  own configured database URL, migration 000010 down/reapply/status.
- Frontend: generated API drift, seven Tracker wire-contract comparisons, checker tests,
  typecheck, production build, lint and format; 146 unit tests passed.
- Playwright: 21 fixtures passed in Chromium/Firefox/WebKit. PM tabs have no document-level
  overflow at 375x812, 1920x1080 and 2560x1440, no serious/critical scoped axe violations,
  tablet Escape/focus restoration and keyboard tab navigation. Nine generated controller
  images have verified routes/viewports/content hashes; liveAcceptance is explicitly false.
- Controller browser evidence is fixture-only. The real chat component uses fixed-origin
  gateway clients; mocked APIs in the browser test do not dispatch Hermes or resume Workflow.

The library/workspace gate and targeted PostgreSQL cases are separate evidence. Tests with
an absent database variable must not be described as database acceptance. Native Windows
Rust linker availability and remote CI head status remain separate gates.

## Cross-Service Evidence

Tracker's own verification ledger records strict central/project/owner/machine authorization,
durable questions/answers/revisions/confirmations/outbox and a real PostgreSQL HTTP test
with a synthetic Central Auth issuer. Its follow-up adds restart-safe human Draft creation,
with project access rechecked on replay, and passes strict Clippy after six minimal
pre-existing warning fixes. It does not dispatch PM. Draft verification is independent
of live cross-service PM acceptance.

Workflow's own ledger records 1,932 unit and 52 PostgreSQL tests, focused PM contract tests,
Ruff/mypy and generated OpenAPI checks. Its continuation API consumes a trusted Fleet probe;
Fleet now implements the callback server. The dispatch/resume orchestrator is still pending.

## PM Readback Follow-Up

### Targeted Runtime Approval Follow-Up

The working branch implements a generated exact-request API, human-session-only
decisions, immutable command reservation before HTTP, audited single-request
resolution and durable invalidation. Broad legacy approval calls fail closed.
Approval requested/responded events are no longer treated as interchangeable.

Additional actual PostgreSQL 16 integration cases passed: concurrent replay,
foreign-owner denial, changed command conflict, secret redaction, single-request
resolution and immutable decision audits; authenticated fake-Hermes HTTP proves
machine denial, malformed acknowledgement remaining uncertain, safe readback and
no duplicate dispatch. A deterministic lock-order regression covers concurrent
delivery/replay and actor foreign-key locks. Signed provider JWTs through the actual
authentication middleware do not prove a human approval session, including invented
session claims. Historical readback/replay survives PM reassignment without dispatch,
but fresh commands fail stale fencing and revoked project access denies all reads.
This is not live Hermes/provider acceptance. Integrated approval UI passes unit tests
and Chromium/Firefox/WebKit fixtures; independent reconciliation of an unknown runtime
decision remains open.

The production ingester accepts Hermes `approval.request` and compatible
`approval.requested`, never `approval.responded` as a new request. Its separate own-PG
authenticated fake-Hermes SSE test proves two exact request IDs, replay/concurrent
upserts and no response-created request rows. CI explicitly runs this ignored test
and directory acceptance on separate PostgreSQL 17 databases.

The directory sidecar passed its separate own-database acceptance and one-query
scoped aggregation checks, all frontend gates (160 unit tests), and Chromium,
Firefox and WebKit fixture flows. Light/dark viewport captures are fixture evidence;
they do not substitute for live acceptance or the final production manifest.

The working branch extends its single pending migration 000010 with an atomic immutable PM run reservation,
write-once acknowledgement mapping and fresh Hermes readback for Workflow. The
callback uses a dedicated machine credential and is not browser-authenticated.
Terminal proof cannot regress; an unknown reservation retains agent capacity.
Hermes alias resolution is supported by pinning the acknowledged effective
runtime session ID separately from Fleet's stable session identity.

The earlier follow-up WSL workspace gate passed with 88 library tests and 19 actual PostgreSQL cases
(18 SDLC foundation cases plus managed settings). These follow-up database tests
used PostgreSQL 16.15 because the shared Docker engine was unresponsive. They do
not replace the required PostgreSQL 17 migration/CI gate. The new cases exercise
actual PostgreSQL transactions and the Fleet callback against an authenticated
test HTTP runtime, not a real Hermes PM/provider. The directory test is explicitly
ignored in the normal workspace suite and needs its own disposable database.

The clean disposable PostgreSQL 16 database passed ten-migration up, pending
000010 down/reapply/status before refreshing main. After that refresh, the final
eleven-file schema passed clean up, pending 000010 down/reapply/status on a fresh
PostgreSQL 17.11 database. This feature owns only one new migration. Previous
unconsolidated schema runs do not substitute for this final-tree verification.
No accepted runtime database or volume was changed. Production rollback must
disable the feature rather than remove run proof.

The security follow-up passed strict Clippy and the complete WSL workspace suite:
93 library tests and 23 actual PostgreSQL 17.11 cases, including both database
environment variables explicitly configured. Directory and runtime-approval-event
tests remain explicit separate gates, not silently counted as normal workspace
acceptance. The directory's updated scope acceptance separately passed PostgreSQL
17.11. Disposable QA databases/role were removed after verifying ownership.

New deterministic regressions prove: historical chats survive PM replacement;
revoked project access denies detail, legacy lists, messages/history, participants,
runs, controls and stop; an already-open SSE stream closes without emitting queued
data; changed assignments during an actor lock wait dispatch zero approval HTTP
requests; a definitely undispatched decision is terminal failed and never replayed;
PM capacity locks remain compatible with mirror author-agent foreign keys.
These cases use authoritative database transactions and controlled HTTP servers,
not live Central Auth/Hermes/provider acceptance. Cross-service replacement still
needs confirmed runtime quiescence, not just a last-moment authorization read.

The subsequent inbox foundation passed the complete WSL workspace gate: 96 library
tests and 26 actual PostgreSQL 17.11 cases with both database URLs configured.
Three new DB regressions prove concurrent exact replay, cursor persistence after
reconnect, changed-payload/stale-page/foreign-binding rejection, receipt immutability
and rollback of the entire page on an injected mid-page database failure. Hashes
and safe metadata are stored, not arbitrary answer/result bodies. No prompt is
queued by projection. At that storage-only baseline the gateway poller and real
PM delivery remained unwired; those tests do not prove either. The newer worker
component evidence is recorded above, independently of live PM acceptance.

The refreshed frontend passed 212 unit tests. Approval response shape errors are
surfaced as loading errors rather than crashing the transcript. Screenshot/browser
fixtures are kept distinct from real PM acceptance.

The complete refreshed browser suite passed 33 tests across Chromium, Firefox and
WebKit, covering the fleet flows, scoped chat directory and targeted approvals.
Nine controller fixture images were regenerated for dialogue, clarification and
requirements at 375x812, 1920x1080 and 2560x1440. Their generated manifest and hashes
passed verification; mobile dialogue and desktop clarification were visually
inspected. This evidence explicitly records `liveAcceptance=false`.

The PM tests cover concurrent identical reservations, payload conflict, immutable
mapping/fence/terminal proof, unknown acceptance holding capacity, bad callback
credential, wrong Hermes run/session identity, fresh observation IDs and runtime
unavailability after terminal proof. Real provider, structured PM tools and
checkpoint/resume acceptance remain separate requirements.

The accepted snapshot [tracker-clarification-v1.json](contracts/tracker-clarification-v1.json)
was generated from the Tracker commit above, not authored as an independent API spec.
The checker resolves schema refs and compares fields/requiredness/types/nullability/formats/
enums. Constraints and business permissions are checked by service tests, not by that wire
normalizer. Rollout must compare the actual Tracker build with the same snapshot.

## Terminal Proof Reconciliation (2 October 2026)

The full WSL workspace suite passed with 100 library tests and 27 actual
PostgreSQL 17.11 cases; both database variables were explicitly configured.
The final four focused PM database cases additionally verify a mismatched runtime
mapping rolls back both terminal proof and visible state. Completion, failure,
cancellation and stop map to matching terminal run states. Late stream updates
serialize under the same binding/run lock order and cannot reopen the old run or
free the next unresolved reservation. Generic terminal cache updates without
proof and altered runtime mappings are rejected. All-target check, strict Clippy
and formatting passed. The separately ignored directory/SSE cases are not counted
as new local acceptance. No schema, public API or UI changed in this follow-up.

## Release Blockers

The effective-configuration follow-up passed WSL Rust 1.88 format, all-target
check and strict all-target Clippy, 118 library tests and 39 actual PostgreSQL
17.11 cases with both database variables configured. Its three filesystem cases
verify same-size drift for every managed file, absent files, re-enabled disabled
skills, wrong snapshot/head/marker, oversized/non-file marker, missing/foreign
workspace and Unix symlinks without repairs or secret-bearing errors. The new
real PostgreSQL HTTP case returns a generic blocker for a database-only active
revision and denies a regular user; identity middleware is a controlled fixture,
not live Central Auth. The final library gate was rerun after marker hardening.
Temporary QA database/role were removed, accepted runtime remained unchanged.
Frontend passed 211 unit tests (including two localized blocker cases),
typecheck/lint/format/build; regenerated source OpenAPI is unchanged and 90
Markdown documents passed link checks. No visual composition, schema or public
DTO changed; existing screenshot evidence was not relabeled as live.
Config/runtime/task-workspace fencing and actual PM admission remain open.
Independent review found that Hermes writes category directories and
`.bundled_manifest` alongside Fleet-managed skills. The corrected check only
attests snapshot-managed files and preserves runtime-owned inventory. A
controlled bundled-layout regression prevents a permanent false readback blocker;
it does not establish actual bundled/native provenance, which remains an explicit
`runtime_skill_inventory_not_verified` readiness blocker.
The corrected tree passed the 118-library and 211-frontend test gates plus
frontend typecheck/lint/format/build. Its broader local Rust rerun was interrupted
by WSL unavailability, not accepted as a pass. Exact-tree Linux CI remains the
authority for remaining Rust checks; the exact-tree success is recorded in the
Merge Gate Review above. No shared runtime restart was attempted.

The 2026-10-02 server-only credential client follow-up passed 100 WSL Rust library
tests, format, all-target check and strict all-target Clippy. Its four focused
cases validate actual Tracker grant formatting and safe integer bounds,
201/200 acknowledgements, strict scope/token/expiry/no-store checks, foreign
origin/pre-authorized-request denial, root/child debug redaction, revocation,
redirect/oversize/malformed/error rejection and unknown-outcome preservation.
The HTTP issuer is controlled test code, not a live Base deployment. Public API,
database schema, frontend and accepted runtime were unchanged in this follow-up.
Persisted issuance operations and actual credential/tool handoff remain open.

Workflow PR #90 now reconciles the feature with accepted catalog v2/master;
it does not implement PM Draft admission. A release-compatible Workflow build
still requires the actual native-skills Git pin
`46eb27f70b68cbefbf53903090f0c7f0fa68b748`. The source was not found locally and
the private GitLab remote denied access. The development capabilities 503 is
intentional fail-closed behavior, not live PM readiness. No fixture manifest or
guessed compatibility hash may substitute for that dependency.

Admitted initial runtime delivery, scoped PM structured tools, live authenticated
Tracker outbox/Fleet inbox projection, live
Hermes PM verification and workflow resume/rebind, independent readiness verifier,
live directory and restart/denial
acceptance remain open in [GAP_REGISTER](GAP_REGISTER.md).

Task-bound ordinary prompts and steer are intentionally rejected. The answer receipt means
Tracker saved an answer, not PM received it. No deployment switch, merge or production-ready
claim should bypass these gaps. Fixture images remain outside the live screenshot manifest.
