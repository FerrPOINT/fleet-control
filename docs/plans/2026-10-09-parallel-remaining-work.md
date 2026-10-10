# Parallel Remaining Work: 9 October 2026

Status: 10 October checkpoint; parallel implementation/review and parent-owned critical path. This document
records work ownership, not completion or permission to deploy.

## Current Dispatch

### Latest Assignment Checkpoint

These assignments supersede the historical table below. Workers use separate
owned checkouts; Tracker and Workflow remain read-only references.

| Owner | Independent work | Acceptance boundary |
| ----- | ---------------- | ------------------- |
| Pascal | Close two frontend-control P2 findings | Independent review of60eee4d reproduced malformed PNG acceptance and an unbounded pipe-reader join. Authorized successor edits only helper/tests in the frozen frontend-control checkout; preserve workflow/source/gates and add negative regressions. No heavy jobs or publication before parent review. |
| Feynman | Narrow C11 credential release candidate | New owned checkout from the reviewed47/64 dependency baseline. Extract only credential modules/hunks, migration11 and related tests/docs/CI fromafc5bb4. Preserve configuration semantics; no migrations12..20, heavy jobs or publication before review. Backend controlsaa5ac3f remain frozen. |
| Curie | Production Chats UX regression closure | New owned checkout at34858b3; write only chat-detail components and their focused tests. Verify draft retention, unsaved navigation, stale/conflict input, explicit choices and keyboard/focus behavior; fix reproduced defects or add missing regression coverage. No backend/schema/lock/control edits or live-PM claim. Frontend controls60eee4d remain frozen. |
| Anscombe | Completed selector correction6c022f3 and current-client captures | Two files/five added lines; lint/format/typecheck/focused test/build and six browser cases pass.27 fresh fixture captures; own processes removed. No backend/schema or live acceptance claim. |
| Parent | Critical path, integration and publication | Authenticate active backend38018281445 on reviewed controls9e838718/source1303be6. Prior38017066472 has authenticated config_files_unit failure evidence. Review frontend-control fixes before publication; integrate worker candidates after review and publish narrow release units after their own gates. |

Ptolemy's codegen work is complete: run37999711562 succeeds and artifact11648708483
contains authenticated schema874230b2. Parent independently reads it back and
integrates the generated API/type alias. Final integrated Rust parity is still
required; code generation is not runtime acceptance.

The prior diagnostic and backend controls are frozen. The new assignments above
are independent: Pascal now owns only the two reviewed frontend-control fixes,
Feynman uses a separate credential-release checkout, and Curie owns chat UX changes
in a new checkout. Parent owns generated API/client files,
integration and this ledger. Historical worker scopes below
are complete, not concurrent assignments. Deliverables identify exact commits,
commands, results and bounded blockers rather than repeat completed broad audits.

