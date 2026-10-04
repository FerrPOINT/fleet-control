# Chat Clarification Verification

Date: 2026-10-01. Status: verified foundation, incomplete approved vertical slice.
No real PM publication/resume or live Backlog acceptance is claimed.

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
At that client-only baseline, persisted issuance and actual credential/tool
handoff remained open. The persisted preparation evidence below closes only
the former component gap; it does not establish admission or tool handoff.

Workflow PR #90 now reconciles the feature with accepted catalog v2/master;
it does not implement PM Draft admission. A release-compatible Workflow build
still requires the actual native-skills Git pin
`46eb27f70b68cbefbf53903090f0c7f0fa68b748`. The source was not found locally and
the private GitLab remote denied access. The development capabilities 503 is
intentional fail-closed behavior, not live PM readiness. No fixture manifest or
guessed compatibility hash may substitute for that dependency. This was a
historical source-access blocker: the authorized canonical Base package is now
available at the exact pin in [GAP_REGISTER](GAP_REGISTER.md). Native attestation
and the compatible installed build remain unverified.

Admitted initial runtime delivery, scoped PM structured tools, live authenticated
Tracker outbox/Fleet inbox projection, live
Hermes PM verification and workflow resume/rebind, independent readiness verifier,
live directory and restart/denial
acceptance remain open in [GAP_REGISTER](GAP_REGISTER.md).

Task-bound ordinary prompts and steer are intentionally rejected. The answer receipt means
Tracker saved an answer, not PM received it. No deployment switch, merge or production-ready
claim should bypass these gaps. Fixture images remain outside the live screenshot manifest.

## Persisted Credential Preparation (4 October 2026)

The opt-in creation continuation now records an immutable credential intent
before the Base command, retains ACK metadata before Tracker context readback,
and recovers through the same parent/origins/key/payload. It performs fresh
parent and child introspection and verifies the assigned task's original human
owner and exact PM assignment. Secrets remain memory-only; public responses and
audit never contain the journal's parent fingerprint or remote bodies. Expired
credentials are not renewed by replay. Creation still stops at
`awaiting_admission`, with `dispatch_allowed=false` and no runtime run.

Exact-tree scoped Linux/Rust 1.88/PostgreSQL 17.6 evidence: 33 distinct tests
passed (two migration regressions, seven infrastructure unit/HTTP cases, two
configuration unit cases, ten credential PostgreSQL/HTTP cases, five existing
creation PostgreSQL cases and seven API cases). Audit INSERT failure injection
proves intent/ACK journal changes roll back atomically; replay after a failed ACK
audit recovers the same child rather than minting another. Timestamp negatives
include invalid dates, hour/offset 24, excess fractional precision and leap
seconds. Additive migration 000011 preserves predecessor operations unchanged,
retains historical 000010 and refuses downgrade while any journal exists.

Scoped infra/API/server/migration Clippy with warnings denied and format checks
passed. Rust-generated OpenAPI is unchanged by the private journal. Node 22.20
checks passed: API drift/compatibility, TypeScript, seven wire contracts with two
checker regressions, and 97 Markdown link files. Changed-file heuristic secret
scan passed; it is not a whole-repository DLP or final release gate.

Evidence log: `.local/pdlc-implementation/credential-creation-scoped.log`.
Final QA project `sdlc-qa-fleet-credential-creation-cf7b2a32240e` was removed
with exact Compose finally cleanup, preserving external caches. The preceding
exact-tree project `sdlc-qa-fleet-credential-creation-52ee6c112bed` also passed
and was removed. Source review findings on the legacy creation guard and
unreadable first-write JSON/timestamps are fixed and covered by regressions.

These tests use real Fleet persistence/coordinator/application code with
controlled Base/Tracker HTTP producers. Actual Base-issued child interoperability
with real Tracker, admission, credential renewal/revocation administration,
runtime structured tools and live PM resume remain unverified. No UI composition,
screenshots, accepted runtime, package pin or deployment was changed here.

### Broader Credential Regression

After the component gate, the Linux/Rust 1.88/PostgreSQL 17.6 regression run
passed all 174 workspace library tests and all 70 `sdlc_foundation` tests with
the real database and explicit pinned-package checkout configured. Workspace
all-target check and strict all-target Clippy passed. A pre-existing
`field_reassign_with_default` warning in the configuration-reader allowlist test
was corrected without suppressing the lint or changing its assertions; the
entire library/foundation/check/Clippy run was repeated successfully afterward.
Formatting passed. Other integration targets, release build and live acceptance
are not included in these counts.

