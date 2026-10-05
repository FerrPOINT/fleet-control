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

## Atomic Terminal And Pinned Recovery (4 October 2026)

Known accepted pending/running/waiting/stopping free-chat runs use the bounded
UUID-keyset recovery queue. Original accepted request/journal, origin, credential
and horizon are checked before HTTP. Pinned recovery performs authenticated GET
only: no second POST or SSE worker. A native terminal packet commits the optional
redacted assistant, preview, prompt delivery, run state/error/timestamps and
trigger-owned durable events in one PostgreSQL transaction. Exact replay does
not change timestamps/cursor; contradictory identity/outcome is rejected.
Empty output does not fabricate an assistant or use nested tool metadata.
Late delta/tool/approval paths serialize with terminal persistence.

Review found a real FK lock inversion in exclusive session serialization. The
fix uses NO KEY UPDATE, compatible with progress FK KEY SHARE checks. Three
deterministic blocking-barrier regressions exercise actual run progress, prompt
progress and approval reservation writes. Independent counter-review found no
remaining actionable P1/P2 in this packet. Fourteen atomic-terminal and five
fresh-supervisor pinned-recovery cases cover rollback, concurrent replay,
historical partial mirrors, empty/failed/cancelled outcomes, drain, original
context, task/PM denial and keyset fairness. Controlled HTTP proves zero POST/SSE
for pinned recovery and root-only final body extraction.

Final exec81673 exited0 on Linux/Rust1.88/PostgreSQL17.6: 192 workspace library
+123 foundation PG/HTTP +1 authenticated approval SSE +3 isolated migration
cases =319 distinct PASS. Repeated targeted subsets (8 HTTP, 14 terminal,
5 pinned recovery, 5 acceptance readback) are not counted again. The optional
ignored native-renderer exporter was not rerun; its prior evidence stays separate.
All-target check, strict all-target Clippy, fmt and regenerated Rust OpenAPI
equality passed. This is not all release/integration targets or installed runtime.

The full shared-database gate initially exposed first-page-only ACK assertions
and reused deterministic UUIDs; both were corrected without weakening identity
checks. A later HTTP/SSE timeout prompted strict fixture isolation: Ready before
prompt, claim only its pending outbox, Running then one production send, exact
bearer middleware before counters. Background dequeue remains covered by separate
production-dispatch/journal HTTP cases. The ten-second state, no-false-completion
and held-capacity assertions remain. The setup race was independently reviewed
and corrected. Earlier failed and intentionally aborted runs remain separate
logs and are not PASS evidence.

Final log `.local/pdlc-implementation/terminal-scoped.log` SHA256:
`c4fe83db77fcb790eb3a16fa7879bba3938fcce2d6d02f0646683b5fa21777ff`.
Exact `sdlc-qa-fleet-terminal-b1da956f0f98` was removed in finally; independent
Compose ps is empty. External caches, accepted images/volumes and secrets were
preserved. Final Docker audit: complete=true, desktop39 containers, runners0/0,
violations=[]. No new migration, public wire, dependency/runtime/package pin or
producer source change is included. Read-only producers remain Tracker `af6ed1e`
and Workflow `2d79461`; task/PM dispatch stays fail-closed.

The unchanged frontend retains its 230-test/typecheck/lint/format/build gate.
Typecheck and API drift were repeated; 135 screenshot and nine controller-image
hash verifiers passed without recapture or live promotion. README and 101 Markdown
files pass; Base README/hub validators pass. Vite's existing large-chunk limitation
remains. Heuristic task scanning reports one exact unchanged parent redaction-test
literal and no new finding; it is not full DLP/history certification.

Known pinned terminal recovery and atomic persistence are closed at this source
scope. Unknown-ID positive authenticated non-dispatch lookup/store continuity,
prepared-intent recovery, missed native tool/approval history, independent offline
controls, SSE frame/multibyte bounds, native loaded-config/process-tree proof,
predispatch admission/first-step, PM tools/resume, Forge handoff/live deployment,
live screenshots and exact-head release CI remain required. The current chat owns
Fleet/Base; Forge belongs to the independent second task. Publication still needs
Fleet 000010/000011 before 000012 and at most one new migration per task PR.
Old remote PR47 checks do not prove this local packet. Nothing was pushed,
merged, installed or designated 100% by this gate.

## Original-Key Non-Dispatch Recovery (4 October 2026)

Fleet's default-off consumer freezes the native extension's store UUID, pinned
source, default-profile scope and closed lookup contract before the original
POST. The journal already owns exact bytes/hash/key/origin/credential/horizon;
there is no new migration or public DTO. Initial POST carries the frozen epoch
header. The recovery worker performs bounded authenticated non-dispatch lookup
only for submitted unknown-ID free chats with original proof. Positive acceptance
commits the original ID/run/message/outbox/journal atomically and then uses native
GET for session/terminal proof. Negative/reset/rotation/expired/invalid proof holds
capacity; no recovery POST run or new key is issued. Task/PM remain excluded.