The current parent codegen bootstrap is the separate build-only commit
5e57d5be967418530918b72c1e457d92a5df7588, with product source8c93f43,
eight exact input blobs and282 canonical exported files. Its15 pure tests,
source/export qualification, four shell syntax checks and diff check pass.
[Run38015043570](https://github.com/FerrPOINT/fleet-control/actions/runs/38015043570)
completes SUCCESS. Parent authenticates artifact11656020208, schema1167220e,
containing both configuration routes and all four clarification command routes.
Client generation/typecheck/drift and compatibility with mainb750e7b pass;
all earlier paths/DTOs remain unchanged. No schema/client is manually combined.
Publishing this build-only
bootstrap does not make the combined product merge-ready or certify runtime/PM.

Actual backend [run38016562420](https://github.com/FerrPOINT/fleet-control/actions/runs/38016562420)
completes FAILURE at Clippy on exactef3b474/sourceb0ad56c. Parent authenticates
safe artifact11656123142/ZIP0ce97501fabd6d423035cfbc73b64e9adcbe41b14eeeb2cab7c7ceb09d5da525,
two diagnostic source locations122/131 and successful scratch/synthetic DB cleanup.
Normal sourceafc5bb4 fixes only those panic expression chains. Reviewed normal
successoraa5ac3f starts [run38017066472](https://github.com/FerrPOINT/fleet-control/actions/runs/38017066472),
now terminal FAILURE on exactafc at config_files_unit. Source-to-controls delta is exactly
six additions; all prior ignored identities and migration registries remain.
Controls pure evidence: Linux110/110; parent Windows108 PASS/two POSIX-only skips.
These do not accept the product. The two run/source pairs remain distinct;
neither Clippy result resolves historical recovered-activation PG62.

Parent authenticates run38017066472 artifact11657125102/ZIP
d00558cf68d03fec24bb1c43a13b478b040f4a75d9e50de9d1a24d27e3e2716e.
The named failure occurs before the special-entry safety assertion, at Unix
socket bind. A lightweight WSL probe reproduces overflow with the long fixture
path and succeeds with the compact path. Source1303be6 changes only fixture
directory naming/exclusive creation; formatting passes, product Rust execution
remains pending. The next controls are prepared in a new parent-owned checkout,
leaving Feynman's frozenaa5 checkout and ongoing C11 extraction independent.
Normal successor9e838718 is independently verified against380 canonical compiled
files,170 Rust files, all81 stages/167 ignored identities and unchanged migration
registries. Windows108 PASS/two POSIX skips and Linux110/110 PASS qualify controls
only. Public fast-forward starts actual run38018281445, confirmed in progress;
do not restart or retarget it before terminal result and authenticated readback.

Current regenerated-client frontend suite passes337 tests/36 files using threads
and one worker. The default local fork attempt was stopped without a test result
after repeated worker-start waits; it is not counted as a pass. Curie's standard
hosted frontend gate must still run. No live PM or physical runtime is inferred.

Integration dependency: Pascal's diagnostic patch and the new API artifact can
advance independently; parent reviews both before final control qualification.
Feynman's inventory preparation need not wait for either, but dispatch does.
If the diagnostic exposes a production defect, fix and requalify the source
before executing the final combined gate. Local heavy jobs remain held by the
resource guard; no worker may bypass it or alter shared Docker resources.

The split review finds source688 lacks nine PR64 files, including workflow,
package and API modules. No current configuration acceptance is inferred from
PR64's separate CI. Normal integration has actual shared Rust/docs conflicts;
the generated schema requires a fresh Rust receipt, not a manual union. Missing
local Git ancestry09b35f1 is found in the existing canonical codegen object store
and restored by a targeted normal local fetch; no history rewrite or foreign
working-tree change is used to make the merge proceed.

Current gate: [38011797295](https://github.com/FerrPOINT/fleet-control/actions/runs/38011797295),
now terminal FAILURE at container_activation_pg62. Parent authenticates safe
artifact11654672637/digest889a5ed28791e8d43c0fb5075dcad58d316433181b7bfa88d2396be412163999;
scratch and synthetic DB cleanup pass. message_order19 passes by sequential
ordering. Positive authorize locations107/133 identify the next diagnostic scope,
not a root cause or permission to weaken recovery predicates. Controls preserve all74 stages,9 journal
cases, canonical21/split24 migrations,167 ignored/294 ordinary cases and all
dependency/API pins. Source-to-controls delta remains exactly six additions;
gate.sh and init.sql are unchanged. The fixed-hint parser now clears previous
panic detail at thread/test framing boundaries. Independent counterreview closes
the synthetic false-attribution P2:14 focused cases and9 independent vectors
pass. Parent full helper suite runs98 cases,96 PASS/two Linux-only skips. These
are control-tool checks, not product Rust/PG or live SDLC acceptance. No hint
alone establishes the cause of an earlier hosted failure.

Integration order: review and integrate product corrections; publish the narrow
PR successor independently; merge the final product source into backend controls
and retarget its exact fingerprints; run the full gate; integrate the browser
regression and inspect screenshots. Native lifecycle and real cross-service PM
acceptance follow only after their prerequisites pass. Unavailable upstream
admission/checkpoint contracts and the unqualified Compose helper remain explicit
external blockers; no worker fabricates those producers or edits read-only
Tracker/Workflow repositories to hide them.

Parent ports the approval ownership correction as9025d42. The new guard applies
before binding/receipt access; the existing test adds foreign-central denial
before reservation and after delivery, with owner/legacy-admin positives. Parent
preserves its accepted-runtime fixture rather than reverting to the older PR
fixture. Rust formatting and diff checks pass; actual Rust/PG regression execution
and successor CI remain pending.

Publication checkpoint: PR47 is normally fast-forwarded to11f97aa, base main.
Its fresh CI38002207012 passes all five jobs: backend, containers, minimum Rust,
docs and frontend. Parent reads the actual PostgreSQL log: the extended approval
case passes within37 foundation cases. Fresh metadata is CLEAN/MERGEABLE, with
zero reviews/comments/review threads. These checks belong to that narrow PR head,
not the combined runtime assembly. It is not merged by this checkpoint.

Final backend controls4113950 normally merge product469dad0 and preserve all
earlier history. Source-to-controls delta is exactly six additions; parent
independently verifies actual final source hashes/declarations. Worker Linux90
PASS/zero skips and Windows88 PASS/two Linux-only skips cover the controls, not
product execution. The workflow summary now matches its source checkout. Normal
publication3600cb7 ->4113950 starts actual run38002746761; the full Linux/PG step
is confirmed running. This gate has74 stages,9 journal cases and canonical21/
split24 migrations. No backend success or credentials-PG fix is inferred before
the terminal result and authenticated readback.

That run38002746761 is now terminal FAILURE at credentials_pg, exit101.
Artifact11650212282 (ZIP SHA-256
bb8ecca71739063181fb8686bdf944d57fc3937ca3d56e3228a0a94146f3c3fb)
is authenticated by the existing failure readback. It names only the reviewed
credential database-guard case and source469dad0 location
backend/infra/tests/support/pm_credential_creation.rs:459:5. Parent reads that
exact Git source: the assertion calls down(Some(1)) on the wrong latest migration.
Scratch and synthetic DB cleanup both pass; no raw private diagnostics are
published. This is the concrete failure addressed bye0d541e, not an invented
credential-runtime root cause. The full gate still has not passed.

Browser worker completes the existing explicit-answer/confirmation scenario in
Chromium, Firefox and WebKit and retains27 fixture PNGs (three tabs, three sizes,
three engines). Parent visually inspects mobile clarification and desktop
dialogue/requirements. The new uncertain-command reload scenario also passes
all three engines. Review corrects its mock stage from nonexistent Clarification
to contract Draft in54fa466; targeted reruns pass in all three engines, with no
skips/retries. Parent ports the two test commits as3f540a4+aabd711; focused
ESLint/Prettier, typecheck and diff checks pass. Browser application evidence is
scoped to3b41/7cc, not new generated-client or live-PM qualification. Own temporary
servers/browsers are removed. All three assigned worker deliverables are now
complete; combined CI/native/live acceptance remains parent-owned.

Parent also prepares test-only correctione0d541e: the credential downgrade case
must select credential11 by registered name, not whichever later migration is
last. It now requires the exact recovery-guard refusal, unchanged version/
applied_at ledger and intact credentials. Independent source review and fmt/diff
pass; Rust/PG execution is pending. This is not yet the authenticated root cause
of the old37999665761 failure; the new38002746761 authenticated artifact now
confirms the same stale target in its source469dad0. Neither run accepts the
later correction or browser-test integration.

Read-through finds one analogous last-migration assertion in the retained Hermes
journal clock-regression case. Independent worker5337997 (sole parent5fb9dc8)
changes only support/hermes_dispatch_journal.rs: select applied time-order14 by
name, require its specific recovery refusal, preserve version/applied_at ledger
and every existing retained-journal assertion. Parent reviews the actual SQL guard,
normally fast-forwards that commit and passes rustfmt/diff checks. Real Rust/PG
remains pending; no historical migration or production guard is changed.

The product source branch feat/runtime-docker-integration-20261009 is normally
published at5fb9dc8 before that successor; it is not a broad release PR or a
merge-ready claim. The subsequent successor and updated exact-head controls
must be published normally after review, without cancelling or replaying a live
run. Task Tracker and Workflow remain unchanged/read-only.

Final sourcef8b9a58 and reviewed controls2b6ae27 are now normally published.
The controls normally merge that source and change only its two Rust test
fingerprints, TESTING fingerprint, three declaration locations, aggregate and
source refs/summary. Parent independently verifies canonical source inputs and
the exact six-file controls delta; worker Linux90 PASS/Windows88 PASS+two skips
are preserved. Actual run38004567349 is confirmed in progress on2b6ae27 and
tests sourcef8b9a58; it is not a blind retry of469dad0. All74 stages,9 journal
cases, canonical21/split24,167 ignored/294 ordinary and dependency/API pins remain.

The browser worker separately qualifies the current generated-client assembly:
own build of sourcef8b9a58, then the same two scenarios/three engines and fresh
27 fixture captures. No source or schema edits are assigned; this task is still
in progress, so the earlier3b41/7cc evidence is not relabelled as current-client
acceptance. Production/live PM and native gates remain separate.

Subsequent terminal checkpoint: run38004567349 fails at foundation after the
credential PostgreSQL stage passes. Existing authenticated readback accepts safe
artifact11651341407, digest
41743dfb3c2b6cef64fe648eeea049dad25f0c17b0f99fb834e058dee3b58caa,
and names only the reviewed approval-history project-revocation case at2126:5.
Owned scratch and synthetic DB cleanup pass. Parent corrects the missing request
key in that test and the production stop/steer authorization order: project
revocation returns403 before the assignment409, without runtime journal effects.
The same case retains history, approval replay and SSE assertions; no gate is
removed. Independent review and next exact-head retarget are separate tasks.

The current-client own build/sourcef8b9a58 passes six fixture browser cases and
retains27 fresh captures. Parent inspection finds missing SDK styles, including
an unreadable requirements selector and clarification disabled button. Therefore
the worker's initial functional report does not close visual qualification.
Anscombe checks only the owned SDK junction/build layout and corrects the
evidence; source changes require a demonstrated product defect first. No live
PM/backend/native acceptance or production-manifest update follows from these
fixtures.

Source140234b is normally published with the project-control correction and
documentation. Feynman's independent actual-diff review finds no P1/P2; free
chats return before Tracker I/O. Pascal normally merges that source into4a8f71f,
with exactly six controls additions. Parent verifies all163 Rust Git blobs,
the changed compiled inputs and canonical aggregate9f0bb1ed. Gate logic,
dependencies and coverage are unchanged; Linux90/Windows88+two skips pass.
Normal controls publication starts actual38006625294, confirmed in progress.
No full PostgreSQL/backend acceptance is inferred before terminal readback.

Anscombe proves that the Windows SDK junction omitted Base Tailwind utilities;
an actual clean SDK19a directory restores those bytes. Six fresh browser cases
pass,27 captures retained, own processes removed. Parent independently views
mobile clarification and WebKit desktop requirements: header/filled buttons are
restored but the revision select still has white-on-white text. The worker now
owns a minimal local select-theme correction, existing regression checks and
fresh screenshots. No redesign, backend change or production-manifest rewrite
is authorized by this narrow assignment.

That UI assignment completes as6c022f3 (sole parent140234b), only two files and
five added lines: local semantic native-select classes plus assertions in the
existing exact-confirmation test. Parent normally merges it asd5f18d0, preserving
the intervening documentation successor. Frontend tree13ccbaba exactly matches
the worker's rebuilt/tested tree. Worker full lint/format/typecheck/build, focused
one-case regression and six browser cases pass;27 fresh fixture PNGs are retained.
Parent views WebKit1920/375 requirements, confirming readable selected revision,
and independently runs all42 chat-detail tests, typecheck and focused ESLint PASS.
No production screenshots, full light-theme/native popup, live PM or backend
acceptance is inferred. Existing hosted gate still tests exact source140234b;
the later UI/docs do not alter its Rust, migration or generated API inputs.

Terminal checkpoint38006625294: FAILURE clarification_pg, exit101, after the
credentials/foundation/clarification domain and API stages pass. Worker and
parent independently authenticate artifact11651638800, ZIP digest
69c966b128a41bb14d612484ded8912e0739745d1c47e3b9163c58a496f7ebac,
against controls4a8f71f/source140234b. Scratch and synthetic DB cleanup pass.
Canonical exports of the six unchanged control Git blobs avoid Windows CRLF
transport drift; readers/validators are not altered and temporary exports are
removed. Safe diagnostics name only expiry and HTTP reload custody cases at
support/clarification_custody.rs309/683. Both are their empty-run assertions,
not lease/authorization/dispatch failures: create_session already inserts a
pending primary placeholder. Parent's test-only correction snapshots that exact
baseline, requires pending/no native IDs, then compares all serialized run fields
after the existing recovery flow. No prior authority/body/lease assertions or
coverage are dropped; production code and migrations are unchanged. Independent
review and corrected-source PostgreSQL execution remain separate gates.

PR64 source compatibility is independently analyzed at60ff01e, dependencyPR47
11f97aa and actualmainb750e7b. Direct-main merge conflicts in six inherited files;
refreshing PR47 first merges cleanly and already contains actual main. The config
unit remains exactly32 paths with stable patch-idd1c3391; pins/migrations/locks
are unchanged. Feynman prepares only a normal candidate in his own clone. PR47
is still OPEN and remains a release dependency; old PR64 CI cannot accept a new
head. No combined Docker/native/admission tail is imported into this config unit.

Publication checkpoint: parent independently authenticates380066 failure and
reviews the corrected baseline; source5d91b13 is normally pushed. Pascal creates
normal controlsf34ed9b, parents4a8f71f+5d91b13. Only the clarification test and
TESTING compiled fingerprints, one declaration location, aggregate and source
refs change. Exact six controls additions,90 Linux/88 Windows+two skips and all
74/9/21/24/167/294 counts remain. Parent reviews the delta, independently verifies
all163 Rust Git blobs and codegen binding, normally publishesf34 and confirms
actual38008974895 is in progress. No actual full backend/native PASS is claimed.

PR64 dependency refresh820a1af is also normally published after parent tree,
patch-id and no-migration/UI/pin/lock checks. Metadata is main-based, Draft,
MERGEABLE, with zero reviews/threads. The remote body preserves the existing
three headings/comments/checklist and clearly distinguishes historical18-gate
evidence from current source/light checks. Fresh exact-head CI38008810511 now
completes SUCCESS on820a1af: all five jobs pass. Parent independently verifies
head/jobs and zero review threads, then updates only PR64's body/checklist after
remote template preflight. Feynman's watch is complete; Pascal still owns the
separate full backend run. PR47 remains OPEN and a release dependency. No force,
rebase, main merge or unrelated PR update is performed.

These tasks have disjoint write sets. Reviewers do not modify frozen owner
checkouts. Shared runtime/admission, live PM and native acceptance remain explicit
later gates, not completion claims or duplicated assignments. Generated API must
precede final client/contract verification; backend compilation must precede
physical lifecycle and cross-service acceptance. Tracker and Workflow remain
read-only. No worker may bypass resource, secret or cleanup guards.

Journal3b41 has parent-executed typecheck PASS and36 frontend test files/337
tests PASS (including42 chat-detail cases). Client generation used the existing
checked-in schema; it does not prove the four new journal routes are generated
from Rust. Journal Rust/PG, reload against live services and PM resume remain open.
Combined7cc normally merges3b41+d43f801, preserving parent runtime fixes and
both histories. Parent verifies byte-identical frontend, rustfmt and8/8 pure
boundary cases. No product-wide/backend/native acceptance follows from that merge.
Dedicated codegen controls7792bfc have parent15/15 pure PASS and seal/source
review. The previously absent build-only branch is pushed normally; Ptolemy owns
actual exact-head outcome and authenticated artifact readback, not invented schema.
Actual codegen37999711562 is now SUCCESS; parent authenticated readback passes
on artifact11648708483. Productb249ee5 integrates schema874230b2 and generated
alias; client generation/post-generation typecheck/openapi:check/Prettier/8 pure
cases pass. Fresh focused React rerun fails at worker startup before any test,
not a product assertion or a new42-pass receipt. Final Rust/PG/browser remain open.

Native-cut c89df468 independently closes both prior ready-publication and
restart-window findings at source/pure scope. Owner Windows87 PASS/11 skips,
Linux22/22; independent Windows8 PASS/3 skips, Linux9/9. Physical crash,
Rust/PG and actual Docker filesystem/recovery checks remain outstanding.

Hosted run37993165800 is now terminal FAILURE in preflight, not in progress.
Container initialization and restricted source checkouts passed; no backend
stage success is established. A real Git archive CRLF transformation was
reproduced and replaced with exact Git blob export without changing expected
hashes. Lightweight Fleet193 + SDK85 source parity passes with owned scratch
removed; Auth89/full70 are not proved by that probe.

Subsequent actual hosted37995542617 completes FAILURE in real compiler check at
container_controller.rs:18:48 (E0599, DatabaseConnection clone under SeaORM mock).
The existing safe-artifact readback authenticates run/attempt/head/digest and
owned scratch/DB cleanup; no raw private logs are uploaded. Product373682a fixes
three test files using separate isolated connections while retaining transaction
concurrency assertions. Controls0709588 normally merge that source, retarget only
its exact fingerprints and preserve all gate counts/pins. Independent review,
Linux76/76 and Windows74 PASS/two skips pass; normal FF publication starts actual
run37996397284, which completes FAILURE at check (E0277 in activation deadline
comparisons). Safe artifact11648090014 readback and owned cleanup pass. Normal
sourcecf86d20 replaces both expressions with the pinned Chrono cross-timezone
duration API, preserving max(deadline-now,0)+10ms. Reviewed controls a78cbf6 retain
all counts/pins and retarget only that source/file fingerprint/aggregate. Normal
FF push starts run37997222729, now terminal FAILURE at Clippy after all-target
check passes. Sourcea95ea81 replaces the registration version or-pattern with an
equivalent inclusive range and extends its existing boundary test. Reviewed
controls851c355 are pushed normally. Actual run37998388510 also passes check and
fails at Clippy in activation/preparation. Authenticated safe artifact11647977851
reports successful scratch/DB cleanup. Independently reviewed source9a57d11
removes three redundant test borrows and moves a byte-identical test module after
production items. Controls3600cb7 preserve all counts/dependency pins and update
only source/file hashes/aggregate; Linux76/76 and Windows74 PASS/two skips pass.
Actual37999665761 completes FAILURE at credentials_pg, exit101, after check,
Clippy/Auth/runtime inventory/realAuth/API/credential units pass. Owned cleanup
succeeds. The previous compiler-only failure artifact does not cover this test
failure, so its assertion/root cause is unknown. Parent controls7a3333d add
allowlisted test IDs/Fleet panic locations only, with strict failure readback;
80 Linux and78 Windows/two skips pass. Raw private logs remain unpublished.
This is diagnostic progress, not product test acceptance or full70 completion.

Read-only existing sdlc1 browser pass confirms agents/chats reachability and the
current-owner empty state; no task/run/message was created. Existing deployed
images are not qualified candidate images, and no live PM detail is proved.

The renewed parallel split keeps the existing workers and their isolated
checkouts rather than restarting completed work:

### Predecessor Review Checkpoint

- Parent fixture/lineage fix0be22c7b3f32f1547bce907b6a30c0ae108fd877 is
  independently closed at SOURCE/PURE level:16/16 loader/utility cases, zero
  skips,11.091s with the reviewer's base Python. A preceding PATH-venv attempt
  retained three subprocess timeouts; it is not represented as a pass. Rust
  compilation, PostgreSQL and native execution are still not proved. The
  subsequent d306 source closure is recorded below, not inferred from this check.
- Exact Forge34c independent review closes the inventory adapter and reviewed
  process-group custody path, but finds two P1 timing defects: initial source
  proof is outside the enforced budget, and an expired one-shot assertion alarm
  leaves finally unbounded. Six actual Linux process proofs, six adapter tests
  and24 aggregation tests pass; three timing regression failures reproduce the
  defects. Ptolemy owns the normal successor, Anscombe its independent closure.
  The prior arithmetic is not a proven total upper bound. Actual full12 was not
  run and publication/dispatch remains held.
- Exact PMdc6 independent review reproduces a P2 evidence bug twice: child
  exit0 can yield PASS with missing/FAIL test receipt. Pascal owns strict receipt,
  inventory and provenance validation plus negative tests; Leibniz owns closure.
  Parent execution of the actual offline packet separately terminates FAILED at
  its unchanged90-second child deadline after16 oracle and four native probes;
  retained report run-e35178825a60 confirms scratch absence. Its performance
  cause is not established. Neither this failed run nor the earlier worker pass
  closes the missing producer barrier/custody/live PM acceptance.

All three follow-ups have separate owners and were dispatched. Closed source
findings do not certify an unexecuted compiler, database or live runtime gate.

| Owner    | Current assignment                                                 | Required handoff                                                                                                              |
| -------- | ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------- |
| Ptolemy  | QA image-candidate provenance/OCI descriptor-chain successor | Nativebc0 review completed15/15 with two OPEN P2. Narrow image-runner source separates historical baked inputs from intended QA inputs and retains actual local OCI descriptors. No heavy build or pin replacement. |
| Leibniz  | Stopped: selected model capacity error | No backend implementation made by this worker. Parent owns the isolated custody/deadline successor; do not keep a stalled critical-path assignment here. |
| Feynman  | Native-cut ready-publication and restart-window successor | Fix both bc0 P2 in new normal source; preserve df657/original nine scenarios,55s withholding/60s product timeout and strict evidence. No native launch or silent pin replacement. |
| Anscombe | Independent source closures complete | Public-safe Forge1db118/118, backenda3 scoped79/79 and ad893db registry-only7/7 closures complete. No full12/backend acceptance inferred; exact runtime results still need separate readback. |
| Pascal   | Completed Fleet/Tracker source qualification and docs handoff | b249/Tracker357 seven DTOs match;39/39 offline checks, no Rust/PG/live HTTP. Docs51ceb normally integrated; no product edit or invented original-key readback. |
| Parent   | Backend diagnostic CI, normal integration and scoped publication | Independently closed a3 published normally; run37992541106 failed before source checkout on Docker Hub rate limit. Reviewed immutable-mirror successorad893db pushed normally; run37993165800 in progress. Actual70-stage acceptance remains open. |

### Fleet/Tracker Wire Source Qualification

Exact Fleetb249bc895e5160fe13383c49d42a24c9308852b3 versus immutable Tracker PR114
357caa7a60a717eb7b0ac72f286b793326992931 passes39/39 offline selectors:20 real
exported Fleet comparator tests, nine synthetic schema-oracle tests and ten
lexical source probes. Seven canonical Git OpenAPI DTO pairs match; required/
nullable, closed nested fields, integer bounds, Analysis, typed response validation
and owner-before-replay source are checked. Fourteen canonical inputs match
before/after. No Cargo generation, Rust/PG/live HTTP or native acceptance follows.

Workspace evidence `.local/fleet-tracker-wire-b249-review-20261009/REPORT.md`
and `evidence/run-9cc4c4b66890/terminal-receipt.json` retain the scope/raw logs.
Receipt SHA256:9f00f0de0900ae20971c3383f9ea73aeca1281d8ee860072f32c62f7fd5a6610.
Packet seal SHA256:f69e81fe085f458c22579c77c6b60cbdf2456c4cd13fdfcd8f74be4a85314ca9.
Measured host Python/jsonschema is an offline oracle, not a qualified Hermes venv.
This extends only SOURCE parity beyond historical PR47 evidence.

Real clarification GET returns `questions[].answer`; saved answer ID/request ID
is not the original answer-command key. `/clarifications/{question_id}/answers`
is POST-only, not an exact-command GET receipt. Authorized same-key/body POST
replay exists, but durable reload/key custody and GET-only unknown-write recovery
remain separate gaps. No producer/Workflow endpoint or capability is invented.

### Retained Base815 Fake-Contract Evidence

New distinct execution `run-020e27bd131b` at exact Base
`815982b648652b2665be9442bf395fc56c7e11d3` passes224 tests with zero skips,
exit0 in44.041s. This is fake-contract evidence, not actual CI or native Docker.
The immutable workspace evidence root is
`.local/base-815-linux224-capture-20261009/run-020e27bd131b/`:

| Retained file | SHA256 |
| ------------- | ------ |
| `full.raw.log` (42,692 bytes) | `52e85c0a93b1aacb38585f7a4c87f6a9c84d665159ff82f472da858d03378221` |
| `terminal-receipt.json` | `dac80a5db4b024119b63d01db8ac83be099eb988e9c382a716131a3eacc5f5fa` |

Raw tail, hashes and retained receipt were rechecked read-only: exact224-name
inventory, source/parent/canonical-input parity, native Windows Git qualification
of14 inputs, empty owned process group and scratch absence. No suite was repeated
for this docs update. Historical88.807s/60.841s remain distinct; these new files
do not reconstruct their missing raw logs. Preflight FAILED receipts are unchanged.
Private Base CI billing and all combined backend/native, producer credential
custody/pre-model admission and external Workflow acceptance gates remain open.

### Current External Dependency Readback

The latest parent remote audit records these legacy producer dependencies;
neither PR is merged:

| Dependency | State/base | Exact head |
| ---------- | ---------- | ---------- |
| Workflow PR90 | Draft/master | `9b4107f0e8c886f37b4bace9921c15921b6f1604` |
| Tracker PR114 | Draft/main | `357caa7a60a717eb7b0ac72f286b793326992931` |

Separate namespace Workflow PR99 (head prefix220afe) and Tracker PR126 (head
prefixfaa9db) do not close the legacy PM producer/admission gate. Private Base
PR180 stays Draft at815; retained CI37968542132 remains billing-denied before
steps. The latest parent host observation is about3.5GiB available commit RAM;
local heavy is not admitted. Fresh capacity/ownership checks and explicit
admission remain necessary; no prepared invocation is an execution grant.

### Current Frozen Handoffs

- Public-safe Forge1dbedf85242c3540b70005ce5f0c20badb682c41 has sole
  parent25be and no unsafe controls ancestry. Parent checks25 components, lock,
  report hash, exact HEAD/tree/parent and clean checkout. Seal SHA256:
  e71bed719ff25477adb2d82145069a6df2bac618c7b110febe8daf1a49cece20.
  Owner118 Linux pure cases and independent118/118 bounded source/lifecycle
  review pass. Independent packet
  `.local/forge-public-safe-independent-review-20261009/qa/seal.json` SHA256:
  9b22eb7db202603dcdf85310072fbbb6c50359b136be736399cd160b71a0cbd3.
  Parent separately verifies its six sealed review files. All12 budgets
  and source265 remain; maintenance commit is null, so authenticated private
  delivery and actual full12 remain blocked. No push or dispatch occurred.
- Native-cut helperbc0b85ee47ceb159daf304b25665d1e1cb70a689 has sole
  parentdf657. Parent checks all27 sealed files, raw hashes/Git blob IDs and
  exact HEAD/tree/parent/clean checkout. Seal SHA256:
  06b072bc5adcfe827c59c3f463cc002ca38503ca1621e3cbdebcb514a7f6c584.
  Owner75 pure cases pass. Genuine original-stop ACK is withheld before Fleet
  phase-CAS; recovery then exercises A -> next B -> failed C -> current B rollback
  with peer checks. This is implemented QA source, not native execution.
  Ptolemy's independent review passes15/15, including two reproductions of OPEN
  P2: ready exists before its JSON is complete, and restart proceeds120s after
  ready despite the55s withheld-ACK window. These are QA orchestration findings,
  not demonstrated product/native false PASS. Feynman owns a new successor;
  neither finding is closed. Report:
  `.local/forge-feynman-cuts-review-20261009/qa/closure-bc0b85e/REPORT.md`;
  review seal SHA256:77197f64897a888faa21ef29f79f274225237b75f58b677f95b597f80bb20c10.
  Locked compile, Linux filesystem and physical timing/cleanup acceptance remain
  open. Exact controller/Hermes images are currently
  absent and local heavy RAM guard fails; no native run was started.
- Fleet controls0fe independent review passes10 bounded probes but confirms two
  findings: reaped leader is signalled by historical PGID, and setup601s plus
  relative gate6600s can exceed job120min before cleanup/upload. Counterexample
  reproduction is not a safety PASS. Source wiring independently covers all70
  transitions,367 input hashes, canonical20/split23 guards and19 DBs. A normal
  parent-owned successora3a2c23ace449f873c79bd5e9ff5624c3199070c has sole parent
  11168cae2e647718f574aeeb64a6bb5821b32956, itself a normal child of0fe, in
  `.local/fleet-hosted-backend-custody-20261009/fleet-control`.
  Owner Linux69 PASS/Windows67 PASS with two Linux-only skips cover the
  held-custody/shared-deadline and verify-log same-group fixes. Anscombe independently
  closes these scoped findings with79/79 Linux tests, zero skips; review report is
  `.local/fleet-hosted-a3a2-independent-review-20261010/qa/REPORT.md`, seal SHA256:
  cc96bf8bdd63dde113b87e45ed20787c37fcb0067c64a3dbf7e5d3ce1150e42b.
  Retained owner handoff `handoff-a3a2c23ace44/source-seal.json` SHA256:
  526b413c968ab86609fb077784ea21332752b733a880422c41219fa94acfe860.
  Old0fe evidence/seal is immutable; actual70 is NOT RUN. This path has no
  ComposeHelper/local Docker dependency, unlike native/Forge maintenance paths.
  Published exacta3 normal branch push produced run37992541106/attempt1/job
  114030153823: FAILED at Initialize containers on Docker Hub anonymous pull
  rate limit, before any checkout/private input/helper or backend stage.
  Normal childad893db1fe34ad2a631297d76ee561c4fa1d0d1a changes four image/test
  string lines only. Metadata-only qualification in
  `.local/fleet-hosted-registry-20261010/qualified-receipt.json` proves ECR and
  Docker Hub root-index raw bytes/digests equal for Rust1.88.0-bookworm and PG17.6.
  No layers were downloaded locally. Immutable mirror references retain exact
  versions/resources/source pins. Narrow independent7/7 closure passes with no
  findings; seal SHA256:
  5b9f39b9abbcf2774cabcdb485828bd762a1c24f91dd51cf493497a1e42111c5.
  Parent verifies six review files plus eight registry inputs. Exactad893db is
  pushed normally to the same build-only branch; run37993165800/attempt1 is
  in progress, not an accepted backend result:
  https://github.com/FerrPOINT/fleet-control/actions/runs/37993165800.
- Fleet sequential-activation successord3066ebbbda168f16f59697831ea92de5132527b
  has sole parentf326. Worker reports21 Linux fake-contract cases and15 Windows
  loader/hash/README cases passing. Two new Rust and four PG regressions have not
  executed. Unreleased migration19 is extended; no upgrade of an already-applied
  f32619 is claimed. Independent F6 review closes SOURCE/STATIC with9 checks and
  no new actionable finding. Normal parent merge1801201c3ad5c47248edec45fcde6609b7a32bd3
  preserves both parents and0be22c7 fixture fixes. Combined16 loader/hash cases
  pass in6.114s and Rust formatting passes; compile/PG/native are still pending.
  A subsequent source inventory finds162 ignored and290 ordinary declarations;
  these are not compiled discovery or execution counts. It also finds two stale
  downgrade fixtures: split single-down expects21 rather than22, and two
  down8 probes stop at credentials11 rather than task_chats10. Parent follow-up
  derives the successor count from the selected registry and asserts the exact
  retained ledger, preserving history checks and the blocked-down invariant.
  Only test files change; no migration SQL or product API changes. Independent
  review closes both findings at exactb249bc8 with4/4 pure/static probes. Actual
  Rust/PostgreSQL acceptance remains open. Retarget milestone1 freezes the
  162/290 inventory and19DB/utility9b/strict-schema contract with8 pure checks;
  the executable full gate is not yet retargeted or ready70.
- Forge timing successordaf044da141c0e03847e9b3d9406c4d04f823917 has sole
  parent34c. Parent repeats79/79 pure checks, zero skips,1.383s. Its component
  lock SHA256 is c58c63c4f203873ef4565d4240a85b6b8734ac1c309fa3c874f84576e687274e.
  The worker's Linux89 and product75 results do not establish full12 acceptance.
  Independent review closes initial sourceprep/shared and assertion-to-teardown
  budgets, but reproduces P1 in the actual bridge: the product adapter catches
  the teardown alarm and the next parent.close has no active timer. One real OCI
  cleanup exceeds its120ms allowance at371ms. Normal successor
  89420cdeec55f8e5583857c2e520de3b42462f1b preserves product25be/SDK19a and
  every original budget, adds an absolute sticky phase deadline and guards
  actual bridge/SDK close and late record I/O. Parent repeats84/84 pure cases,
  zero skips,21.228s. Lock SHA256:
  4c8306ea5a5c71569f648b13515e7a3140aa7731b00e4c726d34eadb86e963ad.
  Independent scoped closure passes17 targeted actual deadline probes, including
  one/multi-OCI collection, swallowed/converted alarm, next parent close and
  independent outer-finally/cache cuts. External daemon/journal I/O is substituted;
  no actual Docker/OCI or full12 result follows. Original P1 is closed only for
  this exact successor. No hosted dispatch or publication occurred.
- Native-driver source df6574bfe5d26445b9ce31a8f396d42914c8c653 retargets the
  preserved nine-scenario matrix to exactb249/Base9b, four raw canonical utility
  modules and a separate opt-in/ACK namespace. Owner47 unique pure cases pass;
  independent source review passes20 targeted pure cases with no actionable
  finding in this bounded retarget diff. Review seal is
  bc4ad45a6abc10c4da8f4fe91adca9c69f90d0f26dfbcf8fa61368fcb28d3fc3.
  Compile/images/native remain unqualified.
  It activates before controller restart, so interrupted protocol4 activation
  and sequential recovered-child F6 cuts are explicitly not exercised. A separate
  QA successor owns those cuts without fabricating SQL/custody or weakening the
  original matrix. No actual prepare or native run is authorized by this freeze.
- PM receipt successore05e77e7c41803a597b07ef669d915e7cae44d15 has sole
  parentdc6ab. Parent runs the actual24-case packet successfully with strict
  receipt/provenance validation,457 canonical imports and closed owned Git reader.
  Retained report run-c74568ddf359 records scratch absence and seal
  685f2a8cd4ccb5f18dfa8a203b16330af75b91bd6795c548beb33a7133dac572.
  Eight real-module probes remain distinct from16 synthetic oracle cases.
  Independent review closes the missing/FAIL-receipt defect and revalidates this
  actual receipt, but reproduces a new P2: tree lookup uses a stale remaining
  duration and permits validation after the90-second deadline. Normal successor
  2ee879f269b1c4bc84818f20a37fc20e8a5decf4 closes this finding independently:
  28 runner units pass and the original virtual91s counterexample fails before
  batch verification. Normal merge379b48e27c3306e5d818d971e59c680a4bf75697
  preserves both parents. Its actual retained run-32a21e5cfeba passes24/24 with
  zero errors/failures/skips, validated receipt,457 canonical imports, owned
  reader exit0 and scratch absent. Seal:
  fc1e620d8ddf58580e33814e7dd0f225d12cd3ac6ea18bb5f5731b1b8fbe6442.
  Earlier failed run-e35178825a60 is preserved; its cause is not reinterpreted.
  Producer admission remains BLOCKED/live=false. This is offline evidence only.
- Parent chat successor8e5d75d allows only original-key reconciliation after
  the final question closes new-answer permission. Node22 passes334 tests,
  typecheck/targeted lint/format/build, and3/3 browsers in2.7m. Nine screenshot
  hashes/dimensions/routes pass and mobile/desktop captures were visually read.
  Independent source review reports no actionable product finding. Coverage
  successor5d52c8a explicitly awaits rendered closed-question context; the old
  guard fails this ordered regression. Production component remains identical
  to8e5d75d. Full334 tests and3/3 ordered browser cases pass; nine captures verify
  and mobile/desktop were visually checked. Live PM/durable-reload acceptance is
  not claimed.

The predecessor handoffs below retain their original evidence and limitations:

- Fleet worker sourcef326cdab4045f726449bf07e61a96a1b75e3063a has sole
  parent30f0993. Independent review closes the original five findings at source/
  pure-check level. Normal parent merge44fa0b562702ce1520031d930c36e9a437aec0d6
  integrates it without rewriting either parent. Review also identifies stale
  three-module Rust fixtures and a stale single-down migration count. The parent
  fixes the fixture module inventory/main entry and asserts the exact preceding
  migration ledger rather than weakening the down/up check. Sixteen loader and
  utility tests pass, including execution of the two captured Python fixture
  templates through the actual loader. These are not Rust compilation or a
  PostgreSQL migration roundtrip; both remain required. Sequential activation
  from a recovered effective child remains
  incomplete and is assigned as a separate normal successor, not accepted as
  a permanent limitation of the requested configuration lifecycle.
- Forge controls34c80c1b2921550ed0e97961be89089dc5afc075 has sole product
  parent25be. The final distribution is three fresh sequential jobs A1-5,
  B6 and C7-12, not the earlier two-job proposal. Parent repeats all73 pure
  controls tests successfully in14.988s; independent frozen closure and actual
  full12 remain required. No push or hosted execution occurred.
- PM conformance packet seal425f0d686a7fff71541f979edc04d4e1718446679e7d8fffc577be5b8be64b2f
  is independently repeated by parent:24/24, zero skips,457 canonical imported
  Hermes Git blobs, scratch absent. Eight real-module probes are distinct from
  sixteen synthetic contract-oracle cases. Admission remains BLOCKED. The tracked
  successor dc6ab80091eb4d7ac8d8f5c02d233e49b2f51a65 has sole parent3fa0847;
  worker reports24 passing cases, and parent/independent source review is next.
  Its runner requires an explicit Hermes checkout. The old packet seal does not
  authenticate this new tracked unit; no runtime endpoint or capability is
  enabled by these tests.
- Parent chat fix3fa084753dc3e755c762e0ad8c3554386632d71b preserves the
  original uncertain clarification answer across questions/versions. Node22
  passes331 tests, typecheck/lint/build and the three-browser fixture; nine
  screenshot hashes/dimensions/routes are generated and verified separately.
  [Scope and remaining live acceptance](2026-10-09-clarification-answer-recovery.md).

These are continuations of the existing five workers, not additional competing
implementations. Each has a separate write set; the parent UI is read-only to
reviewers. Heavy jobs remain parent-admitted one at a time. The Forge prepare
has terminated failed and cleaned its exact disposable resources; no local
heavy job is currently admitted. Capacity and exact-image prerequisites fail.
No successor is admitted merely because a worker prepared an invocation.

### Handoff And Dependencies

The renewed assignments were sent to all five existing workers. Fleet and Forge
implementation run independently. Their reviewers use separate evidence paths
and review frozen handoffs, not an evolving working tree. Pascal's PM contract
checks do not modify either implementation checkout or enable an unproved
runtime capability. Task Tracker and project-workflow remain read-only.

Each handoff must identify its exact commit or sealed inputs, changed paths,
executed commands, results, remaining limitations and dependencies. A prepared
invocation or successful fixture is not live acceptance. The parent reviews and
normally merges accepted source without rewriting existing history, runs the
admissible combined gates and publishes task-owned PRs only with accurate status.

The release sequence is implementation, independent closure, normal integration,
combined backend/migration/runtime gates, live chat/PM and Forge acceptance, then
release evidence. Documentation and existing UI evidence are reconciled with the
accepted source; earlier screenshots do not prove a new live flow.

Current external prerequisites remain private Base CI billing restoration,
qualified private maintenance source/delivery and public-safe Forge controls,
qualified exact runtime images/local resource admission, trusted producer-side
pre-model admission/server credential custody and compatible Workflow native
catalog/first-step proof. The parent must not bypass these with a mock,
automatic redispatch, paid-resource changes or a relaxed gate. Work on source
fixes and bounded conformance checks continues independently of those blockers.

Publication revalidation distinguishes repository visibility: private Base's
run37968542132 is billing-denied before steps, while public Forge's later
run37973076579 completes four actual hosted jobs on unrelated source20d54ec.
That observation neither accepts full12 nor proves private Base CI restored.
There is a separate source-disclosure blocker: unpublished Forge34c-to894
history embeds copied Base maintenance files, including base64. Base is private;
Forge is public. Do not push that ancestry, even after deleting the current files.
Ptolemy audits exact private source availability and authenticated pinned-Git
delivery plus a new public-safe controls history. Frozen local packets and prior
deadline closure remain intact; no private source disclosure or changed SDK/budget
is authorized by the independent timing review.

Current publication: [Base PR180](https://github.com/FerrPOINT/services-base/pull/180)
is Draft on815982b, a normal merge of9b53 and accepted maina119. Its ten-path diff
does not change product migrations, SDK, auth or UI. Historical parent-reported
Linux224/224, zero skips (88.807s) remains separate from the new retained44.041s
execution above; its full raw log was not found in the bounded evidence search.
Documentation12, README/hub/manifest and merge-review observations retain their
own evidence scope. Fake Engine execution is not native acceptance.
Actual GitHub run37968542132 does not start any of its11 jobs: authenticated check
annotations report failed account payments/spending limit. No steps execute.
Restore private Base CI access explicitly before its hosted execution; do not retry, buy
resources or relax gates to bypass this blocker.

The reviewed Fleet full46 gate was normally pushed at160bd againstc02ee92.
Actual run37967213470 fails check101 with authenticated E0425 at
runtime_acceptance.rs:542:33 and E0594 at sdlc_foundation.rs:2702:5.
Safe artifact11634537223 has ZIP SHA256
7b6a9894766f451bf617a4b973c4ac43ef7f01f7bf91729b24e19b3e35227b3a.
Source-plan readback and exact scratch/database/platform cleanup succeed.
The previously fixed archive defect is not the new failure. Minimal compiler
fix6e9 changes only the two existing test files and verification ledger. Parent
repeats all nine source checks; independent review finds no issues. Normal
mergee963ab1 integrates the three paths without conflicts and parent formatting/
README/diff pass. Full Rust/PG acceptance remains open. Parent verifies prepared
controlsda9473a, its26-file seal8ccf3f9c and all41 pure checks; unchanged full46/
135 ignored gates still require real execution. No push/dispatch occurs during
the explicit CI availability hold.

Independent review of image-build wrapper seal7c8c8e94 finds two P2 control-flow
defects: stopped foreign containers are missing from cleanup inventory, and
materialized recipe bytes are not compared against sealed inputs. Two pure
reproductions confirm them; no Docker effect occurred. Successor packet9cda3075
seal459f037177b1f649ef0907c78413a9a6bfc64f9dbcae1f4cafa3ea691e804c3d
uses all-container inventory and sealed recipes/full-context prebuild/terminal
parity. Independent review closes both findings with six counterproofs; parent
repeats23 wrapper tests and6 counterproofs and verifies the exact seal. These
are source/pure checks, not image builds or native acceptance. Old7c8 stays
unaccepted. Exact image transport, qualification and resource prerequisites remain.

Recovered-activation draft pre-review finds four independent issues: child SQL
custody does not inherit the original anchor lease, secret-bearing Replacement
has no compatible redacted Debug, a proved original-stop/phase-CAS-loss cut is
unconditionally held, and stopping a published child invalidates its owner anchor.
Parent adds a fifth concrete liveness issue: saving a next config draft changes
desired revision and loses terminal activation provenance although effective
runtime is unchanged. Findings are assigned before freeze/integration; no
compiler/PG/source closure is claimed for this draft. Original missing/unknown
evidence must remain held rather than gaining new authority.

Parent actual Linux review of the new Forge controls reproduces a live orphan
after its command leader exits. The worker changes group cleanup to keep the
leader unreaped with WNOWAIT before signalling its verified session/group.
Parent repeat passes1/1 in0.196s on observed host.py SHA256
5b189daa6451a734bd60b12f6424c497f03e29a98841e72745d275430ffa1cb0.
This is a draft process-cut check, not final full12 acceptance. Bounded hosted
startup/stage/cleanup timing and final frozen controls review remain required.

Safe diagnostics controlsbc4ee52 were reviewed, passed41 pure tests and were
normally pushed once. Actual run37964514708 fails check with authenticated E0609
at acceptance_readback.rs:58:30; both independent readbacks and all cleanup
steps succeed. Parent fixes the proved source defect in separatee2e33b2, then
normally merges it with recoverye391 ata634. Review finds and fixes a pagination
false-positive in the added test; newc02ee92 uses an ordered negative/positive
sentinel and is normally integrated ata006. The old source packets remain
immutable. Full backend/PG and corrected native acceptance still remain open.

Independent source review closes both configuration18 findings at906 and the
PG capability-fixture finding atc1ff. These are not Rust/PG/native passes.
The two-Hermes packet474783 is prepared against oldbde and must be replaced by
a newly sealed packet for the final normal-merged source before execution.
Forge accepted main is nowd128 (PR87 merged); normal merge25be retains its
confirmed-process-exit guard. Neither the old d0 packet nor historical PR checks
certify this new source.

The four recovery16 fixes and provisioning17 have been normally merged at
`be1b040597a9ddd0847aca2c10fdaadb96e4c4a9`, preserving both original parents.
The 18 unique light cases pass; Rust/PG/physical Docker remain unverified.
QA0e coverage review found all 130 ignored identities and strict schema gates
present; that static audit is not a native test pass.

Parent reload recovery is a separate UI source unit: sessionStorage contains
only actor/session-scoped command handles, never steer text. Distinct original-key
readback and explicit receipt settlement are implemented in the UI, but depend
on the lookup API and authentic generation before integration acceptance.
Earlier frozen results below remain historical evidence, not evidence for these
new changes.

The parent reload fixture now passes Chromium, Firefox and WebKit (three cases)
with original-key recovery and no automatic POST after reload. The first run's
Chromium beforeAll build exceeded the unchanged 120-second limit; Firefox and
WebKit passed. That failed packet remains in `frontend/test-results/runtime-controls-css`.
The complete successful rerun is separate in
`frontend/test-results/runtime-controls-css-recheck`. Its nine screenshots are
published through the explicit-input generator and verified; neither run is
live runtime acceptance. Independent review subsequently found two UI defects:
late callbacks after settlement and pre-reservation oversized input. Both are
fixed with eight new regression cases; all328 frontend tests pass. Independent
closure proofs pass both cases. Final browser verification in
`frontend/test-results/runtime-controls-css-final` passes all three engines,
including the UTF-8 guard, and its nine captures replace the fixture manifest.
These results are not inferred from the earlier packet.

UI source `ae027dd` and lookup producer `cfa30f39` are normally merged at
`82b7c8e`. The parent normal-pushed only the reviewed build controls at93a2d9d;
actual run37953053154 succeeds. Worker and parent independently authenticate
artifact11626597579 and its source-plan provenance. Actual schema SHA256
`b074c77295f7ad89912e3667124545ab66f6727184257d1f72030b4417c87f82`
and regenerated TypeScript are integrated; equality, fresh-main compatibility
and typecheck pass. Backend/PG/native acceptance remains required; codegen is
not a release or SDLC success.

Pascal's QA24 packet for be1b040 is prepared with 19 helper cases, 13 Fleet
Python and five original Base tests passing. Configuration18 is separately
frozen at bde64862, with 23 light Python cases passing. Neither has native
Rust/PG/physical Docker acceptance. Their normal integration and missing
live-driver preparation are independent successor tasks.

The actual Forge full12 packet dcbba1cb4e3f terminates exit1 at the unchanged
300-second smoke deadline. Python75 and row-smoke pass; smoke and the nine later
stages do not. Latest diagnostic inventory records350 starts and347 synchronous
completions (298.93495 seconds), with two held overlaps; partial markers are not
acceptance. Parent inspected the
terminal probe and same-daemon cleanup: owned containers/networks/volumes empty,
baseline20/source265/protected caches preserved. No replay occurred. The next
task diagnoses and corrects demonstrated transport overhead without shrinking
the matrix, replacing real PostgreSQL or relaxing limits.

The parent integration is frozen at
`7f9ae892f9db482bbf44fda5d3082a074b6421b1`: a normal merge of the checked
foundation/UI source and backend `37ec604a`. Backend, CI and migrations match
that backend commit; frontend/API/Base inputs match `3f8ed8f`. Formatting and
staged-diff checks pass. The earlier 270 frontend tests apply to the identical
frontend tree, not to native execution of the newly integrated backend.
The acceptance worker receives this exact immutable source, including its 130
ignored integration cases. An inventory or prepared command is not a test pass.

All five assignments have been sent to the workers. Only the parent coordinates
heavy-job admission, integration and publication; workers do not independently
deploy or push release branches.

## Latest Frozen Results

Parent source `0e854c0e97beb9b3ac64eb1415247ce81604a460` retains the integrated
backend and adds authentic generated OpenAPI, typed GET-only command readback,
explicit security migration checks and regenerated fixture evidence. All286
frontend tests, typecheck/build/targeted lint, schema equality, contract snapshot,
compatibility8 tests and all three browser fixture cases pass. Nine screenshots
are generated/verified; native execution and automatic UI uncertainty settlement
remain unaccepted. Leibniz retargets the strict backend preparation to this SHA.

Original ancestry recovery is complete in a standalone object DB, with fsck
passing and no rewritten history. The parent normal-pushed build-only commit
`db2bb4bd`; [hosted run37945042300](https://github.com/FerrPOINT/fleet-control/actions/runs/37945042300)
successfully generated the exact fc6 source schema. Both worker and parent
authenticated artifact11622004476. No release PR or runtime deployment follows
from this generator run. The worker now reviews the consumer independently.

Automatic initial Docker preparation is frozen separately at
`337aac87092be42027f669fc0730858f3724824f`, one own migration17. Fourteen Python
checks and light source gates pass; Rust/PG/physical Docker remain pending.
Pascal's independent16 review found canonical Git/CRLF utility hash mismatch,
Unicode mapping serialization mismatch, slow-loop lease expiry and foreign-owner
heartbeat settlement risks. Fixes are a separate source unit, not silently
folded into the frozen provisioning commit. Config activation and replacement
remain Feynman's next implementation scope, not completed by preparation17.

Forge's reviewed diagnostic full12 helper passes35 pure integration tests
independently. Prepare-only orchestration is authorized with the original
resource/ownership guards; actual run requires a fresh seal review and separate
ACK. No inherited partial stage result is promoted to acceptance.

## Work Ownership

| Owner    | Independent task                                          | Deliverable and boundary                                                                                                                                                                                                                    |
| -------- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ptolemy  | Forge smoke diagnostics                                   | Private helper with phase/call timings, bounded child waits, cleanup and pure regression tests; retain the 300-second aggregate deadline and all assertions. No product change without a demonstrated cause.                                |
| Pascal   | Steer transcript durability                               | New backend follow-up from controls13: persist exactly one redacted control-message with original author/session/run identity when acknowledged; uncertain/rejected delivery must not become successful history. No UI or approval14 edits. |
| Leibniz  | Approval integration and backend verification preparation | Normal-merge frozen approval14 into recovery/controls in a new checkout; preserve every test/fence and prepare an exact combined inventory/helper. Do not alter generated contracts manually or treat preparation as execution.             |
| Feynman  | Docker mapped-volume/controller recovery                  | Continue the isolated lifecycle successor from `333c06d9`; retain Base custody, original generation/origin and durable receipts. No socket access for agents or unrelated SDK/pin changes.                                                  |
| Anscombe | Hosted Rust OpenAPI generation preparation                | Minimal build-only workflow, pinned source/Base/Rust, generated artifact with provenance and hashes. No push/dispatch until reviewed; no hand-edited schema or weakened existing CI.                                                        |
| Parent   | Integration, UI, documentation and publication            | Review source changes, generate client types from authentic Rust output, implement receipt states/readback/reload recovery, verify UI, run gates and publish scoped PRs.                                                                    |

Each worker owns a separate checkout/write set. Frozen commits and previous
evidence packets remain immutable. Task Tracker and project-workflow remain
read-only. Generated API/client files belong to the parent integration path.

## Successors And Verified Progress

The table above records the initial split. Completed source work now advances
through these non-overlapping successor assignments:

- Ptolemy: integrate the sealed diagnostic component into a new prepare-only
  full Forge orchestrator, keeping all 12 stages and the 300-second deadline.
  Parent independently passed all 32 diagnostic pure tests; no native rerun yet.
- Pascal: frozen steer mirror `71b17da7` adds seven regression cases and no
  migration. Review frozen mapped/controller16 correctness independently while
  Leibniz integrates that mirror. Linux/PG/native mirror acceptance is pending.
- Leibniz: frozen approval integration `a26d8b35` preserves normal parents
  recovery/controls and approval14; prepare42 passes 25 pure tests and inventories
  123 ignored cases. Next normal merge adds the separate steer mirror, not UI.
- Feynman: frozen mapped/controller16 `fc2e27b7` adds one migration and remains
  source-only. Next isolated unit implements automatic preparation with a durable
  pre-create intent and original Base reconciliation, not config activation.
- Anscombe: reviewed hosted-codegen controls are committed at `db2bb4bd`.
  Both pushes were rejected by missing shallow-ancestry object `09b35f1e`;
  no workflow run exists. Recover complete ancestry in a separate object DB,
  without modifying existing shared histories or inventing an ancestor.
- Parent: foundation [PR47](https://github.com/FerrPOINT/fleet-control/pull/47)
  is review-ready at `28c9a5ee`, main/CLEAN, all five exact-head CI jobs green
  after readiness reconciliation. CI passes 262 unit tests, 48 browser fixtures
  and 135 screenshot entries; 27 live tests skip. Local readiness browser9 pass,
  captures were inspected and the owned preview stopped.

The new foundation/UI integration passes all 270 unit tests across 33 files,
typecheck, production build, Rust formatting and 109 Markdown link checks. The
existing bundle-size warning is retained. Backend, API, Base pin and runtime
control UI code are unchanged by the foundation merge. Nine existing control
fixture images verify again. This does not certify controls/recovery on Linux,
physical Docker behavior, model admission or the live PM vertical.

## Existing Results

- Approval14 source is frozen at `68b59625e1b8a57139131a7672d90b6a3f465271`:
  one new migration, 21 added cases prepared, light checks pass. Rust/PG/native
  execution remains pending.
- Recovery/controls QA preparation contains 37 stages and explicitly covers all
  106 ignored cases; 20 pure tests pass. Its original source is `ed798638` and
  it must be retargeted/reviewed for later source and generated contracts.
- Native codegen preparation passes 17 pure tests but cannot execute under
  current host memory/commit reserves. The resource guard is not waived.
- Forge's latest full packet failed at the smoke aggregate timeout: only 2 of
  12 stages accepted. Owned resources were cleaned and independently checked.
  The delay's exact cause is not proven; diagnostic preparation is next.
- Integrated UI source `f32ecb7` retains 267 passing unit tests and three-browser
  fixture evidence with nine screenshots. Native controls/recovery are not
  accepted by those fixture results.
- Base Auth [PR126](https://github.com/FerrPOINT/services-base/pull/126) is
  review-ready at `dc43d0e25d60afa073c201e74d1a2cfe9aab8939`, with all ten
  exact-head CI checks successful at review. It is not merged or installed by
  this work split and does not establish end-to-end PM admission.

## Integration Order

1. Review backend approval integration and steer mirroring as separate changes;
   preserve original recovery/control identities and normal Git history.
2. Generate OpenAPI from the final Rust source, verify provenance, then generate
   frontend types. Retarget strict schema-parity and backend test inventories.
3. Complete receipt-state/readback/reload UI and regression/browser evidence.
4. Execute reviewed backend and Forge gates using explicit immutable source
   heads; retain failures and unknown outcomes instead of relabelling success.
5. Publish scoped PRs with exact-head CI and dependencies. Keep migration units
   separately reviewable; never publish the broad historical runtime tail as
   one release or claim fixture evidence as live SDLC acceptance.

## Resource And Acceptance Rules

Source work runs in parallel. Owned heavy Docker/Rust jobs run one at a time
after review and explicit orchestration ACK. Use genuine temporary Compose
projects, Base cleanup journal v2, exact disposable resource lists and immutable
Git exports. Do not restart Docker/WSL, prune shared resources, lower guards or
change accepted runtime images to make a test pass.

The remaining producer admission/readback dependency in project-workflow is a
separate integration blocker; no worker may introduce a model-admission bypass.
Completion requires actual Linux/PG, runtime and contract evidence, followed by
checked PR publication. Source commits, prepared helpers and healthy processes
alone are insufficient.