Evidence: `.local/pdlc-implementation/credential-regression-scoped.log`, final
project `sdlc-qa-fleet-credential-regression-1c527db0f006`; exact finally cleanup
removed its containers/network and preserved caches. The earlier warning-failed
project `sdlc-qa-fleet-credential-regression-b8ed8e9bf5c7` was also cleaned up and
is not counted as a passing gate. Node 22.20 frontend passed 229 tests, lint and
format. Manifest/hash verifiers passed for 135 fixture screenshots and nine
chat-controller fixture images; images were not regenerated or relabeled live.
The opt-in real-producer credential target was compiled by all-target checks,
not executed by this library/foundation run.

### Actual Base And Tracker Credential Interoperability

The opt-in `pm_credentials_live` integration target now executed against actual
locked Base and Tracker binaries and their separate disposable PostgreSQL
databases. Base producer `ddfb436bf2b3253561672c92b2dbc06803cabf90`, Tracker
producer `af6ed1ee26f6d26534a0dd1526e3b4d168962160`, Fleet library snapshot
`605e19b1556278fb7acef5b917ab047856053a7f` and SDK
`9408802dfa978cba2f67162a49adca6f65851b01` are frozen in the harness evidence.
The test SHA256 is
`80fff1364630049ea736910361bc104b16a340eec51002e958510d8221db8fba`.

One actual test passed on Linux/Rust 1.88/PostgreSQL 17.6, with scoped Clippy and
format checks. It uses the production issuer/command, real owner Draft and PM
assignment APIs, exact child replay/expiry/lineage and current context readback.
Tracker local UUIDs differ from verified central subjects. Foreign task, legacy
API, owner reservation readback and owner-only confirmation are denied by the
real server. Authenticated parent revocation invalidates child introspection,
Tracker access and further issuance. No synthetic issuer ACK is involved.

Actual exec session 34009 exited zero. Preserved log
`tmp/pm-credentials-live-run-3.log` SHA256:
`1b013d075248f137d995d256fda191e6489588fe90c15cb05f54a3cdf67726a5`.
Project `sdlc-qa-pm-live-af76a8a560fd` was removed by exact Compose finally;
independent inspection found no containers/network. Private fixture was deleted
and caches retained. The [harness](../scripts/pm_credentials_live/README.md)
requires explicit registry-download consent if its locked build cache is cold.

This closes issuer-to-current-context interoperability, not the persisted Fleet
creation coordinator's complete live saga. Its journal/fault-injection tests
remain separate evidence. Workflow admission, runtime credential handoff/tools,
renewal policy and genuine PM question/answer/resume acceptance remain open.

## Accepted Free-Chat Run Recovery (4 October 2026)

The source commits verified Hermes HTTP 202 acceptance atomically across the
run, prompt delivery and outbox, before effective-session GET. A pending run
with a durable native ID holds capacity; the keyset worker only reads that ID.
Authenticated bounded readback pins the actual session once, not the requested
Fleet alias. Concurrent recoverers have one stream-start winner. Task-bound/PM
records remain excluded even when durable capabilities are valid.

Final Linux/Rust 1.88/PostgreSQL 17.6 source gate: 176 workspace library cases,
85 foundation PG/HTTP cases and one separately enabled approval SSE case passed
(262 distinct cases). All-target check and strict Clippy, formatting and exact
Rust-generated OpenAPI comparison passed. Existing Base Git blobs were copied
unchanged from the read-only package mount to a disposable Linux layer with the
same HEAD, avoiding Windows-mounted Git IO; no source pin or production timeout
was changed. README validation and all 98 Markdown link files passed.

Eleven atomic repository cases include outbox-trigger rollback, concurrent ACK/
pin, identity reuse, task/PM denial and terminal replay. A restricted PostgreSQL
role injects a real full-agent read failure after ACK commit; subsequent
`Failed(None)` cannot erase delivery and the original run remains recoverable.
Three production-adapter HTTP cases cover initial GET outage, foreign identity,
restart, unchanged single submission and queue progress past 21 rejected ACKs.
Forged running snapshots cannot issue stop/steer while the persisted run awaits
pin. Existing EOF/terminal negatives and authenticated approval ingestion pass;
the fixture requires one POST/one SSE and authenticated native status readback.