The Base native plugin uses the supported platform-handler hook on the same
listener. An AFTER INSERT witness commits in native reservation SQLite before
inference, survives native pruning and tombstones the original key. Historical
rows are not backfilled; missing/foreign/linked/partial/incompatible stores are
not adopted. Epoch, canonical path/inode, closed schema/triggers and native module
hashes are checked. This does not defend against privileged host/DB modification
or prove OS isolation. Saturation rejects new admissions, not existing reads.
Independent review found and fixed this read/admission distinction. The publisher
is Base [PR #140](https://github.com/FerrPOINT/services-base/pull/140), exact
head `177edb889e3429b18f12affa35f7034623f11523`; original implementation commit
`3a56e5e0f8f2433fa213742f723f8ee3c5f0e31f` has identical task blobs. Existing
dependency PR #126 is untouched. SDK/auth/package/runtime pins are unchanged.

Final Fleet exec91381 exit0: Linux/Rust1.88/PostgreSQL17.6, 195 workspace library
+127 foundation PG/HTTP +1 authenticated approval SSE +3 isolated migrations
=326 distinct PASS; targeted repeated subsets are not counted again. All-target
check/strict Clippy/fmt and regenerated Rust OpenAPI equality pass. Two new HTTP/
PG cases cover the real production epoch submit, malformed ACK, original-ID
recovery and exactly one original POST. Two deterministic PG races observe actual
blocked backends: ordinary/recovered ACK converge to one immutable mapping, and
a journal-lock wait that crosses the DB-clock horizon rolls back every changed
run/message/outbox/journal/session/event field. No deadline waiver is introduced.
Earlier failed fixture runs remain separate logs, not PASS. Final log
`.local/pdlc-implementation/recovery-sdlc-qa-fleet-original-key-8c5219928304.log`
SHA256 `f6b83bce66ad4a5f5080863b3456a1a30845f9d644c55256746ed79adf9bb940`.
Exact `sdlc-qa-fleet-original-key-8c5219928304` finally cleanup and independent
Compose ps pass; no containers remain and shared caches are preserved.

Separate Linux plugin exec76323 exit0: 56 stdlib SQLite/auth/boundary cases,
no skip, including linked paths. Log SHA256
`bfcd8bf79dd9b9414ceeafa697c98d193633191b064f941fafbc646c219a5700`.
Exact `sdlc-qa-recovery-plugin-d0e052a00db7` cleanup/independent ps pass. Base
README/hub and scoped launch/cache checks pass. All six exact publisher
[CI jobs](https://github.com/FerrPOINT/services-base/actions/runs/37213900238)
pass on `177edb889e3429b18f12affa35f7034623f11523`: recovery, Python CLI,
README/hub/Compose, backend PostgreSQL/OTLP, SDK MSRV and frontend/Chromium
viewport regression. No GitHub review threads exist at this observation; no
full SDLC or installed image acceptance is inferred from producer PR checks.
The PR is ready for review, CLEAN and not merged; checks remain green after the
Draft-to-ready transition. The target-main template headings/comments/checklist
are preserved, with the unchanged-UI item explicitly not applicable.

Actual pinned native exec65750 exit0 uses clean source
`bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3` and unchanged dependency image
`sha256:aeb97055b0f5aee433e29998eeafd8065b81e70d1fcb69345c520c2bfbf23777`.
Two real API/AIAgent/SQLite cases pass: dropped202/process restart/eight parallel
non-dispatch lookups restore the original ID with one inference; native pruning
retains the witness/tombstone and database reset rejects the old epoch without
new inference. Only the model is deterministic-local. Native helper/probe are
copied into disposable HOME; plugin discovery occurs only with explicit opt-in.
No installed gateway/config/image/volume or upstream Hermes source was changed.
Exact `sdlc-qa-hermes-protocol-bc4ccbbda982` cleanup and independent ps pass.
Log SHA256 `120bbd5e0ae6ae998770dad464731d98156615eec30d5ad4a081900eb446ea8d`;
probe `b27cafffc6555b9eb88ca05dfc7648f50d28c12b2f1d934de4b9bcce49712c7d`;
runner `d700e0d08cf3459554bdbdabad00ba0cb59bb48df5a5420c98995808e785512c`;
archive `571fba4903d9094ade7f6d0ef5dfcee8c068e50f62e65f46611ec3ad65697e02`;
native helper `4abff4b993a24494450f15f90312eaaa71fc485f2c3864da34e35b8b9603fb8e`.
The four runtime module hashes match exact Base publisher Git blobs:

| Module | SHA256 |
| --- | --- |
| `__init__.py` | `3834f23e1ccba41dd3aed4632393cb90a253d17bab9d9261d2b4993cb6238a52` |
| `plugin.py` | `ca7c1ce3e6727741dd33514cbe523a87ad3bd12ac8fe55e8523ab775f245da6c` |
| `store.py` | `bff0da47840a78b677d42cad8d5a7d8645b821c19d7b02fc416494afcf195359` |
| `plugin.yaml` | `43290436e93490f26e9b1bc279feb19eb77175fab474d696f98a4bf049513aea` |

Fifteen host harness safety tests pass, including recovery source preflight,
read-only snapshots/hashes and exact cleanup evidence without starting Docker.
Production UI is unchanged; existing
fixture screenshots are not live acceptance and are not recaptured by this
runtime packet. Frontend typecheck/OpenAPI drift, README and 103 Markdown files
pass; 135 screenshot and nine controller hashes verify without recapture. Staged
heuristic scan of38 task files has no findings, not DLP/history certification.
The staged native helper/runner/probe hashes match the recorded native evidence.
Fresh Docker audit: complete=true, desktop35 containers,
runners0/0, violations=[]. Installed Fleet/native lost-ACK end-to-end, prepared
intent delivery, missed native tools/approval history, bounded SSE, safe process
quiescence/loaded config, assignment/first-step/PM tools/resume, compatible Forge
handoff/live deployment, live screenshots and release ordering remain required.
Fleet 000010/000011 must precede 000012, one new migration per task PR. No full
SDLC success, automatic rollout or merge readiness is inferred from these gates.

## Prepared Dispatch Restart Recovery (4 October 2026)

The new `prepared_dispatch` worker covers the crash window between immutable
journal commit and consumption of the submission permit. It scans at most20
records per five-second UUID-keyset cycle, excluding submitted/accepted/legacy,
failed/expired/drained/archived/task/PM records. Fresh health/protocol, original
origin/default-profile fingerprint/request hash and optional store epoch precede
the shared transactional claim. Its sole winner sends frozen original bytes/key
through the same ACK/session-readback helper as ordinary dispatch. A prepared
uncertain outbox changes only atomically with that claim; consumed permits never
reset. A post-permit unknown outcome retains pending delivery and capacity.

Final exec97351 exit0, Linux/Rust1.88/PostgreSQL17.6: 195 workspace library
+132 foundation PG/HTTP +1 authenticated approval SSE +3 isolated migrations
=331 distinct PASS. The repeated targeted five prepared tests and existing
claim-denial case are not counted twice. All-target check/strict Clippy/fmt and
regenerated Rust OpenAPI equality pass. The optional native renderer fixture
exporter remains ignored; previous actual native evidence is separate. The first
attempt failed to compile an incomplete test DTO fixture, was corrected and
cleaned up, and is not reported as PASS.

The five new tests prove: two fresh supervisors send one original body/key/run
and persist one terminal assistant without a second SSE; concurrent prepared
uncertain claims have one atomic winner; malformed202 followed by another
supervisor never repeats POST; rotated credentials or invalid fresh capability
leave the original permit/deadline untouched; drain/task/failed/expired/submitted
records are not selected or allowed a new submission. Tests use actual PostgreSQL
and controlled HTTP, not a managed native gateway/model. This packet creates no
new schema, public route, migration, dependency pin or Java execution capability.

Final log `.local/pdlc-implementation/sdlc-qa-fleet-prepared-d29bd5afdd06.log`
SHA256 `df7648f73c1fc75472eb1a50d1dea249ce335b43fd4c3f9b5eb67d48c7413632`.
Exact `sdlc-qa-fleet-prepared-d29bd5afdd06` finally cleanup and independent
container listing pass, preserving shared caches/accepted runtime resources.
Fresh Docker audit is complete: desktop56 containers, runners0/0, violations=[].
Frontend production sources are unchanged; typecheck/OpenAPI drift and135
screenshot/nine controller fixture hashes verify without recapture or promotion
to live evidence. README and104 Markdown files pass. The26-file task heuristic
scan has one exact unchanged committed-parent synthetic redaction-test literal
and zero new findings; it is not DLP/history certification. Remaining gates
include managed Fleet/native recovery,
missed native tools/approvals, bounded streams, safe process/config attestation,
assignment/first-step/PM tools/resume, Forge handoff/live deployment, production
screenshots and exact-head release CI. Fleet10/11 must release before12, one new
migration per task PR. PR47 remains Draft on its older remote source; its green
checks do not certify the accumulated local integration packet. No full SDLC
success, installed rollout or merge readiness is inferred from this component gate.

## Managed Native Supervisor (4 October 2026)

Final exec85937 exit0, project `sdlc-qa-fleet-native-3cd0c16348a2`: one explicitly
executed ignored Rust target passed in71.65s, zero failures/ignored cases. It runs
actual Fleet provisioning, enabled-skill content installation, renderer-2 draft
activation, supervisor, prompt outbox/adapter, terminal mirror and native restart
against two real gateway CLI/API/AIAgent processes. Only OpenAI model inference
is a deterministic loopback fixture. Owner rows use the real disposable database,
not Fleet HTTP auth; task binding/admission is deliberately absent.

Checks cover separate HOME/cwd/ports, non-root mode0600 dotenv, actual distinct
SOUL in inference, cross-agent token401, one inference/assistant per original
prompt, identical message-key replay, native run/session identity after restart,
no duplicate dispatch from a fresh observer supervisor and tracked parent stop.
This is not process-tree quiescence or cross-instance process-ownership proof.

Clean native source is `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3`, archive SHA256
`571fba4903d9094ade7f6d0ef5dfcee8c068e50f62e65f46611ec3ad65697e02`.
All13770 tracked bytes verify before inference. The unchanged dependency image is
`sha256:aeb97055b0f5aee433e29998eeafd8065b81e70d1fcb69345c520c2bfbf23777`.
An owned BuildKit source-only layer stages the clean archive on Linux filesystem,
preserving every dependency layer/venv and the `fleet-control` non-root user.
QA image ID `sha256:05b1fc3a7df12a0fe22dcd7e79c85a1a8b73694f4226e9b25d886699bbbe13b6`
is not an installed runtime. Base SDK is clean exact `9408802dfa978cba2f67162a49adca6f65851b01`;
launcher comes from committed Base `082f25675009b061bb0ba16e3d6828b3029c9481`,
SHA256 `75ad258e5901df8dc7eecff892f3f2d054a6b21fc49e4dc66755c5dc24fbf3d8`.

The Fleet parent was `74436fe6b02a700fbffb98b9a16a9c735e45e7d6` with a dirty
task-owned candidate, recorded honestly rather than as exact-head CI. Test source
SHA256 `96f01ef2c4f4d329f3f6ee4490bcb5930334c43553842c2eb2cb35a1391d5ee7`;
executed binary `c7235aa60960364887392b13b30554cbff002d51b5b48dc7f06ee26e4318a627`.
Harness hashes:

| File | SHA256 |
| --- | --- |
| `run.py` | `aa2486bb271e4df3a510d48e46d2bc8b3b3d5853e5dedce1c39b1c176b63a938` |
| `build.sh` | `90ba8ff898b1d6f54b1b0bd3c60b037026ed01f0d767964c511f854d6c0fbaaf` |
| `native.sh` | `866b9f6b9395f0515e5d9341dfbde182b822e2186c0db236deeba37a6a988bd4` |
| `preflight.py` | `34623a4309e1fe494a3fab7cea555bf57823e0a08009552ac7fc8fd8b72919cf` |

Evidence directory: ignored `tmp/native-supervisor-live/sdlc-qa-fleet-native-3cd0c16348a2-d3gir5l0/`.
Native log SHA256 `1fec30dec363b57656a3402acab687cf5d0a8f566c0c3e153bc9b220e735567c`;
build log `4a6493343ee24e28e22d6080a80c7a931284f86a7acdb570b8b76e38319a9e3d`;
evidence JSON `1c35c51956a8cdf71602ffecc9255dc3052cf27892d07a4d378f9472aa54386a`.
Rust1.88 fmt, all-target locked/offline check, strict native-target Clippy and
exact executable compilation pass. This is one opt-in native case, not another
331-case workspace regression run. Nine new host harness tests and15 existing
native-protocol safety cases pass separately, without Docker or live credentials.

Earlier owned projects `bbc3f32955cb`/`9abbf2a5d353` failed the unchanged60s
readiness deadline with Windows source bind mounts. A live observation in the
second run saw the gateway in D-state `p9_cli` wait. This supports a host-source
IO diagnosis, not complete cold-start performance proof. A first source-layer
build `0635880a1b63` failed because BuildKit interpreted a bare config image ID as
a registry tag; the runner now creates/verifies/removes a unique owned local alias
and verifies inherited layers. All these failures remain failure evidence, not
passing tests. The production timeout was not relaxed and no root override used.

Final exact Compose cleanup/independent ps are empty; both unique source/dependency
QA tags are removed and the accepted dependency image ID remains unchanged.
No published ports, accepted HOME/volumes/secrets/SDK pins or sibling product code
were changed. Production UI remains unchanged;135 screenshot/nine controller
hashes, TypeScript/API drift and Markdown links verify without live promotion.
README structural checks and three validator tests pass; Markdown verifier checks
104 files. The18-file scoped heuristic scan has zero findings, not full DLP or
history certification. Fresh Docker audit is complete: desktop52 containers,
sdlc1 runner1, sdlc2 runner0, violations=[]. Other tasks' resources are untouched.

Remaining: managed lost202/original-key recovery, missed tools/approvals and
offline controls, full native loaded-config/plugin inventory, safe descendants,
Fleet HTTP auth/UI, assignment/fencing/first-step, PM structured tools/delivery/
checkpoint/rebind, compatible Forge handoff/deployment and seven-agent SDLC.
Release ordering and exact-head CI remain mandatory; no full merge-ready or
installed runtime claim follows from this happy-path proof.

## Bounded Native Stream Consumer (4 October 2026)

The [consumer profile](contracts/HERMES_EVENT_STREAM_V1.md) is Fleet policy,
not a new Hermes capability or upstream durable cursor. Production code rejects
wrong HTTP/MIME/encoding, malformed UTF-8/JSON, missing/non-string/foreign run
IDs and conflicting event/session identity before mirror writes. The pinned
Hermes `gateway/platforms/api_server_runs.py::_run_event` emits run identity on
every data event; valid approval fixtures now follow that real envelope.

Incremental byte framing preserves split Unicode, BOM, LF/CRLF/CR, comments,
one colon-space and multiline data. Empty frames reset event names; partial EOF
never dispatches data. CRLF bytes within a frame count toward1MiB; its blank-CR
delimiter's optional LF is ignored only after dispatch. Budgets cap input32MiB,
8192 data frames, transcript1MiB and cumulative full-text snapshot text16MiB.
Idle60s, partial assembly30s and lifetime30min cannot be extended by keepalive
traffic; empty transport chunks do not extend idle time. Snapshot text accounting
is not a whole database quota. Failure retains original pin and capacity;
authenticated GET-only recovery independently proves terminal state, without
another POST/SSE consumer or automatic cancellation/config activation.

Final compiled source hashes, printed before the final regression commands:

| File under `backend/infra/` | SHA256 |
| --- | --- |
| `src/runtime/sse_wire.rs` | `2272325b2d4d8185d4b886aa0a70a67c63358a2eb243092b22cd9c1cd5c24c02` |
| `src/runtime/mod.rs` | `bf4cc3efcc3245fa9fcdccf0893b679ec8247280675b11e7bf9308c1cbb2eb3a` |
| `src/runtime/acceptance_readback.rs` | `67977aabc789d1c6471903103cb196f3fa38fa4628b9c43117f117c8667c5dc9` |
| `tests/sdlc_foundation.rs` | `5791ebaa3fc19a16672f76197bc21c32406960466589d311e97312e7e57103e9` |
| `tests/support/runtime_stream_bounds.rs` | `4ac7a17789384a7b786eaf5843c95df9c9c665fcd19a9ba250f52815c7373849` |
| `tests/runtime_approval_events.rs` | `7d13f5715c921bb275c76ae728fbf27f8919f177754a51e5fb731bac6f82e952` |

The new10 source units cover byte/counter/transcript/snapshot limits, framing
and independent deadlines, including empty chunks. Seven PostgreSQL/HTTP cases
use actual30s/60s timers, original-key replay/one POST, held capacity and exact
run/session pins. They exercise malformed/unbound/foreign delta/tool/approval,
split Unicode, truncated terminal, wrong transport and oversized frame/text;
rejected control events cannot leave an approval, tool mirror or delta.

Final regression exec56274 exit0, project
`sdlc-qa-fleet-stream-final-5800407eb471`: Linux/Rust1.88/PostgreSQL17.6,
205 workspace library +139 foundation PG/HTTP +1 authenticated approval SSE
+3 isolated migration upgrade/down/up cases =348 distinct PASS. No targeted
repeats are counted twice. The native-renderer fixture exporter remains ignored;
it is not another passing case. All-target locked/offline check, strict workspace
Clippy, fmt and source-generated OpenAPI equality pass. No Rust source changed
after this hash-pinned run. Log SHA256
`12e09c8a20eb0fde41a8e4395e398f17c0ea397ef5ecc8f15c4fbf95d2bc6f1b`,
ignored `.local/pdlc-implementation/sdlc-qa-fleet-stream-final-5800407eb471.log`.
Exact finally cleanup and independent ps are empty; external caches and accepted
resources are preserved. Fresh Docker audit is complete: desktop37 containers,
runner0/0, violations=[].

Node22 frontend typecheck/API drift,135 screenshot/nine controller hashes pass
without recapture/live promotion. README validation/three validator tests,
105 Markdown files, nine native-supervisor and15 protocol host safety tests
pass separately. The21-file scoped heuristic scan has zero findings, not DLP
or whole-history certification. Neither browser/live UI nor full Base gates
were rerun for this Fleet runtime-only change; Base source/plugin is unchanged.

Final native compatibility exec88348 exit0, project
`sdlc-qa-fleet-native-8c2e9beb04f9`: one named managed native test PASS in104.37s,
zero failed/ignored. Actual Fleet activation/supervisor/outbox/mirror and two
real gateway CLI/API/AIAgent processes run against a deterministic loopback model.
Every13770 Hermes tracked bytes match clean `bbaf7af5`; archive, SDK9408802,
unchanged dependency image aeb97055, non-root user and launcher bytes match the
preceding native gate. Launcher is read from Base Git `76fc0952f1bf8d7ae1c18ee6eee598957cf8ecd8`,
SHA256 `75ad258e5901df8dc7eecff892f3f2d054a6b21fc49e4dc66755c5dc24fbf3d8`.
The source-only QA image `sha256:a2125a0e293cbdfc26d2ee11116de1b137cf9130cee2a49e34d6f2608c78f6b6`
and unique dependency alias are removed after exact Compose cleanup and empty ps.

Native evidence directory: ignored
`tmp/native-supervisor-live/sdlc-qa-fleet-native-8c2e9beb04f9-cgt_f6dl/`.
Binary SHA256 `e1ccdc78ea629dd5eccead604ca8be566166ff8e1ada276160a7714f717bd646`;
native log `f93abae671e58e98fa3a30b5ce06bc61192bb482bab2e78af074ae63d7a390db`;
build log `6e5cd5cfe402286527974d85fa10811a8b3d7ac5ff1b6f2d241623a552a6d186`;
evidence JSON `c3d46321f4fcf68a7f376574c406bf57c69a15c1afe4d7d721ca60fbd2ae8ef2`.
The unchanged test/harness hashes are listed in the preceding section. Native
fmt/all-target check/native-target Clippy pass. Parent Fleet source is a9613a1
with a dirty task candidate, not exact-head CI or installed runtime acceptance.

Preliminary `sdlc-qa-fleet-stream-07a30b182863` failed strict Clippy on an
Option-returning test helper after passing component tests; this is not PASS.
`sdlc-qa-fleet-stream-e10836897070` passed its initial regression, but the empty
chunk guard and two new negative payload variants were added after its test
executables compiled. It is not final-source regression evidence. A first
native run `7eb35370f66a` passed64.72s before that guard; final evidence above
supersedes it. All preliminary projects cleaned up their own resources.

No public Fleet DTO/route, schema/migration, SDK/package pin, Java capability,
accepted runtime image/HOME or UI source changed. Remaining requirements include
missed native tools/approvals, offline control outcome reconciliation, durable
upstream replay, expired Fleet-cursor snapshots, safe process-tree stop, complete
loaded config/plugin attestation, task/first-step/PM tools/resume, compatible
Forge handoff/deployment, live UI and seven-agent SDLC. Resource retirement does
not close those gates. Migration10/11 before12 and exact-head release CI remain
mandatory; PR47/main and Base publisher PR140 are not changed by this packet.

## Managed Native Lost-ACK Recovery (4 October 2026)

Final recovery exec22937 exited0, owned project
`sdlc-qa-fleet-native-b9fbf9a6626a`: one exact named test PASS20.57s, zero
failed/ignored. This is the actual Fleet provisioner/renderer-2 activation,
supervisor/outbox/journal/readback against real Hermes gateway CLI/API/AIAgent,
with a deterministic loopback model only. Base recovery plugin bytes come from
committed `5f86c1fc4367f3fee5466cd0b9dc7155a3cefefc`, not mutable worktree files.

A separate QA platform middleware authenticates first, observes exact request
hash/key, invokes the real native handler and closes the connection only after
its actual202. No response ID or inference is manufactured. A lookup barrier
keeps the first Fleet process's journal submitted, native ID unknown, run pending
and capacity held. The transcript contains one prompt and the normal session
creation system event, not an assistant response. Fleet PID27 exits without Rust
destructors. The parent test has no supervisor: it owns only the local model and
coordination, verifies native terminal GET and releases the lookup barrier.
Fleet PID64 starts a new supervisor and restores the same
`run_f9334662044e47b3a9571a6343bae891` through real Base witness lookup and GET.

Assertions require unchanged exact request body hash, original key, Fleet IDs,
origin, credential fingerprint, capabilities, submission timestamp and recovery
horizon; one native POST/reservation/inference, one Fleet run and one final
assistant mirror. No native SSE request occurs, because this test deliberately
recovers an already-terminal run. A further6s observation retains the same mirror
and IDs. This is a real Fleet OS-process restart, not two supervisor objects in
one process; it is still a test executable, not installed Fleet HTTP/auth/UI.

Final lifecycle exec2451 exited0, owned project
`sdlc-qa-fleet-native-bb04111b8f6a`: the other exact named test PASS49.82s,
zero failed/ignored. It rechecks two homes/ports/SOUL, private non-root dotenv,
cross-token401, one prompt/assistant, native gateway restart identity and tracked
parent stop. Both gates use the same compiled binary SHA256
`866959839bb56e09cee90d1db6bab20d74047a0bde600049bd942f20ba3adb1b`.
Rust1.88 fmt, locked/offline workspace all-target check and strict native-target
Clippy pass in each build. The earlier348-case component regression is not rerun
or counted as new evidence: this packet changes only native tests/harness/docs,
not production Rust/API/schema. These are two distinct native cases, not three
cases from the recovery driver's nested test banners.

Common inputs: clean Hermes `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3`,
all13770 tracked source byte hashes verified before inference; archive SHA256
`571fba4903d9094ade7f6d0ef5dfcee8c068e50f62e65f46611ec3ad65697e02`;
SDK `9408802dfa978cba2f67162a49adca6f65851b01`;
unchanged non-root dependency image `sha256:aeb97055b0f5aee433e29998eeafd8065b81e70d1fcb69345c520c2bfbf23777`;
launcher `75ad258e5901df8dc7eecff892f3f2d054a6b21fc49e4dc66755c5dc24fbf3d8`.
Recovery preflight additionally verifies exactly four committed plugin files:

| Source | SHA256 |
| --- | --- |
| `native_supervisor_live.rs` | `e7377d129aedfcde0e4a9d8b20e03dc3a1442536aab518fd461c799a86581856` |
| `run.py` | `631861a868cb923aeb52fd0d1fcf37ac853548c35876774d47596b2651d04aa3` |
| `build.sh` | `90ba8ff898b1d6f54b1b0bd3c60b037026ed01f0d767964c511f854d6c0fbaaf` |
| `native.sh` | `ae3ae3fe0ce4f6c41946191e6e442547b1344eaad084db18d6f090429e0cd233` |
| `preflight.py` | `966550676d1f220853fe24d5d6345cb8d9998aea851c46c4d1882bc89c38c4bf` |
| QA `discard_ack_plugin.py` | `b3a44db060e88e4148683df1335c4ee7f26646ca31ad3812e5b93456645ef1d4` |
| host `test_harness.py` | `4c47572e45c8d13eadd8e14ed4ea3daf2f6ac9d73bc1367a0330638d87138362` |
| Base `__init__.py` | `985ef5ef48b1c3e6b6232055ffb77ee360ca854f1a4400e565dff04fe4f7ad63` |
| Base `plugin.py` | `cd9115db0949c00c2671e0ebc5e98f4860303456ec1f60d699d801f712d7f9dc` |
| Base `store.py` | `ef45cf58f2dc71f3ed262e3b584a4d2c0991988537895954915dc4d43d53bd0f` |
| Base `plugin.yaml` | `494a102a86b86308303e43c416d36b99372a9af2e6111f8ef5b7bed555e63b7f` |

Ignored evidence directories are
`tmp/native-supervisor-live/sdlc-qa-fleet-native-b9fbf9a6626a-ezast_jz/` and
`tmp/native-supervisor-live/sdlc-qa-fleet-native-bb04111b8f6a-cloz09ad/`.
Recovery native/build/evidence JSON SHA256 respectively:
`4beb4ed81f5d9c479e8ea339fa5380a9916c1147555ac49cd9009d416b72cd24`,
`40aef8ca765d15c8c3b9ab443451e7e8cdedb17844ab22f39aef7f12bf0dbadf`,
`4c8428ff40a3ce27cf54cb67b425074c191d03e227008aff5a1f002b7f96275e`.
Lifecycle native/build/evidence JSON SHA256 respectively:
`1e21735c1a21e9f1fa3305076ca0c04732951e9234b898d5dedbb4dced6152c4`,
`d6a149dc7930c58ae762e2527ba2685f33779ad00c79bb943c69f01bece75335`,
`7a0b1cb57b5fa16e96e2cb8f47239a46ec1f8ae64bfd933baab43cc5d13f3bb4`.

Reproduce via the native supervisor README command, explicitly selecting
`--scenario recovery` or `--scenario lifecycle`, exact clean source/SDK, committed
Base plugin/launcher and existing external cache names. Both finally cleanups
exit0; independent ps is empty and unique QA source/dependency tags are removed.
Orphan gateway descendants are reaped by the disposable Compose namespace. This
is not a safe-stop ownership-transfer or production OS isolation attestation.
Accepted images/HOMEs/volumes/secrets/caches and SDK/package pins are preserved.

Initial recovery exec97687/project8389fd184e97 failed an incorrect fixture
expectation of one transcript row: session creation already stores a system
event. It is not PASS (native log SHA256
`17b50a7e69a7cc190fc719afa4c4d5aaf42d85d12c3ce0dce4a006f5eec46a51`).
The corrected final case explicitly checks one prompt, one creation event and no
assistant before recovery. Initial host fault tests lacked the Windows-only
stub for Linux `O_NOFOLLOW`; unit-only flag emulation fixes host ordering tests,
not Linux path guarantees. All16 final harness/fault unit tests and15 earlier
protocol host tests pass. Frontend Node22 typecheck/API drift and135/nine fixture
screenshots hashes pass without recapture or live UI promotion.
README validation/three validator tests and105 Markdown link checks pass.
Fresh Docker audit is complete: desktop35, both runners0, violations=[];
only the exact owned QA projects/tags were cleaned, not other tasks' resources.
All seven staged test/harness blobs match the recorded executed source hashes.
The14-file scoped heuristic scan has zero findings, not DLP/history certification.

No public DTO/route, new migration, production control handler, accepted runtime
or Base plugin source changes. Remaining requirements include installed rollout,
running/waiting/native-crash recovery, prune/reset in the managed Fleet path,
missed tools/approvals and control outcome readback, safe process-tree stop,
loaded config/plugin attestation, task/first-step/PM tools/resume, Forge handoff,
live UI and seven-agent SDLC. Tracker source af6ed1e and Workflow2d79461 are
unchanged read-only prerequisites; reserved assignments still forbid dispatch.
Fleet PR47 remains Draft938b4ed; Base PR140 ready177edb8 has six successful CI
checks but is not merged/installed. Migration/release order and exact-head CI
remain gates; dirty candidate source evidence is not release certification.

## Native Run Control Hardening (4 October 2026)

Production steer/stop no longer accept arbitrary2xx/JSON or a caller-only run
context. `run_control` observes the original accepted journal by its indexed
unique run ID, verifies original bytes/key/origin/credential fingerprint and the
live native session, probes exact advertised capabilities and performs bounded
GET before POST. ACK requires HTTP200/JSON/identity encoding, at most64KiB and a
ten-second request/body deadline. No automatic retry or arbitrary upstream body
is exposed. Task-bound controls fail closed until verified control admission.

Steer reads current Fleet state after ACK and never writes running over a
concurrent waiting/stopping/terminal transition. Stop ACK may persist stopping,
not completion or capacity release. A full native terminal race ACK requires the
original run/session and validated flags; independent terminal mirror commit is
still required. Run-wide approval is now denied inside the adapter too, not only
the public route. Exact human request decisions retain their separate flow.
See [consumer profile](contracts/HERMES_RUN_CONTROL_V1.md).

Component exec78862 exited0, owned project
`sdlc-qa-fleet-controls-724c553b33bc`, Linux/Rust1.88/PostgreSQL17.6:
208 workspace lib cases,146 foundation cases, one authenticated approval SSE
case and three migration cases =358 distinct PASS. The ignored renderer case is
not counted. All-target check, strict workspace/all-target Clippy, fmt and Rust
OpenAPI byte equality pass. No public DTO/client/new migration changed.
Log `C:/git/azhukov/sdlc/.local/pdlc-implementation/sdlc-qa-fleet-controls-724c553b33bc.log`
SHA256 `67797fac28e5e7430c640023f4165057b3050ee476a10e9ad3a237cf14a9167e`.
The own Compose project was removed in finally without deleting shared caches.

Seven new PG/HTTP scenarios cover concurrent waiting after steer, stop-only
stopping/capacity hold, invalid ACK identity/MIME/encoding/202/oversize, actual
timeout without retry, foreign native session/stale original context before POST,
full terminal stop race without invented mirror, and legacy run-wide approval
denial before HTTP. Three unit cases separately validate ACK shapes and flags.
Fake-runtime tests prove consumer failures, not native behavior.

The seven executed component source hashes are:

| Source | SHA256 |
| --- | --- |
| `backend/app/src/lib.rs` | `fbf21c78ad3f02eb68ade2fe1b2eb1cc71cd9baac205b08ebe60865101f66340` |
| `backend/infra/src/lib.rs` | `e1e822622d7f7420c9015d7f28c06674de46fb3572b8bfc64566217765e71656` |
| `hermes_dispatch_journal.rs` | `d5bf1c9edbdab480a8949a2fd1419c8c24fe467b4d9d36be698abcb2ee77438c` |
| `runtime/mod.rs` | `632d0fe68f9e322cd64f6cc1a085e67e62c63ee3a8339becc12e507000b2de57` |
| `runtime/run_control.rs` | `a73c3ab54908df7da5b1d636f1412a3b0e191fe3a22bbfc1c4ad383851303dd7` |
| `sdlc_foundation.rs` | `2e2ef77186c08b8212f262990d2973d67a738128de553fe7f6c590ba50e687d3` |
| `support/runtime_run_control.rs` | `2d31b7e37228d086233ac29177a58d2d4acfa505b5017a10ce29deedbfd1bd97` |

Native exec60198 exited0, `sdlc-qa-fleet-native-9063c06ac6f8`: one exact named
real-AIAgent steer/stop test PASS16.34s, zero failed/ignored. Actual Fleet
configuration activation, supervisor and prompt outbox dispatch one run; only
the loopback model is deterministic. A model barrier holds inference while
authenticated native status records the real steer. Interrupt ACK is distinct
from terminal outcome; after release, the same run ends failed/cancelled, never
completed. Original journal and single inference remain, late steer is denied.
This is not a targeted approval, guidance-consumption/model-quality, safe OS
descendant stop or task/PM admission test.

Evidence directory:
`tmp/native-supervisor-live/sdlc-qa-fleet-native-9063c06ac6f8-nqbojlgp/`.
Native log SHA256 `20fd6aa6b39b0def552494c91974ec3478400bb641cdcb8e7f604828ab44fbb4`;
build log `69ab0fb39531d622b90a03d1b8bc5975e1a847d9bbc70bc7cf5d4508ba15733b`;
evidence JSON `65ce9d9ad32062814435aa22454b0c5d03fd061572d4b7ab940e133f2b3a3cfd`.
Binary `fa31bba2df225fc3d48359056933f7d38d68dd6c4ed8bb3707216ded22bc0ed8`;
native test source `de89d8134ab31a48edf2aadd16cb4a80a9b730a0239bf2762b3670c8b5c79f60`;
runner `56ae2f2fc4ad2995842f90259abf4abfeb0f3d9333758f16db067aa59d9d08eb`.
The existing build/native/preflight/fault fixture hashes are unchanged from the
previous section. Build gates include native-target strict Clippy and all-target
check. Cleanup0, independent post-cleanup ps empty, both unique QA tags removed.

Clean Hermes bbaf7af5,13770 source byte hashes/archive571fba49, non-root dependency
image aeb97055, SDK9408802 and committed Base launcher/plugin bytes are unchanged.
The launcher is read from Base10f2a428 Git, not worktree files. Reproduce the
native supervisor README command with `--scenario controls`. Dirty worktree
source evidence does not certify a published exact head or installed runtime.

Durable control-command idempotency/receipts, unknown control acceptance readback,
missed tools/approvals, running/native-crash recovery, safe descendants, complete
loaded inventory, task first-step admission/PM tools/resume, Forge handoff and
seven-agent SDLC remain requirements. Tracker af6ed1e and Workflow2d79461 remain
read-only producers with dispatchfalse. PR47 remains Draft938b4ed; PR140 remains
ready177edb8 with six successful CI checks, not merged/installed. Migration10/11
before12, release partition and exact-head CI remain gates. Component/native
success does not make the full task merge-ready.

Final sibling native executions use the same binary/source/runner hashes above:

| Scenario | Exact execution / result | Native / build / evidence JSON SHA256 |
| --- | --- | --- |
| Lifecycle | exec67340, project `sdlc-qa-fleet-native-bb91f91cbeb0`, one case PASS51.03s | `223d177006a482c66cc23153bd228bbf457cefb28db1759199cb9ac94bbaed9f` / `d28a60f8849cfc8c464360c0b21e4faf1f4a2503ca8e1fc07ef75eae24e4d9c5` / `cb13b22c7894382c69c045d4a0748293d4dbb84f14032d8227e91b9800a1a77e` |
| Lost-ACK recovery | exec94323, project `sdlc-qa-fleet-native-5fba8e950267`, one case PASS28.73s | `4b470d015a084e39cec80aa192d110ba402ba0fcfe264fc52ceb41ee5ac9be71` / `9ac5ff1b1b47615b669f17c811e7ec8aedfb967d4d7e80b7c767d9226f259e8b` / `ff06b10e8166955885040bfc0db8ea5aac60023f9f07d8f353b8559b0582d891` |

Evidence directories end `bb91f91cbeb0-2pthn5l_` and `5fba8e950267-zd5dixar`
under the native supervisor artifacts root. Both execs exit0, zero failed/ignored,
verify13770 source files and finally cleanup0/empty ps/own QA tags absent. Recovery
also verifies exactly four Base plugin files and restores
`run_523f3751637045d0b009f578e69ecf20` across Fleet PID27 ->64, one original
POST/inference/assistant without native SSE. Nested child test banners are not
extra cases. These are three distinct native cases including controls, not
installed HTTP/auth/UI or running/native-crash recovery acceptance.

Final Node22 typecheck/OpenAPI drift,135 screenshot presence/count/header checks
and nine controller fixture hashes PASS, without UI changes or recapture. The
old full-screen verifier did not check hashes; the earlier wording overstated
this evidence. Generated full-screen SHA-256 proof is added in the next section.
Sixteen native
harness,15 protocol harness and three README validator unit cases PASS;
README validation and106 Markdown links PASS. Fresh Docker audit is complete:
desktop77, both runners0, violations=[]. Other tasks' containers/caches and
accepted volumes/images/secrets remain untouched.

The24-file scoped heuristic secret scan returns one generic-secret match in
`backend/infra/src/lib.rs`, inside the existing
`streaming_redaction_withholds_split_credentials` unit test. Its literal exists
unchanged in HEAD fa91b77; the current lib diff only adds the repository method.
This is a reviewed baseline synthetic redaction fixture, not a new credential.
No scan rule/allowlist or fixture bytes were weakened to obtain zero. The raw
scanner exit1 is retained; this disposition is not a zero-findings scan or
DLP/history certification. Production source hashes therefore remain exactly
those exercised by the358-case gate.

All seven staged component blobs and staged native test/runner match the executed
hashes above. Final host test source SHA256 is
`91cbaa79dd3d95e63e209d7f363b301bff4dbc37fe225c8f506836aa34ebdfc3`.
This byte comparison prevents CRLF normalization from silently publishing
different code; it does not substitute for exact published-head CI.

## Durable Runtime Control Journal (5 October 2026)

Candidate migration000013 follows000012 without modifying historical migration
bytes. It stores immutable actor/key/payload hash and original native context,
one submitted claim, ACK/audit/durable-event transaction and bounded scoped
history. Identical retries read the same receipt without another native POST;
changed payload or scope conflicts. Unknown effects retain the run control hold.
Only independently committed accepted journal/prompt/terminal mirror proof can
mark `terminal_observed`; that state is not command ACK or safe OS quiescence.
Task/PM controls remain blocked. Java chat/control remains phase2.

The actor is derived from authenticated `CurrentUser`, not JSON. This alone is
not `VerifiedHumanSession` proof. Actual HTTP authorization denial acceptance,
generic control identity retirement and multi-instance/native unknown-command
recovery remain gaps; repository/unit evidence must not be promoted to these.
The API requires a bounded actor-scoped `Idempotency-Key`; receipts exclude raw
input/key/token. No blind-reset, automated resend or OS-stop proof is introduced.

### Failed Gates And Current Regression

Initial broad executions failed, and their passing subsets are not full gates:

- The credential fixture assumed migration11 was the latest. It now explicitly
  invokes the unchanged migration11 nonempty downgrade guard.
- A QA bare clone lost canonical Base origin metadata. The harness now restores
  the verified origin after cloning and rechecks the exact existing package pin;
  production provenance checks were not bypassed.
- A prepared-runtime fixture panicked on foreign authentication. It now returns
  HTTP401 before probes/effects; an additional PG/TCP case verifies zero counters.
- A PostgreSQL Hermes journal timestamp check failed after the VM wall clock moved
  backwards. The watcher independently observed repeated9-11second regressions.
  Historical immutable timestamps/guards remain unchanged. A separate earlier ACK
  failure lacks its primary diagnostic and is not attributed to clock drift.
- Exec64401/project `sdlc-qa-fleet-control-ledger-b37b4b15194e` passed209 library,
  153 foundation and four migration tests but failed the isolated approval fixture
  (old count13 vs actual14) and strict Clippy format arguments. Its exit1 and exact
  finally cleanup are retained. The approval fixture now compares exact applied
  version names to the registered migrator, not another hard-coded count.

Final component regression exec78899, own project
`sdlc-qa-fleet-control-ledger-121eb661400f`, exits0:209 workspace library tests,
153 foundation tests, one isolated authenticated approval SSE case and four
dedicated migration tests =367 distinct PASS. The ignored renderer export is not
counted. All-target check, strict workspace/all-target Clippy, fmt and Rust OpenAPI
byte equality pass. This scoped aggregate does not run every ignored integration
target, real-producer PM credentials or all native scenarios. The QA aggregates
independent outcomes, retains clock observations and cleans only its own Compose
services/network, preserving external caches. Component log SHA256 is
`d647935c052854df3c1ddd6778bc3e678f573b67c983ec606f8c23840f658086`.

The executed candidate sources have these hashes:

| Source | SHA256 |
| --- | --- |
| `backend/domain/src/runtime_controls.rs` | `10c02a93f8d93650e6dee8e3494de640e3a82a697affa2bd246b59e2bb42f156` |
| `backend/infra/src/runtime_controls.rs` | `95b9b7da5573e0f02afc7ffd6ac8ff1e729c4a8c864199d157574a123d05c488` |
| `backend/infra/src/runtime/run_control.rs` | `1d3c7509f4b8cce460d475fa2d5171dbddf1562c45c9b430fcf07c355ef95a94` |
| `backend/infra/src/runtime/mod.rs` | `7c23757edeac9a314dc864a9d7ee3324fa835253ab9f1f3ab168f66d353a86cd` |
| `backend/infra/src/runtime/acceptance_readback.rs` | `97ed6137b386aca158fba972ec11f4007c005d19d9666ebb94bc1cad151da633` |
| `backend/migration/src/m20261005_000013_runtime_controls.rs` | `9f91a0c79d4ec6bcbfd54b4290abb656ab6045967553d92c8789089bc4e7920a` |
| `backend/migration/tests/runtime_controls.rs` | `5c0175d7013acde9ed1d8a7a3a408d8b120980f822aaa5492ee4c08374ca68c0` |
| `backend/infra/tests/runtime_approval_events.rs` | `b40529467b3d7b1443e4a9005b2290f18d3642a7c8626d33c76eec75b06b86fd` |
| `backend/infra/tests/support/runtime_run_control.rs` | `856ee3bb45dddadd38c063bf46aa0307c1fd9d6f91fe8e16b512bcd1a9d2f65e` |
| `backend/infra/tests/support/runtime_prepared_recovery.rs` | `79a45055081f8dce66e375de89e01ecd5d841d3d6c3744781272b174dde27326` |

Supplementary project `sdlc-qa-fleet-controls-extra-ae43408a56ea` completed three
additional distinct PG cases: managed settings, scoped chat directory and central
subject coexistence. Clean migration CLI up/status/down-one/up/status succeeds
for all14 registered versions. Workspace doc tests succeed with zero executable
examples; they are not additional test cases. Together with the component suite,
370 distinct Rust/PG cases pass on this source. Supplement log SHA256 is
`b916fc4b9dc519bf7fbde53c260f568f1238758442fbc5b4c868af9cc5be231c`.
The tool was interrupted during observation; terminal log and independent empty
Compose ps confirm completion/cleanup. The earlier interrupted project3f770ea9
never reached QA execution and was removed by its exact Compose command before
retry. No missing process handle alone was counted as PASS.

### Native And Browser Evidence

Native exec75512/project `sdlc-qa-fleet-native-91e2f74ad7de` exited0: one exact
`managed_native_run_steer_and_stop_require_native_ack_and_terminal_readback` case
PASS22.19s, zero failed/ignored, two filtered. Real AIAgent guidance/status and
interrupt ACK use the same command receipts on replay; history persists through
terminal readback. Only the model is deterministic. Fake PG/TCP tests prove one
native POST under concurrent retries; the native case does not independently
count control POSTs. No native approvals, unknown-control acceptance, task first
step or process-tree ownership-transfer proof follows.

Evidence directory is
`tmp/native-supervisor-live/sdlc-qa-fleet-native-91e2f74ad7de-_er3pv75/`:

| Artifact | SHA256 |
| --- | --- |
| Native log | `1466ef2a7c0a1d10e4aa630c4e0be05839c13355b9808ca510d8c216d22bd7b4` |
| Build log | `416aa44aff9263946042200883be75570a56d8b0884cf6a8e5fb99579a15e329` |
| Evidence JSON | `7e823fb1b459b771f0a03ceb14967d7fae8cb8f08750f615a8a9863a3a8a8396` |
| Compiled test binary | `9f6c55871f2f4a349434eeaa4796508ae59324c82bb6d0d3bf7d326026a48d26` |
| Native test source | `2d6c4d35638f7fa1dde9b89018be66b33c40059306957990f58480a7798bfd51` |

Clean Hermes `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3`,13770 tracked byte
hashes/archive571fba49, SDK9408802 and non-root dependency imageaeb97055 are
verified. Launcher comes from committed Basee0d091a, hash75ad258e; runtime/package
pins and accepted resources are unchanged. Own cleanup exit0, independent empty
ps and removed own QA tags are recorded. This is dirty-source disposable QA, not
published-head CI, installed rollout or complete source inventory attestation.
Later test-only approval/format fixes do not change the native test or runtime
implementation; no later native rerun is claimed.

Node22 frontend controller tests passed235 cases. Exec1109 passed six fixture
Playwright cases (control unknown/reload and PM dialogue/clarification/requirements
across Chromium/Firefox/WebKit), not the entire E2E suite. Frozen client command
body/key live only in memory; reload reads server receipts and preserves the hold,
not a browser draft or recoverable secret key. Three control screenshots and nine
PM controller screenshots have separate hashes and `liveAcceptance=false`.

Full-screen capture previously returned0 despite missing chat mock endpoints;
visual inspection found an error banner. The capture now rejects every unhandled
mock API before saving a screen. The strict run next found missing workflow
catalog; the fixture was corrected, not silently accepted as404. Final exec75142
captures135 current pages at375x812/1920x1080/2560x1440 with linked JSON/Markdown
route/viewport/PNG-size/SHA256 evidence. Nine verifier negative tests pass. Current
mobile/desktop private chat and mobile unknown-control drawer were opened and
inspected; no mock error banner or incoherent overlap was present. This is fixture
UI evidence, not actual runtime/identity or all-page visual acceptance.

| Fixture manifest | SHA256 |
| --- | --- |
|135 full-page screens JSON | `20c7e3ee8613783b0c3ac98efaa1e1fe60d334a4192099f4176118893a2a8a9a` |
|9 PM controller screens | `72fc752300485f0b2d9d8ef4a3fc1540a6c353785e1f67edfb3c0a6b7aeb8557` |
|3 unknown-control screens | `8faa74146ff77fb18276b03fb9572ee40400e6c5db218f64b8c637c8a31c6494` |

Frontend lint/semantic/format, regenerated API equality, README and107 Markdown
links pass independently. The unchanged Base compatibility checker initially
rejects the two new required headers, correctly identifying a breaking change.
Fleet's explicit closed security-migration wrapper now checks their exact required
string shape on steer/stop only; eight wrapper tests deny weakened keys and other
breaking changes. API_VERSIONING documents unsupported legacy unkeyed clients.
This is a deliberate security migration, not ordinary backwards compatibility.
The existing706.74KiB main bundle warning remains,
not a performance-gate pass. The scoped heuristic scan retains one unchanged
synthetic redaction fixture in infra/lib; raw exit1 is reviewed, not zero findings
or DLP/history certification. No scanner rule or fixture secret was weakened.

All40 staged code/schema/script/CI files match the verified worktree bytes.
Three legacy `.mjs` files initially differed only through Git line-ending
normalization; they were formatted to LF and all17 compatibility/verifier tests
rerun successfully. Formatting and API compatibility pass on the normalized
files; no source or fixture rule was skipped to obtain equality.
Independent Compose ps is empty for the exact component and supplemental QA
projects. Final Docker grouping audit observes56 Desktop containers with no
violations, but cannot reach either registered runner endpoint: `complete=false`,
exit1. The earlier complete audit does not override this final observation.
Accepted/shared resources and other tasks' projects are not changed to repair
the unrelated runner availability. Full fleet grouping acceptance remains open.

Remaining: fresh HTTP route denial/live identity, native unknown-control/approval,
expired stream snapshot, loaded config/plugin inventory, safe descendants,
fenced task first-step admission, PM tools/delivery/rebind, compatible Forge and
seven-agent acceptance. Migration release order10/11 ->12 ->13, one new migration
per release PR and exact-head CI remain gates. PR47 stays Draft938b4ed; publisher
PR140 remains ready177edb8, not merged/installed. This packet does not close them.

### Publication Readback

The source packet `d976a82881315188a875dafe66ec973d18bd1691` was published by
regular fast-forward to `feat/hermes-runtime-integration-20261004`; remote SHA
readback matches. No main, PR47 branch, accepted runtime or dependency pin was
pushed by this task. This integration branch has no automatic push CI trigger;
local source/component evidence is not an exact-head release CI run.

Later remote inspection supersedes the pre-publication PR47 observation above:
another task published `5240107596ce9b645a4e30cd9b9506ecfc739905`, changing only
`.base-revision` to `6080e11fa9f59db00b102939867d2a97441d6cbb`. PR47 remains Draft
against main; its five CI checks are SUCCESS on that release head, not on this
runtime integration source. Its remote work is preserved without rebasing or
force-pushing the PR branch. Our component/native gates use SDK9408802, so release
dependency reconciliation and renewed exact-head validation remain necessary;
the green sibling pin-update CI does not certify migration13 or this runtime code.

## Human Runtime Control HTTP Boundary (5 October2026)

This follow-up to647ea572958d0be8e021d59e7f6f465805c5d637 changes the public
human stop/steer boundary, not native wire/adapter, schema, migration or runtime
pins. Both routes require middleware `VerifiedHumanSession` before session/run
lookup, like exact approval decisions. A sessionless principal cannot reuse an
admin role, authenticated local user ID or forged HTTP human header as proof.
Scoped machine control remains a separate admission contract, not a fallback.
API_VERSIONING records the deliberate authorization security retirement.

Two new PostgreSQL/HTTP tests execute the actual production handlers. One uses
issued HMAC login JWTs and actual `require_auth`: unauthorized401, unrelated
user403, foreign session/run404, unkeyed rejection, owner actor derivation,
same-key replay, changed-input409, authorized operator/admin receipt reads and
active-user revocation. Each successful steer and stop sends exactly one fake
Hermes POST across two requests; all rejected commands leave that counter and
journal unchanged. The other injects a sessionless admin only to isolate the
human-proof guard, including nonexistent IDs and a forged human header:403 with
zero runtime calls and no command rows. This is not actual central JWKS/PAT
acceptance, distributed machine fencing or live SDLC.

Final exec19550 exits0 with owned Compose project
`sdlc-qa-fleet-human-controls-925f1083ec4c`. Rust1.88/PG17.6 gate passes209 library,
155 foundation, one isolated approval SSE, four migration and three supplemental
settings/directory/central-subject cases:372 distinct PASS, zero failed. The one
renderer export remains explicitly ignored. All-target check, strict workspace
Clippy, fmt and byte-equal regenerated OpenAPI pass. Clean migration CLI
up/status/down-one/up/status verifies14 registered names, not a misleading count
based on the last ordinal. Workspace doc tests pass with zero executable examples.
No `GATE_FAIL` or clock-regression line occurs in this log; earlier observed VM
clock regressions remain unresolved, not declared fixed from one passing run.

| Evidence                         | SHA256                                                             |
| --- | --- |
| Component completion log         | `ab08b7046ccffd96318de5e43148467dffa75d169faedd81b2ba5fe4913fcff5` |
| Public session routes source     | `7f6a1695fd08072bdf8ce009d3c6454902e690c9429ea025f5f8f145d8cf8683` |
| Control HTTP/support test source | `f7fc0653acd1a21a168f5b7f5e9cb95c8bdf6ba3ec3f12cf73c2988e7f0f6d1b` |
| Own PowerShell QA launcher       | `93cc53636e0a917364657707b8d7e5977c288a4cd230540a4cf4f2501681f903` |
| Own scoped shell entry           | `f4871be593c1bc63f11b9427ac4c57738d7d920c7cb261b8fce930539e1f5a86` |
| Own Compose descriptor           | `10bfedf240720d86735f134e3819c9af13f2d6068383a9d6ebacbf390cf9a4a1` |

The launcher runs preflight format/check/generated OpenAPI, the existing complete
component recipe and the supplemental recipe sequentially against distinct owned
databases; all Rust commands are locked/offline. It retains external caches and
uses exact finally Compose down. Independent ps is empty. SDK9408802, private
package4b9b4c9, accepted images/volumes, other QA groups and read-only producers
are unchanged. New Docker audit checks37 Desktop containers without violations
but cannot reach either registered runner:complete=false/exit1. It is not full
grouping acceptance, and shared infrastructure is not changed to make it green.

Node22 typecheck, API equality and the existing explicit compatibility wrapper
pass; authorization semantics are not proven by schema compatibility.135/9/3
fixture screenshot hashes are reverified, not recaptured or promoted to live
evidence. Frontend source is unchanged; prior browser/Vitest/native results stay
at their recorded scope, no new native binary or browser run is claimed.
README/Markdown/format checks are separate document gates.

Remote PR47 remains Draft5240107/main with five SUCCESS jobs, no reviews or review
threads, mergeable/CLEAN at readback; that is not review approval or this source's
CI. PR140 remains ready177edb8/main with six SUCCESS, not merged or installed.
Base9408802 versus release pin6080e11 has no diff in Rust crate sources/Cargo.toml
or frontend sources/scripts, but Cargo.lock, frontend package/lock and selected
materialization scripts differ. Source similarity cannot replace exact dependency
checkout/build validation. No pin is blindly adopted or sibling commit overwritten.
Local runtime work now follows its own integration branch/tracking ref; the former
local feature ref and remote release branch are preserved, without force push.

Remaining: live central identity and machine/task scopes, native unknown controls/
approvals, expired stream snapshot, safe descendants/loaded configuration,
fenced first-step admission, PM tools/delivery/rebind, compatible Forge and full
seven-agent acceptance. Release partition/order10/11 ->12 ->13 and exact-head CI
still apply. This component packet does not make the full objective merge-ready.

## Managed Native Exact-Action Approvals (5 October2026)

The owned `--scenario approvals` gate now executes a real Hermes gateway/AIAgent,
actual terminal approval detection, request queue and tool, Fleet activation,
outbox/mirror, local issued HMAC JWT middleware and production decision/read routes.
Only the upstream OpenAI model is deterministic loopback. The native request is
not fabricated in Fleet/SQLite. One chmod command targets one disposable file
in the concrete agent's workspace, with explicit terminal cwd and private tmpfs.

Three independent owner chats verify once, deny and a lost exact successful ACK.
Modes change600 ->666 for once; deny retains600. Each decision has one native
POST/request/choice, resolve_all=false, two model calls, one final assistant
message plus distinct tool events. Original dispatch/run/session identity stays
fixed. Two same-key replays and GET keep the same decision and transcript bytes;
changed-payload returns409. Stranger403 and unsupported always422 issue no native
POST. After the third real tool effect/ACK, the QA-only observer closes transport:
Fleet retains uncertain after terminal and replay instead of claiming delivered
or sending a second approval. This does not recover the unknown native outcome.

The observer authenticates before body reads/effect/observations and requires
explicit opt-in plus an existing fixed Linux QA directory. Only IDs/choice/flags
are recorded, not command bodies, keys or credentials. It is embedded in the
test binary and installed only inside disposable HOME. Five new host tests
cover auth ordering, exact-once ACK loss, foreign/error ACKs, non-approval bypass
and root/opt-in denial; total21 host cases PASS. Linux O_NOFOLLOW/private file
mode is native evidence, not the Windows host-unit flag stub.

Final exec11165 exits0; owned project `sdlc-qa-fleet-native-f3f5f3a93010` passes
the exact qualified ignored test in11.30s, one passed/zero failed/zero ignored.
Rust1.88 locked/offline fmt/all-target check/native-target strict Clippy/build
pass. Every13770 Hermes tracked file is verified against archive571fba49 before
model startup. SDK9408802 and committed Base launcher5f7698 are unchanged.
Image runs as fleet-control/non-root, read-only root/drop caps/internal network,
no host ports, QA tmpfs. Exact Compose down exits0, both owned image aliases are
removed and independent ps is empty. Accepted resources/shared caches unchanged.

| Evidence | SHA256 |
| --- | --- |
| Executed test binary | `a7fe2a2fd9e3d27de9b812804bef399f8aa05785debf08d0032ad2f819867e96` |
| Native completion log | `ea043f7b068540dd4a761f6dc96e11d873c5bede3af5ac3224017eac8086df0e` |
| Main native test source | `013cbf0281ee7dc65ef036aee0bbb9ff73699d69ad87649943eaac4d4891fb1c` |
| Approval child test source | `9debe433f24979e6d58ae234ea8414753c53a478d6f54efceebf93d3afed887e` |
| Approval QA observer | `3fa21b195f8b707560e02e89b59df130f560925b1a8d8ef6c7fb0fa47ea76c0d` |
| Harness entry | `9db93d64daf454a55e9aea63b8c83e7f345d5fe5cddb0f3c07ca15c2a565a9ec` |
| Base launcher | `75ad258e5901df8dc7eecff892f3f2d054a6b21fc49e4dc66755c5dc24fbf3d8` |

The first native attempt ad0906f45345 failed because the new test counted all
agent-authored messages, including three valid tool events, as final answers.
It remains failed evidence: log352af3c19cba138267e2ccd7bcb3270d6e891cdd7b23ce70b185c7b936b08def,
binaryb3273dc652ed895e56b43d46e8e0c647a37962c141e08dcfec239c46c03a378b.
The corrected assertion requires exactly one AssistantMessage, separate ToolEvent
presence and full transcript equality after replay. No runtime protocol/deadline,
production approval code, pin or permission was relaxed. Its cleanup also exits0.
An initial formatter Compose descriptor lacked the inherited postgres declaration
and failed validation before starting containers; the owned descriptor was fixed.

This is executed dirty candidate source atopdb9c03f, not exact-head CI or installed
acceptance. The report now fingerprints the child test source as well as its
parent. The372-case component result above is not rerun or increased by a native
scenario; normal cargo test ignores it. Production UI/API/schema did not change,
so no new screenshot/live browser claim is made. Central auth, task/config-generation
admission, waiting-approval crash recovery/missed event replay, native unknown
decision lookup, safe descendants, PM first-step/tools/resume and full seven-agent
flow remain. PR47/main and PR140 are unchanged; migration release order remains.

Before publication, both Rust test blobs, the embedded approval observer and
run.py in the Git index are byte-equal to the executed worktree hashes above.
This preserves source provenance; it does not convert local QA into release CI.

Follow-up README validation,107 Markdown links and generated OpenAPI/client
equality PASS. Existing135/9/3 fixture screenshot hashes reverify; no UI source,
browser run or new screenshot capture is claimed. Final grouping audit checks54
Desktop containers without violations, but both registered runner endpoints are
unavailable:complete=false/exit1. Own cleanup is independently empty and does not
turn the incomplete full-group audit green. Earlier VM clock observations remain.

## Exact Approval Context And Delivery Lock Order (5 October2026)

Targeted approvals now reuse the same original accepted free-chat context guard
as stop/steer. Current Fleet run, concrete agent/origin, primary session, native
IDs, exact original journal hash and derived-credential fingerprint must agree.
Fresh authenticated capabilities must expose the exact approval endpoint;
authenticated bounded GET must report the pinned session and the currently
waiting exact request. The one decision POST requires exact HTTP200, JSON MIME,
identity encoding and a64KiB/ten-second bounded ACK identifying one action.
Legacy/task-bound effects fail closed: PM reservation is not admission. Durable
preflight failures retain uncertain without POST and cannot be retried after
availability improves. This is not native configuration-generation attestation,
distributed authorization/fencing or unknown-decision outcome recovery.

Four new journaled PG/HTTP cases cover native capability/request/run/session
denial, changed origin/credential/legacy/terminal context, bounded exact ACKs and
actual local JWT HTTP preflight hold/replay. Existing human approval HTTP tests
now use original accepted journal fixtures rather than unproven run flags.

The preliminary broad run exec54678 failed, not PASS: project
`sdlc-qa-fleet-approval-context-ad3efc58274a`, log
`b9d46ee17619e9e16e5c14e4ee3e4ad1221806a1a64c2285ae6ce5bb860e5609`.
It passed373 distinct cases and failed unknown-acceptance replay with PostgreSQL
deadlock40P01; the three supplemental cases and migration CLI did not run.
Clock watcher observed21-22second regressions, but they do not prove the cause
of the deadlock. Preliminary native exec32051 passed17.81s on pre-lock-fix source;
its log96e36920/binaryc4a64fe0 does not certify the later repository changes.

An explicit PG holder/barrier reproduced message-before-session inversion,
without relying on timing or PG's choice of deadlock victim. Before the fix,
delivery held message while its UPDATE/event FK waited for the held session;
session-owning dispatch could not acquire message with NOWAIT, code55P03.
Exec63651 failed on project `sdlc-qa-fleet-delivery-lock-848b2b184413`:
test log `796a9a9592c72b407771b09ef14f92b82379d43924bd8a52c8657e7aa3bc62fe`,
PG log `25535d5316c67e072d37e9a941fbd71e6dfafabe2ca40016cf971c03e186c2ae`.
Delivery now locks session with NO KEY UPDATE before message and verifies the
identity after waiting. Event/cursor updates remain atomic; unknown acceptance
stays pending/held. No migration, retry or weakened dispatch assertion is added.
The same test then passes exec25037/projecte55b711c5ad1 in0.45s, completion log
`cd435a50bd5d77f7e9efddc7da15ba3f585b9f04d9afc2d0ef5a17309cc06605`.
Both exact finally cleanup operations complete; later QA saves PG logs before
down. The regression prevents the identified inversion, not all possible DB
deadlocks or proof that Docker VM clocks are fixed.

Final production bytes pass two separate opt-in actual native scenarios:
exec92194/project `sdlc-qa-fleet-native-8a69944f6fa1`, approvals10.91s;
exec93493/project `sdlc-qa-fleet-native-03bdcf5eabdf`, controls9.45s.
Each executes one named ignored case, one PASS/zero failed/zero ignored.
Both use binary `8d5a29128c39d1119563be6048b7f20aba59a9d4b392a96aafc2567819c52f32`;
approval log `f9ca6645649f8e7c69f12bd8ea47d01f63ca7838d17b50e0fa9e77b640a0c701`,
control log `f48fe9a4e25ff750fee0e7ec9c7aa0318f32b4480e8dff06f0ba4e11dab2a5aa`.
They exercise real Hermes gateway/AIAgent/terminal behavior with a loopback model,
not PM/Tracker/Workflow, central JWKS, OS descendants or installed runtime.

Rust1.88 locked/offline fmt/all-target checks/native-target Clippy/build PASS.
Every13770 native source file matches pinned archive571fba49; SDK9408802,
Base launcher55d52eb/blob75ad258e, non-root dependency imageaeb97055 stay fixed.
Own source images/internal Compose networks/tmpfs use no host ports; both down
exit0, unique source/dependency tags are removed and independent ps is empty.
Report now fingerprints delivery/journal and context/control/approval source,
not just the native test. Generated credentials are never exported from tmpfs.

| Executed source | SHA256 |
| --- | --- |
| Repository delivery | `185d55b547b16bf0715ba21e89c6e5fbab953485949e8b22d1348f74ef2531ae` |
| Original journal (unchanged) | `d5bf1c9edbdab480a8949a2fd1419c8c24fe467b4d9d36be698abcb2ee77438c` |
| Runtime module | `1fcd9654eb364870baedc95114f0e79bfcebc6dd9a6caeec3324aa99e4f55742` |
| Shared accepted context | `9ad933a3c2f20b0093cc49e1661e85bc8ddafecd815bea6ecb398819c89fa6d9` |
| Stop/steer adapter | `ccd03e207d27c4f39258ce82506df3508e5ab9328fa3202eee0ce9b6a9cd7a55` |
| Approval adapter | `6f6bdc63f1201eaa1b0e1069d7a1c5624c84d3c663e66ca7fe2aae7ecdaa3eb2` |
| Foundation test module | `2a2dd4f61245e22f14a5e10e5adc115c4d1da9979d52fbfd46fab43498cbda6d` |
| Approval component cases | `dd486e07e04bd8aedf5ae1f31c4f908987218cc3f4a42d39af6dbca92bc50c45` |
| Acceptance/lock regression | `3e498c292c2a575cf6857e0a3c763591c82234638ccde0a58127deca06cfd4d6` |
| Native harness entry | `0aecd5341ee0909bea3c7560aca150cc64ffc0672713e7120388a1232ab0c140` |

Executed source is a dirty candidate atop1f94824, not a release-head CI result.
No DTO, API schema, migration, Base SDK pin or production UI changes here.
Unknown control/decision outcome lookup, waiting-approval restart/replay,
loaded generation/config inventory, safe descendants, task first-step/PM tools/
delivery/rebind, compatible producers/Forge, seven-agent flow and release order
10/11 ->12 ->13 remain. Read-only producers, Forge task2, other-worker PR47 and
Base PR140/main are preserved. This packet does not finish the complete goal.

Final broad exec55106 exits0 on owned
`sdlc-qa-fleet-approval-context-255620f98b1f`:210 library +160 foundation +1 isolated
approval stream +4 isolated migrations +3 supplemental cases =378 distinct PASS.
The single renderer export remains ignored; the two opt-in native cases above
are separately executed, never silently counted by normal cargo test. All-target
check, strict workspace Clippy, fmt, generated OpenAPI byte equality and clean
DB CLI up/status/down-one/up/status pass across14 registered migration versions.
Workspace doc tests run successfully with zero executable examples. The new
lock regression and original unknown-acceptance replay both pass in this run.

Completion log `a8f3cf680bde3cf2fb038aac24ceff9a51745eeaf11f018a18b19679650c3eca`;
owned PG log `e964f73992d55fdbe8827dcc88852f3d58e1bc49b06f7c7040880da95c18cd6e`;
own launcher `a6a4bdc26a353dab7eed50a20f73514681092d39d78c5e0f3a97e1ec31165393`;
own Compose descriptor `5e40dda27143f2b167d895bbe6a2ff0870b1029eaa614e4832873f3e17aa1044`.
PG logs remain ignored owned-fixture diagnostics, not exported transcripts or
tracked trigger statements; deliberate rollback/constraint errors are expected.
No deadlock40P01 is recorded in the final PG log. Watcher still observes20-22second
backwards VM-clock jumps. Exact finally down completes and independent ps is
empty; no caches/accepted volumes/images/other projects are deleted.

Host21 harness safety cases, Node22 typecheck/generated API equality, README and
107 Markdown links PASS. Existing135/9/3 fixture PNG hashes reverify, not new live
captures or visual acceptance. UI is unchanged; full Vitest/browser suite is
not rerun in this packet. Docker audit desktop41 and sdlc2-runner0 have no
violations, but sdlc1-runner is unavailable:complete=false/exit1. Partial audit
is not full infrastructure acceptance. Base only changes its source ledger;
its README/hub/diff checks pass, not a new full Base Rust/frontend/plugin gate.

Before publication, all nine changed Rust/harness blobs in the Git index match
the executed worktree bytes; every native runtime source hash above is rechecked.
Source provenance does not turn local candidate QA into release-head CI.

## Current Approval Snapshot Recovery

Implemented: authenticated GET-only restoration of the currently visible native
approval for an originally accepted pinned free-chat run. Fresh capabilities,
exact run/session/request and bounded action/detail are checked. One transaction
locks agent -> primary session -> run, rechecks journal/origin/credential and
commits redacted pending request plus running-to-waiting. Replay produces no
mirror message/event and does not reopen resolved/stopping/terminal state.
Task/PM, foreign identity and missing capabilities remain fail-closed.

Actual native exec36015 exit0/project6177e860ea8d PASS32.52s. Two distinct Fleet
OS processes, one surviving real Hermes gateway/AIAgent/terminal and loopback
model: first process loses real waiting GET and its only SSE, then exits before
mirroring the request. New Fleet restores the original request by GET, delivers
one local-JWT owner once decision, observes600 ->666 permissions on the owned
file and stores one final answer. Exactly one run POST, one original SSE attempt,
one approval POST, two model calls, unchanged dispatch snapshot and same gateway
PID/native IDs. Replay never repeats the decision or transcript. No fake native
approval/status/tool result is emitted by the observer.

Binary `3669f0747e4babdab3c9815a5f86637fd32ff0a9bb75e75be097af9594e10206`;
native log `ac62aca94d168b1d2896fe5e7c19669b2e51bc704b8919d9cf2add86c964f997`.
SDK9408802, Hermes bbaf7af/13770 source files/archive571fba49, dependencyaeb97055,
committed launcher Base414f68a/blob75ad258e are verified. Exact runtime/parser/
repository/test/observer hashes are saved in the owned ignored evidence report.
Cleanup exit0, both unique image aliases removed and independent ps empty.
The owned Compose namespace reaps the orphan gateway, not Fleet safe-stop.

Host25 safety tests PASS; final full Linux/PG regression is being rerun and is
not yet claimed here. Preliminary full run4f50a3acf24d FAILED: one existing
activation file-readback timeout, then three new test faults (15s global keyset
rescan assumption, PM FK without task binding, two DROP commands in a prepared
statement). Focused25aa807fd7ad then5PASS/1FAIL revealed an incorrect one-message
assumption; replay now compares the complete original transcript, including the
existing queue event. Isolated activationb83f5ce6feef PASS0.59s without code or
timeout changes. Preliminary failures remain failures, not final gate evidence.

Only the current request is recovered. Historical approval/tool replay, unknown
decision outcome lookup, task first-step/PM tools/resume, central identity,
loaded-generation/descendant quiescence, seven-agent acceptance and release
partition/exact-pin/head CI remain open. Approval recovery itself changes no
schema; the independent additive migration 000014 below repairs journal time
ordering. No API DTO, installed image, SDK pin, read-only source or Forge task2
changes. UI is unchanged; fixture hashes
or older release PR checks are not live UI or exact integration-head acceptance.

After migration000014, native exec34768/project56a2fe922775 PASS37.23s on
binary `2ce10fc5a68649b58f596e2a334d9fadc7e51681e1bfaec6d8313357a6f35a2f`;
log `3731ec70ce5710eabb29d9ee0fb05240c7bf2a064b982f439612a79f9ede64f2`.
The same two-process assertions pass with the registered additive schema.
Report fingerprints23 runtime/migration/test/harness files, rechecked against
current bytes, including the unchanged000012/000013 migration sources. Launcher
Base414f68a/blob75ad258e and SDK9408802 stay fixed. Exact cleanup0, both unique
aliases removed and post-cleanup ps empty. This is dirty-candidate source QA,
not release-head CI or installed acceptance.

## Journal Clock Order Repair

Second broad approval run exec2138/projectda34f8b771b9 FAILED:213 library,
165 foundation, one approval and four migration cases PASS (383 distinct);
one existing control case failed in fixture journal acceptance, before its HTTP
assertions. Completion log
`a3a76029ae5012d8ce2f433247b8b6275f58365fc16f73acc4bb1cbc60b9e6f7`;
PG diagnostic log
`c0f306550f27c6a173692333287446f9eff47107582a5dcf9d3eda0ce5a9796f`.
Primary error is journal check4: observed accepted time08:17:02.598558 precedes
submitted08:17:23.477559. This is a clock-order defect, not an HTTP retry or
deadlock. All six new approval recovery cases passed. Supplemental cases were
not executed after this failure. Raw private fixture rows remain ignored.

Deterministic exec19509/project244bfbd4bb23 before migration registration FAILED
check3 on submission. An owned insert trigger models persisted creation30s ahead
of the current clock with the unchanged86340s horizon. Test log
`c60a84ccbb3a7e6a360b98bf38bfd7532c7d3680b9d11dd4633ae976d4e905e4`.
The same regression after additive migration000014, exec9786/projectb44e767148c2,
PASS0.84s, log
`430783a10815c483fc10a421ba28db6bed096c8fc86a0778a6fef1e098547edf`.
It verifies logical submission/ACK order, original deadline, immutable replay,
one run/permit and refusal of nonempty downgrade without losing the trigger/row.

Isolated migration exec93232/projectda5157570db3 PASS0.59s, log
`6626b2a86d26a8243d9e91824e9cdd2a6aad715e9dfb1dba147ddd37229efb72`:
fresh prior schema, legacy user history and original guard are preserved;
actual trigger order, empty down and reapply are checked on their own database.
Both finally down operations exit0. Applied000012/000013 bytes are unchanged.

The added trigger follows the original identity/ACK/expiry guard and only floors
new submitted/accepted timestamps to prior progress. Creation, deadline, key,
exact request/hash and submission permit are unchanged. This supplies logical
progress time, not wall-clock stability, a trusted retention clock or native store
continuity. No automatic unknown redispatch is granted. Release000014 follows13
in a separate ordered migration packet. Pre-migration binary3669f074 remains
historical evidence, not the new binary.

Final broad exec89810/project5e5ce216eb45 exits0:213 library+167 foundation+
1 isolated approval+5 isolated migration+3 supplemental=389 distinct PASS.
All-target check, strict workspace Clippy, fmt and generated OpenAPI equality
PASS; clean DB CLI up/status/down-one/up/status PASS across15 registered versions.
Doc tests execute zero examples, not additional cases. Completion log
`f894ca6e126e3a8dea8c1fc118efc67dd32b3b7df34d2cc4a43f231d4cb9668c`;
owned PG diagnostic log
`9dfa7cd2e930ffca0b308c360a6dde422597407f76102126a9bbe0cf812b8078`.
All six approval recovery cases and the previously failing control ACK fixture
pass. Finally down exits0. Clock watcher still observes20-22s backwards jumps;
the passing gate does not repair VM clock synchronization. Isolated earlier
failures remain failures, never retroactively counted as PASS.

Node22/pnpm10.28.1 typecheck/lint/format,235 tests across31 files and build PASS.
Vite retains its >500KiB chunk warning. README/109 Markdown links, generated
client equality and135+9+3 fixture PNG hashes PASS. No production UI change or
new browser/live screenshot acceptance is claimed. New CI YAML wires migration14
to its own empty PG database; local YAML/step verification is not an actual
GitHub run. Ordered release/exact Base pin/head CI, task/PM/producers/Forge,
central identity, loaded generation/safe descendants and full SDLC remain open.

All five managed native scenarios pass on the same binary2ce10fc5, original
SDK9408802/Base launcher414f68a and exact Hermes bbaf7af archive. Exec29207 exits0
after the four supplemental scenarios; exec34768 covers approval recovery above.
Each runs its exact named ignored test, one PASS/zero failed/zero ignored, with
13770 native files verified; lost-ACK recovery also verifies the four committed
Base extension files. These are real gateway/AIAgent/tool executions with a
deterministic loopback model, not a live provider, central auth or PM workflow.

| Native scenario | Owned project suffix | Duration | Completion log SHA256 |
| --- | --- | --- | --- |
| Current approval recovery | 56a2fe922775 |37.23s| `3731ec70ce5710eabb29d9ee0fb05240c7bf2a064b982f439612a79f9ede64f2` |
| Once/deny/lost real approval ACK |90c7174e0780|15.66s| `49ff593dd382649f955ef8879625ef57ce4894d82141120b3fa3f1d77ca80b59` |
| Steer/interrupt/terminal readback |046d42ba44ee|13.20s| `4e1d467a1c07757065d89154f3c4a0f79629c92e5577344ece97d8234dcac51d` |
| Original-key lost-ACK recovery |93412b0c20e9|18.93s| `efea7a1fc174d12cc1e43fdcddc5f3af84301d7dfe3d9a6689ade26185c1a41c` |
| Two-home lifecycle/restart history |481e16d4e58d|43.86s| `d16aeb8b16017cc95e30c41e7ceb1e34fae151a082254c96edbb287a23a52755` |

Every report's23 executed source/test/harness hashes match current bytes.
Approval repository `5b68fb56ec6e6bcb35ba029d968f0d1a8cd5ac544165cb31ceeb22be7b574d63`,
snapshot parser `78b842f5635feba0c89509770c0e927df95fd3da7d9ca6b166b23cfd04b13f5f`,
readback `ad8c5123d672074e2a26f212d41ccb298dc6d44355738cb6e5ae22706e5f4bdf`,
migration14 `5172b6f5a22cd90c18ebc8925eb24ac890cf1de96539f728b42059a72721bce1`.
All exact-project cleanups exit0, unique source/dependency aliases removed and
post-cleanup ps empty. Independent broad-QA ps is also empty. Shared caches,
accepted images/volumes/secrets and other workers' resources are preserved.
Docker groups audit: desktop58 and sdlc2-runner0 checked, violations empty;
sdlc1-runner unavailable, complete=false/exit1. This remains a partial audit.

Production UI is unchanged. Existing durable approval events invalidate the
production task-approval queries, but no new authenticated browser acceptance
of the recovered request is claimed. No read-only Tracker/Workflow/Hermes/Forge
sources, Base core/pins/launcher/plugin or accepted runtime are changed. PR47
Draft5240107/main and Base PR140 ready177edb8/main checks belong to those heads,
not the integration candidate. Release partition and exact-pin/head checks
remain blockers; neither this packet nor its fixture evidence completes SDLC.

## Linux Managed Configuration Persistence: 5 October 2026

The activation journal was already fsynced before effects, but managed file
rename/unlink and newly created directory entries were not all persisted before
effective-head acknowledgement. `configuration_disk` now fsyncs files before
rename and parents after rename/unlink, synchronizing new ancestors leaf-to-root
under the existing guarded agents root. Apply and rollback use the same barriers.
Persistence failure is unavailable, not an active revision. Private staging
files can survive interruption; they are not public receipts or adoption proof.

Six filesystem component cases cover nested creation, replacement, unlink,
Unix0600, missing/foreign/non-file paths, symlink aliases and test-only failure
after visible rename/unlink. The actual PG lifecycle regression injects a
post-rename directory barrier failure and verifies retained journal/drain, no
effective head, no runtime spawn and no second activation claim. Injection is
task-local and absent from production builds. It is not a physical power-loss
test, interrupted-activation takeover, loaded-config attestation or descendant
quiescence proof. Windows directory durability remains unverified.

Preliminary exec86163/project6ef3a4ad74db FAILED: the delayed issuer fixture
compared expiry against pre-request wall time, and later formatting changes were
not yet normalized. All seven new persistence regressions and167 foundation
cases passed, but the overall gate is not green; supplemental cases did not run.
Clock watcher observed20-22s backwards jumps. Cargo additionally waited in
`jbd2_log_wait_commit`; disk free space remained adequate. This was a live IO
wait, not permission to restart Docker/delete shared caches or duplicate a test.
Finally down exited0 and independent ps was empty. Completion log
`95cff11f6d5bf962f33f9970c4ee8b50716168251b6c7d28b783899e9090d25b`;
PG diagnostics `4280b7926cd12d2584242fedde690d890cf0e9cb533ebd0e753c10436a81fe2f`.

The issuer delay test now uses monotonic elapsed time plus exact fixture
post-delay issuance/expiry, not two VM wall-clock samples. The production
predicate is unchanged: fresh nonexpired credential and receipt time + requested
TTL +5s ceiling. Deterministic UTC vectors test the exact ceiling, +1ns/expired
rejections and why using request-start wrongly rejects a delayed valid receipt.
This improves test evidence; it does not repair host clock security/retention.

### Final Candidate Verification

Final broad exec52692/projectf4ee8b3e7b20 exits0:221 library+167 foundation+
1 isolated approval+5 isolated migration+3 supplemental=397 distinct PASS.
All-target check, strict workspace Clippy, fmt and generated OpenAPI equality
PASS. Clean PG migration CLI up/status/down-one/up/status passes across15
registered versions; doc tests execute zero examples. Completion log SHA256
`8cc87aae5b15c8ec0de6d1d499a77d7b140d0f185657b1ae208ee7194c2e21a4`;
owned PG diagnostic log
`d5ec0f4055d2ca21dd28538147ec68a67ab63e52225b7d9f9312f7cc8c8611fe`.
Finally down exits0 and independent exact-project ps is empty. The preliminary
failure above remains failed evidence; VM clock integrity is still unverified.

Node22.20.0/pnpm10.28.1 typecheck/lint/semantic/format,235 tests in31 files,
build and generated client equality PASS. The seven-schema recorded Tracker
snapshot and its two Node cases pass; this is not exact remote producer parity
(see below). Screenshot checks pass135 general+9 chat+3 control fixture PNGs
and nine manifest cases. README,109 Markdown links and25 host native harness
cases pass. No production UI changed and no new live browser/screenshots are
claimed; Vite's >500KiB chunk warning remains.

All five managed native cases pass with exact executable SHA256
`5be030234f70dade504d581686acd681d3357237af8f75d6b92c8fb62c93a56a`,
SDK9408802, Base launcher88b9519 (unchanged launcher blob75ad258e) and pinned
Hermes bbaf7af archive571fba49. Each verifies13770 native tracked files and
26 current source/test/harness fingerprints. The deterministic model runs on
loopback; these are real gateway/AIAgent/tool executions, not live provider,
central authentication, PM workflow or safe OS process-tree acceptance.

| Native scenario | Owned project suffix | Duration | Completion log SHA256 |
| --- | --- | --- | --- |
| Two-home lifecycle/restart history |18f90f9804a8|40.93s| `2a782a737054b692953604bdf8795461af71690b5c83e29f46e388ffee4016e3` |
| Original-key lost-ACK recovery |315bd8da3e98|17.06s| `a8c95568a560eac1d8429bc9a1ea77901bbd902aebb7496727e999ca75344073` |
| Steer/interrupt/terminal readback |c9889222edb0|13.35s| `e7a685719b14ce5a12fb129eb2372186e16c69a66886f6c4faacdfcec80a9469` |
| Once/deny/lost real approval ACK |597375bc2841|14.37s| `8d514ad9b779a352b1126b39f3593962e7ebf5b68bff519ec466e2e3bb8f43cc` |
| Current approval recovery |d6104cddeda8|31.51s| `69ad79e0eec21ab4b662ad1c9e491e229a481025ee7d82115d065ee4abf7c338` |

Every report has cleanup exit0, removed disposable image aliases and empty
post-cleanup ps, independently rechecked. Shared caches, accepted images,
volumes/secrets and other workers' resources are unchanged. Executed new
configuration disk source hash
`4f7e249078e8a7a7191f53b874a763785c9bcac6d6417f067aba8b1b7be31815`;
credential source
`329f6c1da84551a040b0d77a3e6075ddf3b11d75c352fd99cedaa6bf4640c341`.
No migrations, producer sources, Base pins/plugins or accepted runtime changed.
Integration-branch evidence is not PR47/head5240107 or Base PR140/head177edb8
CI, release partition, installed-agent rollout or full SDLC acceptance.

### Exact Producer Release Audit

Fresh GitHub PR list/read and Git fetch confirm the only open Tracker PR114 head
`8c80a41fae3bf1c10439ddb7e536b05bf320340d` and Workflow PR90 head
`e4fba60f55aaefb2fa62cb2d6c151e075d7d5b37`, both Draft against their approved
main/master bases. Tracker's exact committed OpenAPI differs in three of seven
accepted schemas: TrackerTaskContext, TrackerConfirmation and
ConfirmRequirementsRequest. Current Stage excludes Analysis; ConfirmCommand
has only content_hash/key and rejects the optional routing extension. The
divergent local Analysis/routing modules are not this release producer.

Workflow's exact remote excludes local Base admission/binding modules. Its PM
contract still binds a running callback after dispatch, not predispatch first-step
authority. Sources are read-only; snapshots were not overwritten to mask drift.
Fleet snapshot parity alone is not cross-service release compatibility. Actual
admission/structured tools/answer delivery/checkpoint/rebind and SDLC rollout
remain closed until compatible producer heads and live integration are verified.

## Native Control Outcome Producer: 5 October 2026

Base source head `a48e53ee37a5a38b189f8a9cce84686d18fc5c83` is published in its
existing integration branch; exact remote SHA is verified. Producer source
bytes are attested below; no SDK pin or installed image was changed.

Base's explicit control plugin reserves the original scope/key/run/operation/raw
SHA before one native handler and commits only exact successful ACKs before
transport. Private producer-owned `fleet_controls.db` is not Hermes SessionDB.
Unknown handler/commit/crash outcomes remain held. Duplicate POST never invokes
the handler; original-context GET returns acknowledged or uncertain without an
effect. Native auth-first/default bearer and strict once/deny decision scope apply.
See [wire](contracts/HERMES_CONTROL_OUTCOME_V1.md) and ADR0024.

Final native exec63552/projectc111ac20a3d4 exits0: two real APIServerAdapter/AIAgent
cases,50.093s. A proxy consumes an actual successful steer/interrupt response
and closes without sending it downstream; original-key GET verifies exact ACK
before/after gateway restart, one model inference and no handler replay.
A native unknown run gives a durable uncertain hold before/after restart;
changed payload is409 and no model work occurs. Model is deterministic loopback.
Expected model fixture BrokenPipe after hard interrupt remains in the log.

Clean pinned native bbaf7af archive SHA256
`571fba4903d9094ade7f6d0ef5dfcee8c068e50f62e65f46611ec3ad65697e02`;
non-root immutable dependency imageaeb97055 unchanged. Completion log
`171d940574783c28fc79f6430a71442af2751a2975dd0f4fe16ac9d1ea59e3f6`.
Producer plugin.py `261904c4db1b2833ef2e14f2a884caad198bbb3847dbf3e2587770e679f62e37`;
store.py `50f3d591f33d793ac4fca7256dd54e41785911d8d131429b39ea5ac0ce7b5b48`;
probe `9fd5c4146c8cb9687659e490f98879347f0d58f72cc115abf9f2156b4aba6b17`;
runner `a147aff33e1aa4631b9b4105b0e4b2962a850e0c45418cb54475af2cd7034b0a`;
native helper `243ae5cc57d9ecb3fde3da4fa114da9a294bb53d0daddcda6ddfd29c7bcfbbe7`.
Every executed plugin/harness hash matches current bytes. Cleanup0, independent
exact-project ps empty. Earlier source checks (discarded reply and before bounded
clone) passed but are not substituted for this final actual transport-loss gate.

Final Linux component project17cfc8925837 passes21 cases with no skips:
single/concurrent claim, exact replay/conflict, ACK commit failure, restart,
foreign/moved/reset schema/epoch, capacity, Unix0600/link guard, auth/no-key,
bounded body/chunked clone and exact-action approval packets. Component approval
ACK checks are not a positive native terminal-tool decision through the plugin.
The Fleet host protocol harness passes17 cases; no Rust/public API/schema/UI or
SDK9408802 change, and the preceding397/235 gates were not rerun for QA/docs only.

Base's mandatory Rust1.88 fmt/strict all-target Clippy and63 tests with disposable
PG pass (exec7549/project33cf8c077554); two live JWKS/three doc examples ignored.
Completion log `2ba55bf6dd0657cdb17eedbc602ee913936e7e4e6a720c651df89888daf40bf8`.
Node22 typecheck/lint/13 Node+83 Vitest, README/hub/unchanged mirror gates pass.
Existing recovery plugin runs56 host tests with one Windows symlink skip.
Owned Linux unit/Base/native QA cleanups and independent ps are empty; shared
caches and accepted images/volumes/secrets remain untouched.

Production Fleet does not yet persist producer epoch/raw body context, send the
headers or reconcile outcomes into receipt/audit/events. Never enable this plugin
on installed Fleet or retrofit historical intents. Consumer integration, combined
extensions, positive native approval, image build/installed acceptance, original
task/PM admission, OS containment/descendant stop, producer compatibility and
ordered release/exact-head CI still remain. No new browser/screenshots or full
SDLC merge readiness is claimed; PR47/PR140 and their pins remain unchanged.