Two source-review findings are closed by those guards and regressions; the
independent re-review found no additional actionable issue in their corrections.
Heuristic scanning of all 21 changed/new files reported one unchanged synthetic
redaction-test literal already present at the parent head; review found no new
secret. This is not a whole-repository DLP or entropy/history certification.

Final exec session 40810 exited zero. Evidence log
`.local/pdlc-implementation/acceptance-readback-scoped.log` SHA256:
`a0bbf74d0be168f9221634d7f769d0ea4d115efbee936ab425a77bfd4c6723f4`.
Exact project `sdlc-qa-fleet-acceptance-f7274d0f5ecd` was removed by finally;
independent Compose inspection found no remaining containers. External caches
were preserved. Earlier failing attempts remain separate logs, not passing gates.

These are controlled Hermes HTTP producers with real Fleet persistence/runtime
paths. Authentic Hermes gateway/model acceptance, unknown run-ID recovery,
exact-request/fingerprint/horizon journal, post-pin stream recovery, offline
controls, process-tree safe stop, PM admission/tools/resume, full release/CI gate
and current production screenshots remain required. No accepted runtime was
installed or updated; no PR was pushed or marked ready by this gate.

## Native Hermes Protocol Acceptance (4 October 2026)

The [opt-in harness](../scripts/hermes_protocol_live/README.md) now executes
the actual pinned Hermes API adapter, real AIAgent, native middleware and SQLite
in separate Python processes. Only model inference is a deterministic local
OpenAI fixture. Hermes HTTP and durable storage are not mocked or rewritten.
Clean source `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3` was extracted from Git;
native module paths are checked against that read-only snapshot. The existing
Base dependency image resolves to
`sha256:aeb97055b0f5aee433e29998eeafd8065b81e70d1fcb69345c520c2bfbf23777`;
its revision label and exact `uv.lock`/`pyproject.toml` are checked. It is not
installed or promoted as a new Fleet runtime.

Four native cases passed:

- Parsed native SSE identifies the accepted run and has one completed event
  with `completed=true`, `partial=false`, `interrupted=false`. Authenticated
  status, native message API and an independent read-only SQLite session row
  match the exact requested session. Two distinct processes/homes reject foreign
  credentials and foreign run lookup; an unknown profile prefix is denied.
- A loopback fault proxy forwards the real POST, consumes its 202, then closes
  without acknowledging the caller. Eight concurrent exact-key replays preserve
  its original run ID and one model execution; changed input conflicts. Killing
  and restarting the process with the same HOME/store/token preserves terminal
  output and session. SSE is unavailable after restart; status GET is authoritative.
- A model barrier holds real inference after the lost ACK. The native process
  is killed/reaped before replay or completion. Restart and original-key replay
  preserve that ID as `interrupted`, with no completed output or second inference.
- A rotated API credential cannot authenticate the old run; its new scope can
  accept the same key as a different run. This proves why Fleet recovery needs
  the original credential/profile fingerprint and must not blindly replay.

Eight host safety cases passed, including corrupt SSE rejection, exact clean
pin, traversal/link refusal, a real keepalive deadline and retained failed
evidence when Compose down/ps time out. These host cases are added to docs CI;
the native gate is opt-in and requires the explicit dependency image. HTTP IO
has an absolute shutdown/remaining-time budget and the Compose runner a 600 s
watchdog. Lazy installs, auto-titling, background review and memory are disabled
through upstream settings, not patched native code. Expected model-peer
BrokenPipe after the deliberate kill is diagnostic noise, not a successful run.

Final exec session 20490 exited zero. Project
`sdlc-qa-hermes-protocol-6c243df9a9b8` used an internal network, no published
ports/host credentials/runtime folders/persistent volumes, read-only source/root
and disposable tmpfs. Exact finally cleanup exited zero; independent Compose ps
was empty. Docker audit subsequently reported complete=true, 47 desktop
containers, zero runner containers and no violations. Other owners' resources
were not changed. Earlier failed attempts remain separate evidence.

Evidence under ignored `tmp/hermes-protocol-live/`:

- native log SHA256: `28cf86678b0d1195ed5a193c795adf62f4eef01bf7315a7a272708db0487f401`;
- probe SHA256: `50877923aff410016d60c0e77d9a605728ab68dd223d23aa07b7990874052573`;
- runner SHA256: `efde005f9f9410a558f7dc7a7fdd1941e9e79a72d663288bd4202af2c73c4da6`;
- Git archive SHA256: `571fba4903d9094ade7f6d0ef5dfcee8c068e50f62e65f46611ec3ad65697e02`.

This is authentic native protocol evidence, not full `gateway run` startup,
Base wrapper/dotenv precedence, compression/rotation, multiplex, positive native
stop/steer/tool approval, OS quiescence, Fleet lost-ACK recovery or the paid-model
PM lifecycle. Exact request/scope/horizon journaling, pin-to-worker recovery,
assignment admission/heartbeat/first step, structured PM tools/resume and Forge
pipeline/deployment/acceptance remain required. Nothing was installed, pushed,
merged or designated merge-ready by this component gate.

## Original Hermes Dispatch Journal (4 October 2026)

The production free-chat sender now atomically prepares immutable original bytes,
SHA256, UUID idempotency key, concrete run/alias, loopback origin, default-profile
credential fingerprint, bounded verified capabilities and a DB-clock retention
horizon. A durable single-winner submission permit commits before HTTP. Verified
ACK commits journal/run/message/outbox in the same transaction. Known-ID restart
GET requires accepted original journal context; legacy or rotated/moved context
does not initiate HTTP. Neither timeout nor unknown acceptance permits another POST.

Final exec 46140 exited zero on Linux/Rust 1.88/PostgreSQL 17.6: 179 workspace
library + 102 foundation PG/HTTP + one authenticated approval SSE + three
isolated migration cases = 285 distinct PASS. All-target check/strict Clippy,
fmt and Rust-generated OpenAPI equality passed. Migration 000012 is additive;
pending, accepted-unpinned and completed legacy runs/messages/outboxes remain
byte-equivalent across upgrade/empty downgrade/reapply. Nonempty journal
downgrade is refused. Older migration regressions now locate their own migration
by name instead of assuming that it is the newest one.

Fifteen new journal PG cases cover concurrent prepare/one-winner claim, exact
model/options bytes, rollback, scope/payload conflicts, capability/drain/capacity/
task-PM denial, horizon/tamper/delete guards, legacy leader envelopes, terminal
ACK replay and sanitized SQL diagnostics. Independent source review found a
late-error race: prepared classification could precede a concurrent submitted
commit. Classification now reads current journal under message lock; the stale
sender read is removed. The regression commits submitted before late Failed,
keeps delivery pending, accepts the actual ACK and preserves it after another
late error. Counter-review found no remaining actionable P1/P2 in this correction.

Controlled HTTP inspects the committed journal before POST and loses a valid
acceptance shape; another send cannot submit again. Existing post-commit ACK
fault now uses the journal and a restricted database role with deliberate final
agent-read denial. The first attempted gate failed before that fault because
fixture privileges lacked the new table; it is retained separately and not counted
as PASS. Privileges/assertions were corrected and the full gate repeated.

Evidence log `.local/pdlc-implementation/dispatch-journal-scoped.log` SHA256:
`f66ee87feab72c98017238f6ca609a661cf2b9e315f20cb10adb3787467f4c25`.
Exact final project `sdlc-qa-fleet-journal-1bbb0d625694` was removed in finally;
independent Compose ps exited zero with no containers. External caches stayed
unchanged. Earlier attempt and before-review logs remain separate evidence.

Node22 frontend: 230 tests, typecheck/lint/format/build, API drift/compatibility,
seven generated chat contracts and their verifier tests passed. A new component
case verifies pending delivery error and disabled parallel-send UI after reload.
README and 99 Markdown files passed; 135 fixture screenshots and nine controller
hashes were verified, not regenerated or promoted to live evidence. Existing Vite
large-chunk warning remains. No production visual change was made by this packet.
Heuristic secret scanning covered all 40 changed/new task files. Its one finding
is the unchanged synthetic redaction-test literal already present at the parent
head; no new finding was introduced. This is not full DLP/history certification.
CI YAML parsing confirms the explicit isolated journal migration target.

This closes original request preservation/single submission and the reviewed
late-error race, not unknown-ID recovery or native store continuity. The pinned
Hermes HTTP API still has no authenticated non-dispatch key lookup or store epoch;
negative/expired/reset storage cannot authorize POST replay. Prepared-intent
restart recovery, pin-to-worker recovery, independent offline controls, native
configuration/process-tree proof, predispatch admission, PM tools/resume, Forge
candidate/pipeline/deployment/acceptance, full release/CI and live screenshots
remain required. Producer heads stayed Tracker `af6ed1e` and Workflow `2d79461`.
No producer, installed runtime, dependency pin or historical migration was changed.
Publication still requires the documented per-task migration release ordering;
the old remote Draft PR head/checks do not prove this local packet.

## Versioned Native Hermes Renderer (4 October 2026)

New Hermes configuration snapshots use server-selected renderer2; Java uses1.
Absent/explicit1 retains legacy serialization and rendered config/env/marker bytes.
The renderer seals native enablement, loopback host, assigned port, derived key,
HOME and explicit CORS, including an empty YAML list. It rejects alternative API
aliases and malformed gateway/platform extra before filesystem access. Existing
files/rows are not upgraded in place; draft/drain/activation remains mandatory.

Independent review found alias precedence, native YAML fallback and direct
gateway-plugin extra gaps. They are covered by new negative validation cases,
direct native YAML processing and no-fallback host tests. Counter-review found
no remaining actionable P1/P2 in those fixes. The legacy synthetic plaintext
secret remains only in its historical fixture, not a valid new v2 draft.

Final exec67148 exited0: Linux/Rust1.88/PostgreSQL17.6, 192 workspace library +
103 foundation PG/HTTP +1 authenticated approval SSE +3 isolated migrations +1
explicit Rust exporter =300 distinct PASS. All-target check/strict Clippy/fmt
and Rust OpenAPI equality passed. Final log SHA256:
`5aa785beea6eb69795236df2a5311e9cd4788c2d04e32e3aed935735ac030e7d`.
Exact `sdlc-qa-fleet-renderer-d382d941c6fa` was removed in finally and independent
ps is empty. Failed pre-final validation evidence is retained separately, not PASS.
No new migration, SDK/runtime/package pin or producer source change is included.

Actual native loader exec64496 exited0 against clean Hermes
`bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3` and existing dependency image ID
`sha256:aeb97055b0f5aee433e29998eeafd8065b81e70d1fcb69345c520c2bfbf23777`.
It consumed files generated by the real Rust provisioner/renderer, not manually
written substitutes. Two native Python processes validated original file hashes,
direct YAML processing (including a malformed-alias negative control), then
combined dotenv/config loading against stale shell listener/key. Assigned ports
29002/29003, HOME identities, credential hashes and CORS matched. No inference,
listener startup or external request was made. Raw keys stay in ignored private
fixtures. The loader-only service matches the exporter UID0 with dropped
capabilities/no-new-privileges and read-only mounts; production UID/OS isolation
is not thereby certified. The earlier image-user permission failure was not
worked around by weakening file modes and is retained as failed evidence.

Native project `sdlc-qa-hermes-protocol-4cbd62c5bec5` completed exact down/ps0;
independent ps is empty. Log SHA256:
`300848f8bce1d596c56edffb3e348626facf9171232f30ddc0341da15c2b007f`;
probe SHA256 `d827e78ac7064d40e0018dec6b4ab3d8488afb78c6ae7af06879aa847c7908f5`;
runner SHA256 `171eaf3fd7e23a1a4c6e1aad036f5d22d6cc7bf8a268fb3559d078d0dd75284d`;
private fixture manifest SHA256
`842c9531ecda378bded09b2d833fded0d9ff641e30822452f8b99bf49cabde8c`.
Thirteen host safety tests pass, including scenario-specific fixture UID and
read-only/no-port guards; default protocol identity is unchanged.

Node22 frontend passes230 tests/typecheck/lint/format/build, API drift/compatibility
and seven chat contracts. README and100 Markdown files pass. All135 fixture
screenshots and9 controller hashes verify without recapture/live promotion;
production visuals were not changed. The Vite large-chunk limitation remains.
Heuristic scan of31 task files has one independently verified unchanged parent
synthetic literal and no new finding; this is not full DLP/history certification.

This packet closes renderer semantics/native loader precedence, not full gateway
lifecycle or installed effective revision/plugin/model-credential attestation.
Unknown-ID recovery/store continuity, pin-to-worker recovery, OS quiescence,
predispatch admission/first-step, PM tools/resume, Forge source/pipeline/deployment/
acceptance, live screenshots and exact-head release CI remain required. SDLC
dispatch stays fail-closed. Publication still needs migration release ordering;
the old remote Draft PR CI does not verify these local changes. Nothing is
merged/installed or designated full merge-ready by this component gate.
