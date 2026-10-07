# Chat Clarification Verification

## Browser Authentication Release: 7 October 2026

Independent release source9b42e777eac64383b8bd397fe347b02beb1dd3f7 is based on
main1ff9066206f28e8a49b2c290511c4f52adf7c105, with no backend, migration,
OpenAPI, package/lock or Base pin changes. [Fleet56](https://github.com/FerrPOINT/fleet-control/pull/56)
targets main, is non-Draft, mergeable/clean and has five successful exact-head
CI jobs in [run37575299434](https://github.com/FerrPOINT/fleet-control/actions/runs/37575299434).
Ready-state readback confirms the same head/checks. No unresolved review thread
was present at release readback; merge/deployment remain separate gates.

Fresh Node22.20.0/pnpm10.28.1 frozen installation uses Base
f04af5fd5906ad6ac7e24c7f18e919a5d8a04965. All73 installed SDK source/script/
package inputs match Git/LF bytes. All150 unit tests in23 files and18 fixture
browser cases pass, with no retry/skip/unexpected/flaky browser result.
Typecheck, lint, formatting, build, API generation/drift/compatibility, Markdown,
README and135 baseline manifest entries pass. The owned preview is stopped.

Main release artifacts are in the private fleet-browser-auth-release-843a9d7
QA directory. Browser APIs and signed SSO are fixtures, not live identity
or Hermes acceptance. Main's existing chunk-size warning remains unchanged.
The integrated runtime checkout retains its own cbb4e99 pin; results from main
must not be attributed to those different bytes or SDK.

Before implementing the boundary,11 focused tests reproduced cache/late-response/
SSO isolation failures. The boundary adds fresh-generation cache/form ownership,
original request token capture, post-body authority checks and late401 isolation.
It preserves memory-only credentials and opaque original-control uncertainty.
Clarification/backend/workflow readiness, Base SSE401/403 callbacks and PM
original-command discovery after reload remain separate requirements.

Integrated candidate validation retains the unchanged cbb4e99 SDK pin. All147
frontend Git/LF inputs match the owned checkout; all73 installed SDK inputs
match the exact clean Base revision. After final source formatting,439 units
in35 files pass. All84 selected browser cases pass in187.1s, with no retry,
skip, unexpected/flaky case or report-level error. Typecheck/lint/format/build,
API drift/eight compatibility cases, seven local-snapshot DTOs/two verifier
cases, nine manifest verifier cases/135 baseline images, pinned UI validation
and132 Markdown link files pass. Published Tracker compatibility is not implied.

Private final reports in fleet-chats-final-3217793 QA: unit SHA256
4c6645f6129016947fe3be8a80d50e4ac16e854b8e0ba88766b481b633f50621;
browser SHA256 e3987239b7248fb0a7e9a048320dad7403fa5f515f64dbe5bdf8479f1b5455fa.
Fresh375 clarification and1920 dialogue fixture screenshots were opened and
inspected. Preview fallback connection-refused logs remain fixture noise,
not zero-network-error/live acceptance. The owned4173 listener is absent.
Historical browser files were retained outside the frontend formatting tree,
without altering report/image bytes. No installed service was changed.

The local read-only Docker grouping audit reports no violations on checked
desktop-linux/sdlc2-runner, but complete=false because sdlc1-runner is
unavailable. This operational gap is retained rather than reported as passing.

## Chats Reload And Identity Integration: 7 October 2026

Inputs: runtime46df7aa43943b3e2ca7e66b109a5038deb512763 and
Chats94e889d2632517936597af5835e45cb607d78c4b, combined by normal merge,
plus four shell sign-out regression fixes. Backend tree remains
4ac2b4b83113fa481ff263c57397fe3da2594d15: the preceding568-case Linux/PostgreSQL
gate applies to those exact unchanged bytes, not a new invocation after this
frontend integration. No migration, installed runtime, SDK pin or opt-in changes.

All144 tested frontend inputs match the owned checkout under Git/LF line-ending
normalization. A raw-byte comparison stopped on CRLF/LF in chat-preview.html;
the subsequent normalized comparison found no content drift.
Fresh isolated QA uses verified Node22.20.0 and pnpm10.28.1 with frozen lockfile
installation. All73 installed Base SDK source/script/package inputs match pinned
cbb4e99230420dc2659431b1c9fb5090e5c940f0. The original checkout's cached package
instead came from c083783; its preliminary409-case runs are not release evidence.
The failed four-case shell regression run is retained separately; the corrected
focused suite passes21 cases and the full suite passes413 in33 files.

Typecheck, strict lint/semantic classes, formatting, production build, generated
OpenAPI client equality, eight compatibility cases, seven local-snapshot DTO
contracts/two verifier cases, pinned Base UI validation,132 Markdown link files,
135 baseline screenshots/nine verifier cases pass. The contract command explicitly
does not compare the published Tracker producer; its compatibility gap stays open.
Vite retains the existing greater-than500KiB main-chunk warning.

The initial browser run retained83 passes and one WebKit timeout: periodic
command polling disabled manual refresh before the test clicked it. The test now
waits for the real failed read, preserving disabled controls, draft and zero-POST
assertions. A fresh complete selected run passes84 Chromium/Firefox/WebKit cases
without retries, skips, unexpected cases or page-level report errors, in300.6s.
The fixture preview still logs ECONNREFUSED to its absent fallback API; this is
not zero-network-error or live backend evidence. The owned preview listener is
independently absent after completion. Fresh mobile chat and desktop stale-question
fixture screenshots were opened and inspected, separately from imported captures.

Private QA JSON report hashes: unit
5f5c3b844dd0b26b742ff37abac5a6cb42c633d4794af96b58e31fe240cb6073;
browser b46a932dd091b7c3dafbd1d5466bd46912aeeee7614dfbb4265057fca6205734.
Artifacts remain in the task-owned fleet-chats-final-3217793 QA directory,
including the preceding failed error context; no installed service was changed.

Two imported dated manifests independently match21 historical Git/LF source
hashes and24 PNG hashes/dimensions. Mobile375 and desktop1920 clarification images
were opened and inspected. Their false live-acceptance boundary is unchanged:
these are historical fixtures, not recaptured current runtime or real PM evidence.

Pending release acceptance remains actual PM tool publication/answer delivery,
checkpoint/rebind, published producer contracts, full controller restart,
same-SPA query/draft/late-response privacy and Forge deployment. Source publication
is not merge/deployment or a declaration of full SDLC readiness.
Fresh remote readback also reports main conflicts for
[Fleet47](https://github.com/FerrPOINT/fleet-control/pull/47) at5f20540 and
[Base150](https://github.com/FerrPOINT/services-base/pull/150) at7d7323a.
Those separate predecessor PR heads were not changed or marked ready here.

## Original Controller Delivery And Historical Outcome: 7 October 2026

The follow-up owns one new migration000021, private delivery/outcome repository
ports, closed native protocol3 receipt validation and a trusted supervisor entry.
Published000020 bytes, original launch/controller identity, SDK/source pins,
frontend, public API and installed runtime/flags remain unchanged.

Command: `python -B .local/fleet-container-control-gate-20261006.py full`.
Exact disposable project: `sdlc-qa-fleet-container-control-859880763d3d`.
Private artifacts: `.local/fleet-container-control-checks-cc0e9cdad08a/`.
Linux Rust1.88 fmt/locked offline all-target check/strict Clippy and full workspace
gate pass:568 passed,0 failed,30 explicit opt-in ignores across39 result groups.
The report records `actual_docker_hermes_acceptance=false` and
`installed_runtime_changed=false`. All303 captured inputs independently match
current bytes. Generated OpenAPI matches the checked-in SHA256. Driver cleanup
and independent exact-project container/network queries confirm empty inventory.

Three new PostgreSQL delivery cases prove16-way command replay, exactly one claim,
payload conflict, strict closed receipt drift, once-only outcome/hash-only audit,
real31-second expiry and historical ACK without version/deadline renewal. The
expired ACK cannot authorize heartbeat/redispatch; same-physical successor stays
held. Two supervisor cases use controlled Python Base responses: lost native ACK
recovers by original readback with one native call; claim-before-call crash with
no ACK stays held across repeated entry attempts, with no native call/model permit.
They do not prove actual Fleet OS-crash or ongoing Hermes/container acceptance.
The migration case proves additive upgrade, empty restore and nonempty history
retention without disabling guards. Java lifecycle remains unchanged.

| Artifact | SHA256 |
| --- | --- |
| `source-manifest.json` | `55b82bf6b0c32402cdacbbc11ab0038e6e2721aecefbeb64216ff85425d5fc01` |
| `gate.log` | `ebbbf0d2e61f222856a22df589492c27c8578c69ae301de54999e79fabecd638` |
| `report.json` | `18e2350695a14615b39084121489abfaa74010cfe0833104f3c432d5b1ed5956` |
| generated/checked-in OpenAPI | `76a27c806961bc142543445e34f10525485cbcf2f0b66ac2fe5d093fc808a10f` |

Earlier focused packets `2dd3b980ba8d` (formatting) and `d72055b5a48c` (missing
type qualification) failed and were cleaned; they are not passing evidence.
The earlier `c33dc0585a7b` packet failed before container creation while Docker
was unavailable; after Engine recovery, independent exact-project queries are
empty. No installed service recreation, volume replacement or global prune.

Base native protocol3 is separately published at
[3facb28](https://github.com/FerrPOINT/services-base/commit/3facb289d449c6a9a2a3863e31a661a661235299).
Its native restart packet is separate from this Rust/PG component proof, and its
source is not automatically selected by unchanged Fleet pins. Automatic startup,
dual DB/native heartbeat and effect admission, interrupted activation, production
logs, task/PM/Forge acceptance and ordered release-head CI remain open. This
candidate is not installed, merged or a complete SDLC acceptance claim.

## Fenced Controller Recovery Storage: 7 October 2026

The candidate adds only migration000020, private request/record/repository
operations and original-owner effects fences. Original launch identity and
controller metadata remain immutable. No HTTP entry point, native worker,
installed flag, SDK/source pin, frontend or accepted runtime image changes.

Command: `python -B .local/fleet-container-control-gate-20261006.py full`.
Exact disposable project: `sdlc-qa-fleet-container-control-8aa68a2fea71`.
Artifact directory: `.local/fleet-container-control-checks-6a3d59daddd0/`.
Rust1.88 Linux fmt/locked offline all-target check/strict Clippy and the full
workspace test gate pass:562 passed,0 failed,30 explicit opt-in ignores across
38 result groups. The report explicitly records
`actual_docker_hermes_acceptance=false` and `installed_runtime_changed=false`.

Six new PostgreSQL cases use controlled Base preparation, not Docker takeover.
They cover16 concurrent identical reservations and conflicting payloads, changed
original identity, version/owner heartbeat and once-only ACK, independent DB
readback, original queue/endpoint/observation fences, real31-second lease expiry,
unknown acceptance hold and predecessor checks, and direct SQL history guards.
The new migration case owns a disposable schema: original agents/launches remain
unchanged on upgrade, empty down/up works, nonempty downgrade refuses without
losing history. No production trigger or clock bypass is used.

All298 captured inputs retain their original hashes. Generated OpenAPI is
byte-identical to the checked-in file. The driver reports exact own cleanup;
independent Docker container/network queries for this project return empty.
No global prune, permanent volume replacement or installed service restart.

| Artifact | SHA256 |
| --- | --- |
| `source-manifest.json` | `58c936cf9212a857290e38ac758bf1e7e813afedf7276c5ed15fbf3aa7d52472` |
| `gate.log` | `751375e069e4ae3fac2a1f0ced460ba168985f20ed8f3bf012680e7bd2bc899f` |
| `report.json` | `481771660672c0b0040795826d419dc3e4fb0338b0da983c1f2731920b74b2cd` |
| generated OpenAPI | `76a27c806961bc142543445e34f10525485cbcf2f0b66ac2fe5d093fc808a10f` |

Earlier focused project `sdlc-qa-fleet-container-control-b929d33c93b9`, artifacts
`.local/fleet-container-control-checks-d7920a373f8c/`, failed compilation because
a test attempted to clone `DatabaseConnection`. It was cleaned and is not a
passing result; the corrected test opens an independent real connection.

The storage ACK is not native custody. Actual Base handover/original-key receipt,
DB/native dual-fence worker, unknown-outcome reconciliation, interrupted
activation, production collection, admission and PM/Forge acceptance remain open.
Older frontend352/72 and native restart packets below retain their original
scope and are not relabelled as new evidence for this storage change.

## Integrated Chats And Runtime Candidate: 7 October 2026

Normal merge `49c11f54f7322bcb671b41b2cdf4b50264713eff` includes runtime
`9a11bde9d4ae7df8a2621746597a235b1edfc499` and original-key UI consumer
`16b751679db3a32e921603acc64e9e9aa4f7c350`. Exact backend tree remains
`d5e37d93cd877b5c0cbd2ef8dbe80222b5c26d2b`, identical to the runtime head.
Exact frontend tree is `4dc8741a3d3400ddbca2a630eeeea8823f584fe0`, identical
to the published consumer. The full555-case Linux gate below therefore covers
the unchanged backend bytes; it was not rerun or relabelled after this UI merge.

Fresh integrated `pnpm test` passes352 cases in32 files. Build/typecheck, lint,
formatting, OpenAPI/client equality and8 compatibility cases pass. Vite retains
its existing large-chunk advisory and Vitest retains the existing localstorage
warning. The135-screen baseline and9 validator cases pass. Existing controller
and runtime-control evidence verifiers pass separately.

Fresh integrated browser command
`pnpm exec playwright test fleet-control.spec.ts chats-directory.spec.ts --workers=3`
passes all72 selected cases in Chromium/Firefox/WebKit in2.2 minutes, with no
retries. Playwright owns the temporary4173 preview and stops it after execution.
The fixture proxy also logged unhandled/cancelled requests to its deliberately
absent3456 backend; this is not a zero-network-error or live-system gate. No
pageerror filtering or Base SDK update was used. README and128 Markdown files
pass their validators after the documentation update.

Twenty-four imported PNGs pass SHA256 and dimension checks, with15 source hashes
checked against their exact historical Git blobs rather than the newest file:
[original-key consumer](assets/screens/chats-control-key-lookup-20261007/validation.json),
[session recovery](assets/screens/chats-session-recovery-20261007/validation.json),
[catalog freshness](assets/screens/workflow-catalog-freshness-20261007/validation.json).
Their manifests retain `fixtureOnly=true`, `liveAcceptance=false`. Fresh mobile
stop-held and desktop steer-ACK captures were opened and inspected without
overlap; they show the retained draft/unknown stop and cleared acknowledged
steer error. They are controlled fixture data, not live PM or runtime acceptance.

Imported original-key identity is still component-memory state. Browser reload
and logout/identity/service isolation are being verified independently, not
asserted by this merge. The pinned Base direct SSE denial/WebKit finding remains
open even after successful fixture repeats. Actual public-route native reply
loss, ownership transfer/interrupted activation, production logs, producer
admission/first-step, PM delivery/checkpoint/rebind and complete SDLC remain open.
No migration, SDK/source pin, installed image or opt-in changes accompany this
source integration. Base PR150 remains an open conflicting main dependency at
7d7323a59d744d50f9b101a3568adb3d59ee9683; Fleet PR55 is a separate clean main
packet atfdbd7dd2c6cf1d9a66651fba5c0bf92a66047529. Neither is changed here.

## Original Controller Restart Observation: 7 October 2026

Base published source
[9171fe6b1b6b05b9504d33fb881f274c0b5541e0](https://github.com/FerrPOINT/services-base/commit/9171fe6b1b6b05b9504d33fb881f274c0b5541e0)
adds read-only protocol2 restart observation. Actual own native Compose project
`sdlc-qa-mount-mapping-0b0ee9958950` passes38 Linux control cases, restarts the
same physical controller and proves unchanged original UID999 synthetic agent
namespace plus private journal bytes. Observer replay succeeds; ordinary
observe/start/endpoint/stop remain denied. All11 frozen inputs match, own
containers/networks/two disposable volumes/three aliases are removed and
permanent runtime is unchanged. This is not a real Fleet process or Hermes model.

- Base native report SHA256:
  `e8fcac83e0add31841ab442aa8c6fad6ec8707538859d11af1649c7c0a37104d`.
- Native manifest SHA256:
  `6abcefae36b3bf42db0305b3d8bb6d1c78ae8c1a7e96d7004c80f08b9b8ea944`.
- Native restart probe log SHA256:
  `2eac8bf9b739f32208ceccd1ade03d12e78056f2bb063c71ac141be8d669e84f`.

Private native artifacts: Base repo
`.local/mapping-live/sdlc-qa-mount-mapping-0b0ee9958950-cahz202p/`.
The earlier `0b08215de800` and `003a2e52d704` attempts remain failed, cleaned
QA-harness evidence; they did not reach the controller restart assertion.
No production guard was weakened to repair initialization/ownership checks.

The separate full Base Linux/Rust1.88/PostgreSQL gate
`sdlc-qa-base-ledger-136ff8acea1c` passes fmt/locked strict all-target Clippy,
63 Rust cases with14 explicit opt-in ignores and145 Linux runtime cases without
skips. All164 captured inputs match and own resources are cleaned. Frontend
13 Node/83 UI cases, typecheck/lint and README/hub checks pass.
Manifest SHA256
`c0a300d695216749b05b98c35c069583bb7df8ca931a2284158d7dae6623a3ff`,
gate log SHA256
`75f7fad6fb3677519e0d516e6a9bb32804f55933542f0eada339ad0a8c2f951c`.
Private artifacts: workspace `.local/base-ledger-checks-1b4a9bb3cc15/`.

Fleet adds three closed client/subprocess cases and one PostgreSQL supervisor
case. A separate restarted supervisor reads original evidence, reports degraded
health and does not mutate agent/launch/files or obtain start/stop/gateway
generation authority; original ACK/PID/source drift remain denied. This is not
the actual Rust-to-Base/Docker controller takeover gate. No migration, public
API, SDK/source pin, UI or installed runtime change is introduced. Durable owner
transfer, interrupted activation, recreated controller/lost journal/source upgrade,
production collection, predispatch/PM/Forge and full live SDLC remain open.

Fresh Fleet project `sdlc-qa-fleet-container-control-bc612f06c2ac` passes
Linux/Rust1.88 fmt, locked offline all-target check/strict Clippy and555 workspace
cases in37 result groups, with zero failures and30 explicit opt-in ignores.
All295 captured backend/SDK inputs remain unchanged; owned containers/networks
are removed and independently queried empty. Baseline is Fleetf1891aa plus the
three captured runtime/client/test changes, unchanged SDKcbb4e99. Rust-generated
OpenAPI is byte-identical to the committed specification; this private witness
adds no public path or DTO.

- Fleet source manifest SHA256:
  `f9577c39e1054a222515a7236abc0239b9d932931e6beae01a34acc1afaaa9f3`.
- Full gate log SHA256:
  `74019412d2e9a2f161e242ebcea307bcc91bcaae55c85b0ac5c15f7afc026b2a`.
- Report SHA256:
  `635805327cc3350d6db1b503294f86947fe26e5f2078bf6d926991b83139915e`.
- Exported OpenAPI SHA256:
  `76a27c806961bc142543445e34f10525485cbcf2f0b66ac2fe5d093fc808a10f`.

Private artifacts: workspace `.local/fleet-container-control-checks-ae060a953134/`.
Three earlier projects stay failed/interrupted evidence, with exact own cleanup:
`4e5be8163f8e` and `4ee56898c0c1` stopped at test compilation (private helper and
unqualified path type); `e1e69c41a69c` was deliberately stopped through its
verified exact Compose manifest before correcting a mock-only missing-file
assertion. The successful test checks the actually persisted preparation/mapping
documents and call ledger; real Compose/SQLite byte preservation remains Base's
separate native proof. No production source guard was weakened.

Fresh existing frontend checks pass292 unit cases in31 files, typecheck, lint,
formatting, README and127 Markdown links. The135-screen manifest and9 verifier
cases pass. No new browser capture or live runtime/PM UI acceptance is claimed
for this backend-only packet.

## Original-Key Control Lookup: 7 October 2026

The additive human GET recovers the original actor/key receipt when an initial
reply loses its command UUID. Closed operation/digest input, fresh human/session/
project checks and the existing actor/key unique index enforce exact scope. It
returns no key/input/hash/native credential context and never dispatches, reserves,
mutates or reconciles a command. Historical terminal receipts remain readable.

Two domain cases verify canonical UTF8 hash vectors and query validation. Three
new PostgreSQL/controlled-HTTP cases prove concurrent/fresh-repository lookup,
lost reply ID, scope/payload conflicts, unrelated user/operator/admin key denial,
revocation and terminal uncertainty without another native POST. Unknown ACK
remains unaccepted even after an independently committed terminal mirror.
These are not an actual Fleet OS restart or native Hermes reply-loss scenario.

Fresh project `sdlc-qa-fleet-container-control-af2745dcc72d` passes Rust1.88 fmt,
locked offline all-target check/strict Clippy and551 workspace cases in37 result
groups: zero failures,30 explicit opt-in ignores. All295 captured source inputs
match after execution. Rust exports the updated OpenAPI after the tests; its
only added public path is the original-key GET. Source baseline is Fleet3f7d4e5
plus the captured lookup changes, with unchanged SDKcbb4e99. Owned Compose
containers/networks are cleaned and independently queried empty.

- Source manifest SHA256:
  `b18f50fb07362f3b83fb7af647f048c43dc911ad79e754a200b59150cf758c0e`.
- Full gate log SHA256:
  `7bf1055c5d8b20f4f5d789feaa7ba863210fa7c605dfde4683c99287776bc3cd`.
- Report SHA256:
  `f8dbdb9daf4a7050d6dadb1aa91f2f6d8c6c658786a28bc8ce39d73af705824c`.
- Rust-generated OpenAPI SHA256:
  `76a27c806961bc142543445e34f10525485cbcf2f0b66ac2fe5d093fc808a10f`.

Private artifacts: workspace `.local/fleet-container-control-checks-fb914e36cd7c/`.
After regenerating the ignored local TypeScript client, fresh OpenAPI/client
equality, eight compatibility cases, typecheck/build, lint, frontend formatting
and292 unit cases pass. README and127 Markdown links pass; the existing135-screen
manifest passes its nine validator cases. No screenshots or browser/live PM
scenarios were newly captured/run for this backend-only route. Vite retains its
existing large-chunk advisory. An additional root-doc/OpenAPI Prettier probe is
not green: the same six Markdown files and exporter JSON were already unformatted
at3f7d4e5. This packet does not rewrite unrelated historical formatting or alter
the native Rust export bytes; it is not a claim of a complete repository gate.

The earlier owned focused run `sdlc-qa-fleet-container-control-bb9c422b6bde`
remains failed evidence (44 integration passes,1 failure,2 ignores): the test
incorrectly expected terminal observation from a run flag without a committed
mirror. The corrected test uses the real independent terminal commit. No backend
terminal guard was weakened; both invocations verified exact owned cleanup.

No migration, SDK pin, production UI or installed runtime change. Consumer
original-key wiring and actual native/OS-restart acceptance of this route are
still required. Neither this gate nor earlier native packets establish complete
PM delivery/resume, assignment admission, production log collection or SDLC.

## Native Provider Rotation And Original Input Custody: 7 October 2026

The expanded ignored supervisor scenario runs two actual pinned Hermes gateways,
the real Rust supervisor and original Base/Engine with UID999 mapped-volume
custody. The local model is an explicitly named custom provider using `key_env`,
not a host-gating bypass for a vendor key. Six real `/v1/runs` finish with one
final mirror each. The model verifies received authorization hashes: original
key for both initial runs and the held run, rotated key after Developer activation
and after readiness rollback, original key after the peer's fresh restart.
The held run drains before any dotenv change. No raw authorization is reported.

All six retained creation intents match PostgreSQL generation/operation/intent
hash, exact original dotenv digest and expected per-agent API/provider inputs.
Private files remain UID999/mode0600/single-link; the initial document remains
byte-identical after rotation. Real readiness timeout retires the candidate and
restores a distinct previous-revision generation; its final native run proves
the restored SOUL and provider key. Base's separate private log probe verifies
six nonempty original exited generations, persisting counts only.

Project `sdlc-qa-fleet-container-live-a8ca318a4ef2` passes fresh Rust1.88 formatting,
locked offline workspace/all-target strict Clippy, test-profile compilation and
the actual ignored test:1 passed,0 failed/ignored in374.26 seconds. The306 captured
inputs remain unchanged. Source baseline is Fleetd50c694 plus the captured test/
driver changes; Base control7bf2df1, SDKcbb4e99 and Hermesbbaf7af are unchanged.
This is not a fresh run of the ordinary546-case workspace suite.

- Source manifest SHA256:
  `3b46d9e0fbe41e96890900ef6582a47a909a90849e9e6c4fd643896ed205fcd2`.
- Build log SHA256:
  `5b6419e18626506bd5ccd095ad39a28031c676358e20276c5b1ac1ade68503b0`.
- Live log SHA256:
  `d765a902575780433b5a4651c1f3705337088ba36b53638fbf749bf5a27bdf30`.
- Live report SHA256:
  `f625af9857d5c8ae9c3084774abc836d46a74774f901369fca87a70c0b081583`.
- Overall report SHA256:
  `86ca86e98f9cf290bfdee127e1d08d918d033b289bdcc4b546fced4a76b2991d`.

Private artifacts: workspace
`.local/fleet-dotenv-native-evidence-20261007/sdlc-qa-fleet-container-live-a8ca318a4ef2-g76dtqzi/`.
The exact owned Compose cleanup removes only this invocation's containers,
networks, three disposable volumes and image aliases. Independent inventories
are empty; permanent runtime lifecycle is unchanged. Sixteen host driver cases,
six source-loader cases, README and127 Markdown checks pass.

The earlier owned `460585ed396a` invocation failed during compilation because
this SeaORM build does not expose `DatabaseConnection::clone`; it never reached
Hermes. It remains failed evidence with verified cleanup/source/runtime preservation.
The corrected test opens its own read connection instead of enabling a dependency
feature. No test assertion, readiness deadline or Hermes host restriction was
weakened to obtain the passing result.

No production API, migration, SDK, UI or installed runtime change. The driver
now rejects missing/mistyped/overclaimed flags and wrong baseline/rollback counts.
This closes native static-provider rotation and original input custody, not
every effective/external/managed secret source, later reload, production Rust
log ingestion/redaction, controller recovery or task/PM/Forge/live SDLC.
All reports retain `sdlc_acceptance=false`; no full merge-ready claim follows.

## Original Container Environment Input: 7 October 2026

New automatic creation intents preserve exact guarded dotenv bytes/hash before
Docker create, including missing/empty identity, under the existing immutable
preparation DB hash. Revision-bound inputs match the rendered file; retries and
prepared starts reject drift. Original documents are retained and historical
intents are not backfilled. The complete serialized private-document limit is
checked before the DB claim, not after a potentially unrecoverable reservation.

Seven new regressions pass: exact raw input without interpreting interpolation;
missing/empty/invalid/oversized/nonfile handling; symlink/hardlink/outside-root
denial; pending rotation/snapshot-removal hold and original recovery; prepared
start drift/deletion with retained private bytes; serialized overflow before
claim/create; and activated-revision file equality before preparation. Fake Base
reads the saved snapshot at prepare time. These use actual Rust/PG custody, not
real Docker or native resolved-secret acceptance.

Fresh Linux/Rust1.88.0/PostgreSQL project
`sdlc-qa-fleet-container-control-fb3acdec10fc` passes fmt, locked offline all-target
check, strict all-target Clippy and546 workspace cases in37 result groups, with
zero failures and30 explicitly ignored opt-in cases. All295 captured backend/SDK
inputs remain unchanged. Own containers/networks are cleaned and independently
queried empty. Six host source-loader cases, README and127 Markdown checks pass.

- Source manifest SHA256:
  `5ef037e8209b165ab3e816a88a3e8f3ab337381265450e2a859440b0351447d4`.
- Full gate log SHA256:
  `19754262834fb992ef3ba9af75954ce1903098769564fc7946b913fb7f6c03cf`.
- Report SHA256:
  `fe3499d693bc1fb794b9955b85c3ac8f20a4ffbf34096fd1bce319e4be207c03`.
- Executed environment-input source SHA256:
  `277d78a98b35f99b998cf365ce1b0bb48bfa238a7f26be0a5604949cd4dcbc0d`.

Private artifacts: workspace `.local/fleet-container-control-checks-c2c82ca061c7/`.
No migration, public DTO/OpenAPI, SDK, frontend or installed runtime change. This
input snapshot does not attest native dotenv expansion/sanitization, external
sources/managed overlays or reloads. Original effective credentials, production
collector/private checkpoints/atomic cursor recovery, bounded source policy and
controller/task/PM/Forge/live acceptance remain open. Prior UI and native Docker
packets are not relabeled as new proof. No full merge-ready claim follows.

## Private Rust Source-Page Client: 7 October 2026

The runtime packet adds private `ContainerControl::log_page` to the existing
hash-captured subprocess transport. It validates closed original receipts,
typed bounded stdout/stderr cursors, exact requested start/advancement, initial
digest, empty-poll identity and source lengths. Binary payloads stay private,
without Debug/public serialization; no source cursor or raw byte enters the DB.

Eight new Rust cases pass: binary/exited receipts; repeated-record paging/replay;
empty poll without completion; cursor shape/digest/bounds; foreign/held/open
receipts; malformed body/range/flags/digest; aggregate scan bound; actual Unix
subprocess roundtrip with pinned wire fixtures. That fixture is not real Base,
Docker, secret-redaction or durable collector acceptance.

Fresh Rust1.88.0/disposable PostgreSQL project
`sdlc-qa-fleet-container-control-98e8bed8d774` passes fmt, locked offline all-target
check, strict all-target Clippy and539 workspace cases across37 result groups.
Thirty existing special opt-in cases remain explicitly ignored. All294 captured
backend/SDK inputs match final source. The own project is cleaned; independent
container/network queries are empty. Six host source-loader cases, README and
127 Markdown-link checks also pass.

- Source manifest SHA256:
  `b398880af17a30a3afb1d1f5faf08d3192fca0f9d466dc5154afadf9ae6c8342`.
- Full gate log SHA256:
  `29ea73bd8234a8352d85a60cc8d9c223cb2f83c22d8e3918ecc447d970ce70a8`.
- Report SHA256:
  `53b170bdf5b5ac1f9bdc93e0764735ba319611d0aedb21911df0925ff1290fc5`.
- Executed container-control disk source SHA256:
  `b5619c9ee931c8146c446af9b029fa63f08d1102749d26dad1194252931030b5`.

Private artifacts: workspace `.local/fleet-container-control-checks-1ae26285d858/`.
No new migration/public API, SDK pin, frontend or installed runtime change.
The prior292/66 consumer packet is not relabeled a new UI or live PM run.
The current Base-generated protected spec rejects `logging`, while its page
reader requires an explicit blocking non-rotating profile. Bounded source
storage/retention must precede compatible policy admission and actual
Rust-to-Base/Docker/DB/authorized-stream acceptance. Original resolved-secret
snapshots, private partial checkpoints, atomic cursor/range recovery, controller
recovery and task/PM/Forge requirements remain open. No full merge-ready claim.

## Integrated Clarification Draft Labels: 7 October 2026

The independent consumer commit
`14982e15e9597e3bf7d6a9490381e0b99f7f120f` is merged normally into exact runtime
integration head `d8a8c0b1263a087b8ef5e8c3a6c331997f3229a5`. The source fix keeps
original option labels with an answer draft. Explicit transfer still removes
absent option IDs and records labels from the reviewed question version; it
never posts or selects an answer automatically.

Fresh parent checks pass292 unit cases in31 files, typecheck/build, lint/semantic
classes, format, OpenAPI/client compatibility and127 Markdown documents. The
135-screen manifest and9 validator cases pass. Six committed
[draft-label captures](assets/screens/chats-draft-labels-20261007/validation.json)
match the fresh Chromium PNGs byte-for-byte at375x812,1920x1080 and2560x1440.
Mobile renamed and desktop replaced views are visually inspected.

The combined fixture gate selects `fleet-control.spec.ts`,
`chats-directory.spec.ts` and `task-approvals.spec.ts` with one worker and zero
retries:66 PASS,0 unexpected/skipped/flaky across Chromium/Firefox/WebKit in
363.40 seconds. All102 captured frontend/lock/config inputs remain unchanged;
the private preview is stopped. Source manifest SHA256
`957fc9f52ba806addea26e97ca9e7b36f92a63aafc55080336c365c875578e02`;
browser log SHA256
`8819f3bfe15e72679ac9ed41d09df6ec5ac1ce71c0c428b0b15a6ad59bbbb28d`.
Private artifacts: workspace `.local/fleet-log-pages-chats-browser-74b1adaa0158/`.

The independent consumer's earlier56-pass/one screenshot-protocol-failure run
and its focused successful repeats remain recorded, not overwritten by this
fresh combined gate. The Base WebKit navigation diagnostic also remains open;
these repeats do not implement an SDK fix. APIs/SSO are fixtures, not a live
Tracker/Workflow/Hermes PM sequence. No backend, migration, DTO or SDK pin
changes; no fresh full Rust gate is claimed for this UI merge. Predispatch
admission, structured publication, answer delivery, checkpoint/rebind and
authorized native projection still require implementation and live acceptance.

## Verified Base Log Source Pages: 7 October 2026

Published Base control head
[`d0eedc16336386ca1a8827387d806d9c9a89b919`](https://github.com/FerrPOINT/services-base/commit/d0eedc16336386ca1a8827387d806d9c9a89b919)
adds private `log_page`, strict cursors/pages and tracked native QA. The original
namespace readback encloses the scan; page identity is stream-local byte offset
and SHA256 of the consumed prefix, not message content/timestamp deduplication.
The current Fleet consumer remains the earlier private tail method: this packet
does not claim runtime collector integration or change SDKcbb4e99.

Fresh owned Linux project `sdlc-qa-base-log-pages-9e4d6c1714d3` passes all137
runtime cases,0 skipped, including17 new cursor/protocol/pipe cases and all8
existing tail cases.81 captured inputs match current source; source manifest
SHA256 `4c05cc93b0d37d90bf1250b378afc1620e68be911164818bcc1f9817f1431179`,
log SHA256 `26af3f5c64b97ae716fe8d94357f32246cf1364c419b2d8d3e3e545f6cf370ff`.

The subsequent full Base source gate on Rust1.88.0/disposable PostgreSQL passes
fmt, locked strict all-target Clippy and63 Rust cases, with14 explicitly ignored
live/doc cases, then all137 Linux runtime cases without skips. All164 captured
inputs remain unchanged; project `sdlc-qa-base-ledger-180354ecd63b` cleans its own
containers/network. Manifest SHA256
`43a5dff1363e7bf1499c71e917b99979079cd8de8e80ae33428c124fd1fb30e3`;
log SHA256 `dced1ff8e3830b2efc8d2c0e7086d02ea7424aa35cbce2ae2e20b363f641bed1`.
Private artifacts: workspace `.local/base-ledger-checks-291a559635ba/`.
This is an additional Base regression gate, not a Fleet runtime or live PM test.

The actual Docker driver uses existing immutable Rust/Python and CLI images,
an isolated non-root synthetic source and a trusted checker with Docker access.
Project `sdlc-qa-base-log-source-71f7ce2627ff` verifies3000 stdout and2000 stderr
records (141044/94000 timestamped bytes),9 pages, identical original-range replay,
empty-poll/no-advance, changed-prefix rejection and reads after exact Compose
stop. A second genuine source with max-size1k is rejected as rotating, before
and after stop. Native source manifest SHA256
`459b6d72f20a959a679885761c7e5cc6c2eedbf31f53bf4deddeb26ab8936b33`;
probe SHA256 `63cc65b693a0a4b4985b946138539ae6827065a19eea95be8a00968c9e27ec25`.
Both owned final projects have empty container/network inventories; own parent
image aliases are removed, shared images/caches/volumes preserved. Only counts
and hashes are saved, no raw logs. Preliminary failed setup reports remain
FAILED/cleaned; native proof required correcting invalid explicit max-size=-1
to a strict effective inspected json-file policy with no rotation capacity.

Artifacts: workspace `.local/base-runtime-log-pages-7a2776674174/` and
`.local/base-log-pages-native-evidence-20261007/log-pages-u7xxwa2h/`.
Base README/hub/mirror-manifest checks pass. CI now includes both log suites,
but no new exact-head remote CI is claimed: the candidate branch has no PR and
still depends on open PR150. No main merge, installed source/log-driver change,
new Fleet migration/DTO/UI/screenshots or full Fleet Rust/SDLC gate is claimed.

Remaining: actual Rust/private protocol ingestion, immutable resolved-secret
snapshot, partial credential chunks, atomic cursor/batch/private-checkpoint
recovery, bounded disk/retention policy and authorized public diagnostics.
Original controller takeover/activation recovery and task admission/PM/Forge
acceptance remain unchanged; native byte-range proof is not their completion.

## Atomic Process Log Acknowledgement: 7 October 2026

Main-based [PR55](https://github.com/FerrPOINT/fleet-control/pull/55), head
`fdbd7dd2c6cf1d9a66651fba5c0bf92a66047529`, fixes `insert_log` using the exact
PostgreSQL `INSERT ... RETURNING` row rather than a latest-log search. Its source
baseline is main1ff9066; Base remains pinned atf04af5f. The isolated old-code
negative control fails specifically with false `NotFound` after an agent-scoped
AFTER INSERT trigger creates a newer row. It is an expected negative result,
not a failed fixed-source test or evidence from thread scheduling probability.

[Exact-head CI37534175874](https://github.com/FerrPOINT/fleet-control/actions/runs/37534175874)
passes all5 jobs: backend PostgreSQL/migration smoke/OpenAPI, minimum Rust,
docs, authenticated Compose and frontend including three-browser fixtures and
generated screenshot manifest. After ready-for-review, PR55 remains
OPEN/ready/MERGEABLE/CLEAN with all5 checks SUCCESS and no reviews/threads.
This is scoped source readiness, not a merged/installed or full SDLC release.

Fresh Linux/Rust1.88/PostgreSQL project `sdlc-qa-fleet-log-identity-3d43a05a59be`
passes fmt, locked all-target check, strict Clippy,96 workspace tests and generated
OpenAPI equality. Ten historical lineage/profile cases remain explicitly ignored;
they are not relabeled as executed. All three new log cases pass: deterministic
interleaving,64 concurrent stdout/stderr writers with exact persisted/redacted
identity/timestamp and failed FK insert without a phantom row. All164 captured
inputs still match. Exact own container/network inventories are empty after cleanup.

- Source manifest: `c23bba00a252edceb89428476f0332a2a638c2a63f7dfb477737a9810e6af8f3`.
- Original source: `b180049ad4997725363bd198f73fc65809f34969f0296df6cd1d7b21b8ecbe80`.
- Negative-control log: `52dc5f5a3481ae6d5c72ec748ab14b5d9a27fcabee2a2014fb38252618dc367e`.
- Full gate log: `d4027f843099f4e8757466500186e3e23cd97920e9e27478c5f97c1cbe71e6b5`.
- Report: `2aed026616fcdf452314544c82bb4a1f5dacb8f3f6dbd38b6c86dff3eca81b57`.

Private artifacts: workspace `.local/fleet-log-identity-checks-f1cc976b7cc7/`.
Earlierb85050658413 and475f705272dd attempts failed during test fixture preparation
and are retained FAILED/cleaned; neither is the accepted negative control.

Only this packet is transferred to runtime source1e0b9ae, preserving its SDKcbb4e99,
existing implementation/docs and migration history. Separate fresh project
`sdlc-qa-fleet-container-control-3d7ff513c70a` passes fmt, strict all-target Clippy
and3 PostgreSQL log cases without ignores. All294 backend/SDK inputs match;
owned containers/networks are cleaned. It does not rerun the prior528-case full
integration suite or actual Docker/Hermes acceptance. The test file is identical
in both source trees: SHA256
`b9780e307c0bc87f3e4649570ce80c93927dc9e04961743ed53622de566bace9`.

- Integration source manifest: `9e9922cf9c33a8d785bf4194d001d02ff12a0d88308bc1273d70c02d054acd75`.
- Focused log: `35dfcebd327659c63a9257674f28252cca2b748c52a3b014078ecb9cdaf4cf9c`.
- Report: `bbcbdfeb06669501a8acfa23750ef65ff52305763d0c7ed38b6cbc676bb6f82b`.

Private artifacts: workspace `.local/fleet-container-control-checks-fc75d5b31397/`.
README validation,87 main/127 integration Markdown links and diff checks pass.
No UI source changes or new screenshots. No public DTO, schema, dependency pin,
runtime protocol or installed services changes. Generic redaction is preserved,
not upgraded into exact per-launch credential coverage. Production Docker log
collection/cursors, controller recovery, task admission and full PM/Forge remain open.

## Integrated Chats Consumer And Base Release Gates: 7 October 2026

The seven remaining consumer commits through
`717aaba4f0ee8b6812eda5651dc17b82cc9ef229` merge normally with runtime integration
`e45d3e244b38ec8727b1e09b84a31b9eb05093e5`, without conflicts, new migrations,
OpenAPI DTO changes, dependency pins or installed runtime changes. The consumer
includes its previously published backend task-context authorization fix and
expanded PostgreSQL regression; its other changes are UI/tests/docs and the
separate unapproved PM Draft preview, not PM runtime orchestration.

Fresh combined Rust1.88/PostgreSQL gate
`sdlc-qa-fleet-container-control-f99a5321f68d` passes fmt, all-target check,
strict all-target Clippy and528 tests in36 result groups, with30 explicit
opt-in ignores. The expanded task-context/reassignment PG regression passes.
All293 captured backend/SDK inputs are rechecked against the final bytes.
Own Compose containers/networks are removed; independent exact-project
inventories are empty. This is not a new actual Docker/Hermes or migration gate.

- Source manifest SHA256:
  `83da949c33f5ae8a9c80d9bd3d2f7951bce6ce01dedc32c9deb7200a93e7fe40`.
- Full gate log SHA256:
  `fc2eb2dc5c4713b643975cab5e112602780629b7c358325f7532eb45b6617cd7`.
- Final report SHA256:
  `d4052c2e2509aed63f86b9b40698569b5ef0d9398aafa70568e1f4c81b05f8ce`.

Private artifacts: workspace `.local/fleet-container-control-checks-931d2902e70a/`.
Workspace Docker audit checks65 desktop-linux containers and0 on sdlc2-runner
with no violations. sdlc1-runner remains unavailable, so `complete=false` and
exit1 are retained; this is not a global Docker audit PASS.

Combined frontend verification passes289 unit tests, build/typecheck, lint,
Prettier, checked-in OpenAPI/client equality and127 Markdown documents. The
135-screen manifest and all9 validator tests pass. The actual three-browser
fixture command selects `fleet-control.spec.ts`, `chats-directory.spec.ts` and
`task-approvals.spec.ts`:60 PASS,0 unexpected/skipped/flaky in453.83 seconds.
It uses the combined production bundle, pinned SDK and same-origin fixture
APIs/SSO, not actual Tracker, Workflow, Hermes or installed user credentials.

Private artifacts: workspace `.local/fleet-combined-browser-20261006/`;
`final-report.json` SHA256
`c7bb3af284d7fddb47a275191cdc1d5894a81fb879c856ba1758f40da1f8f991`.
The final captures are retained separately from `initial-cross-origin-interrupted`.
That first run failed and was stopped after a cross-origin fixture setup mismatch;
its results are not relabeled. The successful run preserves the original page-error
and authorization assertions. Its owned preview is stopped and port4178 has no
listener after cleanup.

All9 committed
[command-freshness views](assets/screens/chats-command-freshness-20261006/validation.json)
match fresh Chromium screenshots byte-for-byte at375/1920/2560 widths. Fresh
desktop dialogue and mobile clarification are visually inspected. The original
35/36 WebKit failure and
[navigation diagnostic](CHATS_PM_WEBKIT_STREAM_DIAGNOSTIC_20261006.md) remain;
60/60 is a verified repeat, not a semantic Base SDK fix or live PM acceptance.

Base PR150 is independently ready/MERGEABLE at
`7d7323a59d744d50f9b101a3568adb3d59ee9683`: CI37528638132 passes all9 jobs,
including messaging/OTLP/PostgreSQL/MSRV and packaged UI checks. Local gates pass
63 Rust/PG tests with14 explicit ignores,181 Linux Python without skips,
443 host cases with12 platform skips, and13 Node/83 Vitest. Its exact-head checks
remain green after marking ready; no reviews/threads are present. The PR is open,
not merged; private log utility69831aa remains a separate dependent source branch.
Base SDK pin cbb4e99 and installed services are unchanged.

This packet does not complete production Docker log ingestion, original-custody
restart/interrupted activation recovery, producer parity/predispatch first-step
authority, PM structured tools/answer delivery/checkpoint/rebind, authorized
Workflow projection, Forge or seven-agent SDLC. No new live PM claim follows.

## Actual Private Base Docker Log Readback: 6 October 2026

Owned project `sdlc-qa-fleet-container-live-453211827a97` passes the corrected
`--log-readback` gate. The real Rust/two-Hermes/five-model-prompt scenario passes
1 test,0 failures/ignores in339.76 seconds; fresh Rust1.88 fmt/all-target strict
Clippy and test compilation pass first. All304 frozen inputs and13,770 staged
Hermes source files are verified. Source baseline is Fleet dad9c3b plus this
captured delta, SDK cbb4e99230420dc2659431b1c9fb5090e5c940f0, Base control
69831aa4d3e52312c7cc7e252c4bf01e36a950cc and Hermes
bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3.

The post-stop probe verifies captured Base source hashes and executes the real
`logs` operation with each original registration, mapped files and journal.
Four exited generations belonging to two agents have acknowledged original
receipts and nonempty output. No raw stdout/stderr is printed or persisted;
`log-readback.log` is empty. The sanitized evidence explicitly retains
`fleet_log_ingestion=false`, `raw_logs_persisted=false`, `sdlc_acceptance=false`.
This is actual Base transport, not actual Rust-client invocation or production
Fleet ingestion/redaction/cursor/rotation acceptance. Readiness rollback is not
enabled in this run; its separate earlier result is unchanged.

- Source manifest: `d4f109eca19a197bb90cacf9da4ccce4f13ce0d35bd725cf89999ecb490ac822`.
- Build log: `7f35861785dd34443fab54a32d669a5227f753dd057b2e2d3bf2cf8fe42de3bd`.
- Live log: `2595674a4dc2d210542a52eb6aa51177448d29a0f89b18ed7e293d5915f7de9c`.
- Sanitized log evidence: `a5cdea0a87c1d9bf9ab8304eb1f1e2725b2cf835ad359a1b48b6a7daa2e85e3c`.
- Final report: `8c69fd040200660a7be7da3b4ca630d2b53cb9859d1944069f8e4ad059605ed7`.

Private artifacts: workspace `.local/fleet-container-supervisor-live/`
`sdlc-qa-fleet-container-live-453211827a97-c409sbww`. Original Engine is
16c44abc-0244-4ba4-879a-b3df5140ef02; controller image
sha256:aa5f47d0a8759b0a97f7e0d06a2f1b6aa42cbeaa6d1b670ab6071f70751b9f8b,
Hermes source image
sha256:dea7a9936db4582acdcc4a3bb12292c4e65087f545e34c906ca159322c4e0542.
Exact Compose cleanup removes containers/networks, the three owned disposable
volumes and own image aliases. Independent container/network/volume inventories
are empty. Captured sources and permanent runtime lifecycle facts are unchanged.
The earlier failed extension remains failed; it is not relabeled by this run.

Final workspace Docker-group audit checks40 desktop-linux containers and zero
sdlc2-runner containers with no violations. sdlc1-runner is unavailable, so the
audit correctly exits1 with `complete=false`; this is not a global Docker PASS.

## Private Log Client Linux Gate: 6 October 2026

The additive Rust `log_tail` client and its two closed-response regressions pass
a fresh full Linux/Rust1.88/PostgreSQL gate:528 passed,0 failed,30 explicit
opt-in ignores in36 result groups. Formatting, all-target check and strict
all-target Clippy pass. Owned project
`sdlc-qa-fleet-container-control-a262dbf97328` is cleaned; independent original
Engine container/network inventories are empty. All293 backend/SDK inputs are
rechecked against the current bytes after the gate. This is not actual Docker
log ingestion or PM acceptance; no migration, public schema, UI or pin changes.

| Evidence                  | SHA-256                                                            |
| ------------------------- | ------------------------------------------------------------------ |
| 293-input source manifest | `c79b5c5d2fefc677f4b197e580478a6d66258e7e8d54fa3ff4b240b0a9fab712` |
| Full gate log             | `5d843fd753dd3dae5b181bcaef3af0ea06a4fa1338ac81ff81d8134186f2998d` |
| Full gate report          | `b1baf44ba7eb4cad688820ad50b897355ea418673b3dece950ccb25d17d5a290` |

Private evidence: workspace `.local/fleet-container-control-checks-62abd55541b3`.
Base control source69831aa4d3e52312c7cc7e252c4bf01e36a950cc is separately
published and tested:120 Linux runtime cases without skips,63 Rust tests with14
explicit broker/JWKS/documentation opt-ins ignored,13 Node and83 UI cases,
typecheck/lint and README/hub/unchanged mirror manifest checks. Its owned
`sdlc-qa-base-ledger-d82803b15286` resources are independently absent. Source163
inputs SHA256 `08dc167d66e6a28ae7c4277a63a9a23677051dd17c78f8de440e0f0a8ca6758a`;
gate log SHA256 `c8f5a09a7e8c0c6416ded70efa14d33122040c5500e40db073770d0f9bce0beb`.
These source/pipe tests alone do not prove actual Docker logs.

The first actual log extension attempt,
`sdlc-qa-fleet-container-live-3746c164eaa4`, passed its real five-run scenario,
then failed because its driver called an undefined function before the log
probe. Its final report remains failed,cleaned with source/permanent runtime
unchanged; it is not log acceptance. The corrected driver has14 passing safety
units, including failure-state retention and insufficient/overclaimed evidence
refusal. Combined loader/README/driver checks pass23 host cases.

Fresh remote inspection finds Base prerequisite PR150 still OPEN onmain at
424ad76b1fc0c976e465e9de272f71a3b03a45b6 with9 successful exact-head checks,
but now CONFLICTING against main e4f0cda89c18bd625fa4cedcfe6284a568aaa573.
The new logs source branch is not a main release PR: main lacks runtime_control.py
and its diff includes the prerequisite. No stacked/oversized release, unrelated
dependency edit, pin change or installed opt-in is performed.

## Actual Docker Readiness Rollback: 6 October 2026

Owned project `sdlc-qa-fleet-container-live-0763697334fb` passes the actual
Rust/Docker gate with explicit `--readiness-rollback`: 1 passed, 0 failed,
0 ignored in 405.80 seconds. Source baseline is Fleet
`b7fe9084c861c68d528b976e7ce09bb012878f49` plus the captured test-only delta;
SDK `cbb4e99230420dc2659431b1c9fb5090e5c940f0`, Base control
`424ad76b1fc0c976e465e9de272f71a3b03a45b6` and Hermes
`bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3` are unchanged. Fresh Rust 1.88 fmt,
all-target strict Clippy and exact integration-test compilation pass. All 13,770
tracked staged Hermes files and 303 frozen gate inputs are verified.

The QA-only wrapper delegates ordinary boots to the pinned production launcher;
an explicitly marked candidate delays boot 120 seconds, exceeding the unchanged
60-second readiness deadline. The real acknowledged candidate remains drained
without effective-revision promotion and then fails for readiness. Both the
original and failed candidate report original-context namespace exit. A distinct
rollback launch restores the previous revision, byte-identical `config.yaml`,
`SOUL.md` and `.env`, effective filesystem verification and the previous loaded
SOUL. The peer generation is unchanged. A sixth actual Hermes/model prompt
completes and mirrors once; the activation journal is retired. Cross-agent token
denial and idempotent message replay still pass. The model endpoint is controlled;
this proves runtime transport/isolation, not model quality or live PM.

| Evidence | SHA-256 |
| --- | --- |
| 303-input source manifest | `6d3b817776fbaaf60ba0d2c3969549d139d1ec40e5cb7436ffb0e4e255cc597c` |
| Build log | `0a86f47cdae793ebc76044ec3fe949f57d5dd27c0fe69be69ce0473df58e5a73` |
| Live log | `5038788cf6ed268a9c8dad392c15b0393588298eec389f51d7f86cfc2d5fbf47` |
| Final report | `67e7b11452615c4d491077bde3eb7dea7d315dd9eb1d0597a114fdb961b8a734` |

Private artifacts are in workspace `.local/fleet-container-supervisor-live/`
`sdlc-qa-fleet-container-live-0763697334fb-kw_mlrun`. Controller image is
`sha256:076f31d5379ec5ba92d85b93d2d7006a5fe47b0eafaa400f52d9829945052816`;
Hermes source image is
`sha256:f42cb0b1115b587eb21c650ab9f1120c94911fc6234748c634faff929622f193`.
Original Engine `16c44abc-0244-4ba4-879a-b3df5140ef02` is unchanged. Exact owned
Compose cleanup removes containers, networks, all three disposable volumes and
image aliases. Independent Engine inventories are empty. Sources and permanent
runtime lifecycle facts remain unchanged.

All 11 driver/fixture safety units pass, including ordinary argument preservation,
owned fault timing and unreadable-config refusal. The earlier owned attempt
`sdlc-qa-fleet-container-live-8b07fea89928` was stopped during compilation to fix
the test's expected readiness error; it is retained as failed, cleaned evidence,
not actual scenario acceptance. No production runtime, migration, UI or pin is
changed. Prior 526-test/19-migration results below remain historical production
evidence. Controller crash, interrupted activation/private-journal recovery,
actual Docker log ingestion, producer admission, PM tools/delivery/resume and
full SDLC remain open. Instructions: [Container supervisor gate](CONTAINER_SUPERVISOR_ACCEPTANCE.md).

## Actual Docker Supervisor Chat And Configuration: 6 October 2026

Owned project `sdlc-qa-fleet-container-live-e823d8ddeef4` passes the opt-in real
Rust-supervisor/Hermes scenario: 1 passed, 0 failures/ignores, 289.11 seconds.
The controller runs as UID 999 on the original Engine, using separate agent
subpaths in its own named volume. Both source-pinned Hermes containers perform
five actual `/v1/runs` against a controlled local model. Each final answer is
mirrored once. The model observes isolated SOUL, and the wrong peer token is
denied. Identical message replay creates no second model call or run. A held
response proves drain before file/replacement effects; the new generation loads
the replacement SOUL, the peer is unchanged, and peer restart creates a fresh
generation with its original SOUL. Both original namespaces are confirmed stopped.

The first actual attempt exposed the old localhost-only Rust and DB journal rule.
Additive `000019` now seals the Base-verified endpoint to original launch/PID;
preparation/submission require that exact origin and private launch ID. Earlier
failed preflight/Clippy/live attempts remain preserved. The initial nested-network
cleanup issue was separately recovered on its original Engine, then fixed with
referenced cleanup-only services and a regression. No failed gate is relabeled.

The successful gate verifies all 13,770 pinned Hermes source files, frozen 302
Fleet/SDK/control inputs and Rust 1.88 fmt/all-target strict Clippy before fresh
test compilation. SDK pin is `cbb4e99230420dc2659431b1c9fb5090e5c940f0`, Base
control source `424ad76b1fc0c976e465e9de272f71a3b03a45b6`, Hermes source
`bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3`. Source manifest SHA256:
`f89cf50ce73468cdddfa16fd73b19eb955b1a1a778be67da97d37db0595271b8`;
build log SHA256:
`7c533a5ee6a79e0fcc6a5d7158eac92d6197a5f51146562a356533405cd8ebe6`;
live log SHA256:
`45c53cac6a327b82cb9790a1713a79c03d4e1459459987d4db6f062f2c253127`;
report SHA256:
`d00466f6fca607acb58a3a7e629bdf0dd56e7bf68c7471068e0fb0ecb1f73f41`.
The gate captured this packet before publication over `9c3cad0`, not that parent
alone. Own containers/networks/three disposable volumes and image aliases are
removed; independent original-Engine inventories are empty. Captured sources and
permanent `sdlc1`/`sdlc2`/`sdlc-common` lifecycle facts are unchanged. Eight no-Docker
driver safety tests and 122 Markdown files pass.

Fresh Linux/Rust 1.88 workspace verification in owned project
`sdlc-qa-fleet-container-control-c8139d6fcc21` passes fmt, all-target check,
all-target strict Clippy and 526 tests in 36 result groups, 0 failures and 30
explicit opt-in ignores. The separately successful actual Docker test is one of
those ignores in the ordinary suite. New PostgreSQL cases verify original
endpoint custody, concurrent identical replay, immutable endpoint history and
rejection of arbitrary origin/generation.

The separate migration gate `sdlc-qa-fleet-container-control-52953f25be44`
passes all 19 cases with no failures/ignores on ten disposable databases. Fresh
canonical/split installs, empty latest down/up and populated historical upgrades
pass; this does not certify downgrade of populated endpoint history. Both gates
freeze the same 293 backend/SDK inputs, unchanged after execution, manifest SHA256
`ad852f623273603b445456a0b3b6e97bb60e7543d4f946cc878f903f82f05adb`.
Workspace log/report SHA256:
`ee586947eef5688cd53b606a2e819a7ac67b0a30693c51a2b9d50d85edb5d687` /
`cf4363fd26f4355adc397c8adc0f224b635fe5d8b8c81a5fb4a98ba24211a00b`;
migration log/report SHA256:
`40f3bd3902e3bce1165be5e6649c3a753b2c33f7a2603eca9b9b4b8c4ef9af7e` /
`9d4e9da5b893de3ade02c59cb06f069a44cd16ef95caa0a1c90cca6f4a411568`.
Owned resources are removed; independent original-Engine inventories are empty.
No API/frontend/SDK pin or installed runtime changes follow from these gates.

This is real controlled-model Docker/free-chat/config evidence, not installed
enablement, actual Docker readiness rollback, controller restart/private-journal
loss recovery, Docker log ingestion, task admission, live PM or full SDLC
acceptance. Ordered release PRs and exact-head CI remain required; local PASS is
not an installed release. See [runbook](CONTAINER_SUPERVISOR_ACCEPTANCE.md) and
[ADR 0030](adr/0030-generation-bound-container-endpoint.md).

## Combined Chats Follow-Up: 6 October 2026

Normal merge `c02f60ebdceebaa4255352d27893976fc26c1389` preserves container
configuration parent `fce4df739fcd7f5294d376e53ec0377d66b043f1` and consumer
parent `77fc54320f7aa39b3d2688adfcd2aadcf9bf6403`. All 291 backend/SDK inputs
remain identical to the passing 523-case configuration gate below; this is
retained exact-source Rust evidence, not a fresh post-merge Rust run.

A separate clean Git checkout at this merge and Base pin
`cbb4e99230420dc2659431b1c9fb5090e5c940f0` passes frozen install, API client
generation, typecheck, lint, format, 268 frontend tests in 31 files, OpenAPI
client/compatibility, local chat contracts, build and Markdown links.
Compatibility uses real `origin/main` at
`3c6b8ef7bdb08f799ca30e0a5d7537914ce40ab6`; local chat-contract success does
not resolve the three published Tracker schema differences. Pinned Base and
its installed UI snapshot verify; the Base UI checker passes 38 route patterns.
The 235 frontend/OpenAPI/SDK inputs remain unchanged. Manifest SHA256:
`a1cbcb06373e4f1e8fcbcbcb7cc6ba401667c86e7be3c07e6e8be0d27408107a`;
unit log SHA256:
`51f2d794b234bbfa8544f942e5ac1d8f744f9fed8f1d5387d0b6c05df8190b1b`.

The proper canonical-localhost, same-origin fixture run passes 35 of 36 cases:
Chromium and Firefox pass all 12 each. WebKit passes the PM form, answer,
confirmation and screenshot actions but its final no-page-error assertion
reports a session SSE access-control error. This is an open browser gate, not
a green suite or established root cause. Browser log SHA256:
`da172373d081c5e7f38554601c700a6193e0740fc7ef59f9ca88203c63cdbe01`;
report SHA256:
`6e7983e1d75c7edfe1f2a845652e078a73b8b01d13c225acb6e01d8db25fc7c0`.
The independent Chats task receives this exact reproduction; no error filter
or production authorization bypass was added. Earlier archive/ref and
127.0.0.1/cross-origin harness failures are preserved separately. The interrupted
intermediate browser run and its descendants were explicitly stopped; every
owned preview terminated. No candidate was installed and no Docker runtime changed.

All 13 review source Git blobs and 10 captured review image hashes match.
One raw working-file discrepancy is only CRLF; LF/Git-blob identity matches.
Historical screenshots are not relabeled as fresh live acceptance. Fresh
Chromium mobile clarification and desktop dialogue were visually inspected;
the full 21-image publisher was not run because the browser gate failed.
README and 120 Markdown files pass. Global Docker grouping audit is incomplete:
available engines have no reported violations, but `sdlc1-runner` is unavailable.

Live PM publication/admission/first step, durable answer delivery and
checkpoint/resume/rebind/history remain unaccepted. Real mapped-volume
Hermes/model/config lifecycle and complete recovery remain open as below.
This integration branch is not a full merge-ready release PR.

## Container Configuration Replacement: 6 October 2026

The candidate adds exact phase/revision/hash admission before preparation as
well as start, original namespace stop before file effects, and fresh activation
and rollback generations. Stopped configuration apply remains stopped. Unknown
prepare/start/stop or foreign custody retains candidate files and drain instead
of inventing a rollback effect. No new migration, Base wire protocol, public
API/frontend/SDK pin or installed flag/image/volume changes.

Project `sdlc-qa-fleet-container-control-0ae96e07367e` passes Rust 1.88/Linux
fmt, all-target strict Clippy and 31 focused PostgreSQL/fake-Base supervisor
cases: 8 new configuration cases, 17 container lifecycle cases and 6 prior
configuration lifecycle cases, 0 failures/ignores. The readiness-failure case
executes the real bounded readiness wait and fresh rollback path. The tests
use a fake runtime HTTP API, not actual Docker/Hermes/model inference.
All 291 frozen backend/SDK inputs match source. Source manifest SHA256:
`9e616ae325556ea5110cad820f4098b6337cf9ff2511a519678642cbc301d826`;
log SHA256:
`03c77f50189012797d58c9658bd8db464f103545120aaf1d9595c4b96b85f3ea`;
report SHA256:
`064f7d7a902fc30c54172084ccc46d7f40a45ef9f669442b1027adc80c1149a6`.
The initial frozen configuration attempt had 3 passing/3 failing cases because
it queried a next ordinal before old namespace exit; this ordering was fixed
before the successful source capture. Its owned project is separately cleaned.
Independent original-Engine container/network checks confirm successful cleanup.

The full workspace project `sdlc-qa-fleet-container-control-c2f2217d36e2` passes
locked/offline Rust 1.88/Linux/PostgreSQL fmt, all-target check, strict Clippy and
523 cases in 35 result groups, 0 failures/29 explicit opt-in ignores, on the same
291 frozen inputs. Log SHA256:
`59b0c78e13babaecbbdf05ff03ba89525c3d16a1b2f7f02d288662ce44105a50`;
report SHA256:
`923a4debd47b8da903883bbd05c82990951721fe4bc0c4e599081e81e6ab3a2c`.
Original-Engine container/network inventories are independently empty.
All 39 migration/crate-root inputs are unchanged from the preceding authoritative
19-case migration gate on ten disposable DBs; that evidence is retained, not a
fresh migration run. README and 120 Markdown files pass. No frontend changed in
this packet, so no new screenshots/browser run is claimed.

Real mapped-volume
Hermes/model/config lifecycle, loaded configuration evidence, complete interrupted
activation reconciliation/takeover, producer admission/PM/Forge/seven-agent
acceptance and ordered release PR gates remain open. Existing UI evidence below
belongs to its original source. Chats follow-up `77fc543` was subsequently
normally merged into `c02f60e`; fresh frontend evidence is recorded separately.

## Durable Container Pre-Create Fence: 6 October 2026

The new additive000018 preparation ledger commits immutable agent/history
ordinal/controller/generation/operation/intent hash before private-file creation
and Base prepare. Credentials remain private; the database holds only the hash.
Exact original replay is allowed; missing intent/directory, changed controller,
changed identity and competing claims cannot authorize a replacement effect or
native fallback.

Fresh project `sdlc-qa-fleet-container-control-f02c1d9715d7` passes locked/offline
Rust1.88/Linux/PostgreSQL fmt, all-target check, strict Clippy and515 workspace
cases in35 result groups, with0 failures and29 explicit opt-in ignores. The
three new supervisor/database regressions execute, including whole private
directory loss with the Base utility still available; the hold therefore does
not merely result from a missing executable. Workspace log SHA256:
`50993f6d68d8dbfc1ec6b6c59d64c6efd3d9cebbf69baf545f0fb86851354f3e`;
report SHA256:
`f52589a2a5e594ddcf69693ce8cd70e52992b52173fa53349b0f0962dac29f71`.
The first compile attempt and a second incorrect directory-loss fixture failed
and remain separately recorded; neither is counted as successful evidence.
Both failed projects were cleaned before the final source capture.

The separate migration project `sdlc-qa-fleet-container-control-51a6a698e2da`
passes19 cases with0 failures and0 ignores on ten distinct disposable PostgreSQL
databases. The new test executes actual upgrade, empty downgrade/reapply,
populated launch-history preservation, immutable row guards and retained-fence
downgrade denial. No accepted runtime database is used. All290 frozen source
files match the full workspace gate's capture; source manifest SHA256:
`c0944d087c1b25f359aab02d94c72982d7eec95a0e9e34ea943e4d7c8565064d`.
Migration log SHA256:
`131c251df77882094d336e01f9dd334704a468d2204a10fea44dda84342744b3`;
report SHA256:
`5752908961f3a5f7fb670db5cd6e4e22aca1200a3174b0d44562184a5f23abbd`.
Independent original-Engine container/network checks confirm cleanup for both
successful projects. The migration-only case returning early without its
dedicated URL in the workspace run is not migration acceptance; the separately
configured gate above is authoritative.

This is not full storage-loss recovery, controller takeover, actual mapped
Hermes/model/chat or container configuration lifecycle acceptance. No installed
flag/image/volume, public API, frontend or SDK pin changes. Existing consumer
frontend evidence below remains tied to its original bytes; parallel Chats
changes require their own integration. Base PR150 now targets main normally
merged in `424ad76b1fc0c976e465e9de272f71a3b03a45b6`; all9 fresh exact-head jobs
in CI37480593149 pass, ready/MERGEABLE with no reviews/threads on recheck, but
not merged or installed. The runtime utility bytes are unchanged by that merge.

## HTTP Timeout Follow-Up: 6 October 2026

The second normal merge adds consumer
`d592a0d06f71176861c8b75da2b012b133187aae` on top of the published combined
candidate `c2b383fdd45725cc9bd00ea5ea4111cfca51daa5`. HTTP408 is an unknown
command outcome: the captured original payload/key survives a saved answer or
confirmation and an explicit retry cannot become a fresh command. Three unit
regressions and the browser answer/confirmation lost-ACK scenario cover it.

Fresh integration-tree262 Vitest cases, build/typecheck, lint and format pass;
all33 fixture browser cases pass in Chromium/Firefox/WebKit. The21 preview
images are regenerated from this new run and source/hash/dimension verified;
preview is not live PM acceptance. OpenAPI client and Base UI checks pass.
The published source identity review remains conditional compatibility on the
shared Tracker/Workflow identity subset, not proof of deployed continuation.

All288 backend/SDK source files are hash-compared with the preceding successful
Rust511-case and migration18-case frozen manifests and remain byte-identical.
Those backend results are retained exact-source evidence, not a new Rust run.
No backend, schema, runtime, SDK pin or accepted deployment changed in this
follow-up. The same real runtime/config/recovery/producer/PM/Forge gates remain.

## Combined Chats And Runtime Candidate: 6 October 2026

Integration merges runtime parent `7564e2e2ad8e1246cb54f38ca2d11fd765d58fd1`
with Chats `0ecee7eaac7fca4ec60bc813d02ae5e037bd3a67`; all three independent
consumer commits and both published histories are preserved. Only the
CURRENT_STATE introduction conflicted; both source-specific blocks survive.
This gate is for this exact combined source, not later parallel Chats changes.

Fresh owned project `sdlc-qa-fleet-container-control-14d3d92374e8` passes locked,
offline Rust1.88/Linux/PostgreSQL fmt, all-target check, strict Clippy and the
workspace tests:511 passed,0 failed,29 explicitly ignored in34 result groups.
The original mapping fake-Base consumer and merged owner-command gateway
regressions execute together. All288 captured inputs remain unchanged.
Source manifest SHA256:
`9e9b4b6fe07713ee46923beae88f2bf4619529b16a1ab07e95f2f074da53a019`.
Log SHA256:
`dd0df251315981f7145edc3368d449b4327fdc62cada4c92fba7b3d195db726f`.
Report SHA256:
`3ad746b5c8dabe83e915677e1a5081e7640d2b348dd2983c1c966900e6fc5009`.

Separate project `sdlc-qa-fleet-container-control-9f0f57e17d2e` executes all18
migration tests with0 ignores and0 failures, using nine distinct disposable
PostgreSQL databases for lineage, transcript order and each journal upgrade.
Captured source identities equal the workspace gate; log SHA256:
`faa15e69f67fead620b3190e35433b6e2a60d8111110d1d7f341253a24c4470a`;
report SHA256:
`7d549879752466c27519e9e81c5b7fcc4bb549e5daaf8fca77f2c7a2ef1dc984`.
Its first attempt `sdlc-qa-fleet-container-control-539b49c4624f` failed because
the harness omitted the dedicated empty message-order DB. That failure is
retained separately; the corrected gate supplies every required isolated DB.
Both successful projects and that failed project have independently empty
original-Engine container/network inventories. No permanent resources changed.

Node22/pnpm10 combined-tree typecheck, lint, format,259 Vitest cases and build
pass. The existing >500 KiB bundle warning remains. The full Chats/Fleet browser
suite passes33 fixture cases in Chromium/Firefox/WebKit. The21 consumer preview
screenshots are regenerated and source/hash/dimension verified. Desktop dialogue
and mobile single-choice clarification were visually inspected. Base's installed
UI checker, OpenAPI generated-client/compatibility, seven local contract checks,
README,118 Markdown files and six captured-source loader tests pass.
WebKit fixture teardown still emits mock-upstream proxy warnings; this is not
evidence of a live upstream. Preview remains `liveAcceptance=false`.

Read-only recheck preserves Tracker114 `8c80a41` (open Draft, conflicting) and
Workflow90 `e4fba60` (open Draft, mergeable), with the documented three-schema
drift and missing installed predispatch authority. Base PR150 remains ready,
mergeable on `98a5bbd`, with nine successful exact-head CI jobs, not installed.
No giant integration-tail PR, release, SDK pin or deployment switch is implied.
Mapped real Hermes/model/chat, config drain/replacement/rollback, logs/controller
recovery, private-intent loss/restore, producer admission/first step, PM tools/
continuation and Forge/seven-agent deployment acceptance remain open.

## Rust Named-Volume Consumer: 6 October 2026

Source `d924799be5ec77935b71decce20059f22b919e10` implements closed mapping DTOs,
read-only resolution and private protocol2/policy3 lifecycle in Fleet. Original
mapping/file/digest bind intent, preparation, registration and DB launch; daemon
projections are not host binds. Current controller/root, local marker/path guards,
Engine/digest, original recipe and private proof stay authoritative. Native
records preserve omitted fields. Unknown start cannot dispatch a second effect.

Fresh owned project `sdlc-qa-fleet-container-control-edc53a6eb393` passes the
Rust1.88 locked/offline Linux/PostgreSQL workspace gate: fmt, all-target check,
strict Clippy and509 passed/0 failed/29 explicitly ignored in34 test groups.
Three new control units and two PG lifecycle cases cover projection/recipe,
digest/Engine/downgrade/sibling guards, original replay, changed controller/file,
unknown preparation/start, original ACK recovery and namespace stop. Base in
these consumer cases is a pinned fixture, not actual Docker/Hermes inference.

All288 captured inputs remain unchanged; five changed backend Git blobs are
raw-exact with passed bytes. Source manifest SHA256:
`5f9cb0f8ae3a479fe8f7106230ffee02c22ebf2387f1e1bb59fe3cf0ad83c410`.
Gate log SHA256:
`8a43fb18dc8fc38abaebc6896c4ff7856ef7a035785018d970deea065b4aa9be`.
Report SHA256:
`db2b0ef1762c1feed43a5801ce64effa2cc6e13ac48ac3a98937502b1752be28`.
Own down exits0 and separate original-Engine container/network inventories are
empty. OpenAPI generated-client parity and117 Markdown links pass; no API/SDK/
migration/UI/screenshot or installed-runtime changes. Preliminary compile
and Clippy failures are retained separately; both own projects are clean and
their partial checks are not substituted for this final gate.

Actual Rust Fleet mapped Hermes/model/chat, config drain/activation/rollback,
logs, controller restart/takeover and private journal loss/restore remain live
gates. The published read-only producers still lack accepted predispatch/first-
step authority; PM/Forge/seven-agent acceptance and ordered release PRs remain.
The Base utility evidence below is distinct from this consumer gate.

## Named-Volume Subpath Lifecycle: 6 October 2026

Base sourcee083651 / published98a5bbd adds private protocol2/boundary policy3.
Docker rejected rprivate bind sources inside its own data root; rslave was not
accepted as fallback. Exact external named-volume subpaths now identify one
agent's four areas. Immutable mapping file/hash binds preparation/registration;
fresh original controller/Engine/volume/local guards precede effects and readback.
Protocol1 downgrade, root/sibling/foreign/options drift and lost ACK do not grant
another effect. Existing host-bind policy1/2 explicitly sets rprivate.

Native0a1f2bdd97e1 PASS:30 Linux control cases/no skips and actual mapped
create/replay/attach/start/endpoint/namespace stop. Synthetic runtime UID/GID999,
RO runtime/RW own areas/no socket/agents root, local symlink denial before start.
10 frozen inputs unchanged; seven changed staged/published blobs raw-exact.
Report SHA256179a98a283848967f96decf630271f8c0b61dc5a5bcb9095a93c7d2c16273853.
Own Compose cleanup/volume/tag deletion/permanent freeze pass. After normal merge
main846a5fa,408 host cases:396 PASS/12 skips;26 Node contract cases, README/hub pass.
Fresh two-real-Hermes host-bind project90158c9d02aa also PASS:13770 pinned source
files, trusted peer attachments, authenticated health/capabilities/cross-token
denial and two original namespace exits. Nine executable inputs are unchanged
and raw-exact with published98a5bbd. Report SHA256
54236e2d41987825709277f829234e8b3053b4d1e10e694c14701bf5e77dc29c.
Exact cleanup0, removed own tags and independent project inventories verified.
No Rust Fleet,
mapped Hermes model/chat/config/PM acceptance; the two-real-Hermes host-bind
regression is a distinct gate. Current PR150 is ready/MERGEABLE with all9
exact-head CI37465043730 jobs SUCCESS, reread after ready. Reviews/threads empty;
remote body matches main template3/7/2. It is not merged/installed.
The previous read-only/ready observations below remain historical.

## Named-Volume Mapping Prerequisite: 6 October 2026

Base source735d74b / published2bcf3d29b7b46ebe0f556b9143c21ae1cd7d4e26 adds
read-only resolve_mounts and retains accepted main999b0e4 by normal merge.
It validates exact controller image/service/PID/inventory/Engine and selected
real local named-volume metadata. Four mapped areas must belong to the same
agentN; nested/aliased mounts, foreign volume/project, links, path escape and
intermediate/readback drift reject without create/start/files/journal writes.
Raw env/inspect inventories are hashed, never returned in the receipt.

Final mapping project `sdlc-qa-mount-mapping-4e80cc716581` PASS, UID999:
real daemon/controller path difference, positive filesystem read/write, unchanged
mapping replay;21 Linux control tests with no skips. All8 frozen files unchanged;
four changed executable published Git blobs raw-exact with the passed inputs.
Report SHA256 `cb09d6cab2ad2213472ac252bb7d8f1e03b5ea801c495adf32518fff0f2059fb`.
Own down, exact disposable volume/tag removal, source/permanent freeze and
independent project container/network/volume inventories all pass. First030de
failed in disposable initialization before the mapping call, then cleaned; QA
chown order was corrected without changing installed ownership or guards.

Fresh real-Hermes project `sdlc-qa-hermes-container-20f7b57334b7` PASS with the
new Base control bytes: original trusted-peer attachment before two starts,
13770 pinned source files, health/capabilities/cross-token denial and two original
namespace_exited receipts. Nine executable inputs remain unchanged and raw-exact
with published2bcf3d2. Report SHA256
`cbd4b46928b33f8ed167fc8358afd65af940361560393e2f67f561cab5990504`.
Cleanup0, containers/networks empty and own derived tags removed, independently
checked. No inference submitted; this is not Rust Fleet/model/chat/PM acceptance.

After main merge:390 host cases,378 PASS/12 explicit skips,26 Node contract tests,
README/hub/mirror validation PASS. Base PR150 is ready/mergeable with nine
successful exact-head CI37461023808 jobs, reread after ready; reviews/threads
empty. Remote body verified3 headings/7 checklist items/2 comments from current
main template. Old CI37458583952 does not certify the new code. No merge/install.
Fleet runtime code/source pins, API, DB, UI and screenshots were not changed.
Original mapping intent plus mapped-lifecycle guard/consumer wiring are still
required; current lifecycle cannot use the daemon projection as a local path.

## Current Mount Evidence And Base Reconciliation: 6 October 2026

Base PR150 head368cfb8205e9cac578a353ddfb5a1e83befa4484 normally merges main475c694.
The only textual conflict was CHANGELOG; both entries are preserved. No force
push or rewritten commits. The Base runtime boundary/bootstrap/control and
Hermes live harness bytes are unchanged from0e14ddf. Full local host discovery:
377 cases,366 PASS/11 explicit skips;26 Node runtime-contract cases PASS;
README/hub/mirror manifest and diff checks PASS. CI37458583952 passes all nine
jobs for this exact merged head. PR150 is ready/mergeable; head/checks reread
after ready, reviews/threads empty. Remote body verified against main template:
three headings, seven checklist items, two HTML comments. No merge/install.

Read-only Docker inspect of `/sdlc1-fleet-backend-1` and
`/sdlc2-fleet-backend-1` reports immutable image
`sha256:c0f6b1a5d2c0b8d824e253b8874d33bd92398d0cb8545dcb04446c1dd8e71572`.
Each maps its separate named-volume daemon Source
`/var/lib/docker/volumes/sdlc1_fleet_agents/_data` or
`/var/lib/docker/volumes/sdlc2_fleet_agents/_data` to controller Destination
`/var/lib/fleet-control/agents`, RW. Read-only `id` inside each existing backend
confirms UID999/GID999. No env secret values or raw inspect inventories published.

This changes the next implementation action: current container_intent policy
uses AgentPaths directly, but those paths are in the controller namespace, not
the daemon namespace. The Base guarded_mount_sources also checks paths locally.
A verified mapping must bind the exact trusted controller mount and Engine,
keep agent-relative guarded paths and exclude controller/sibling storage. UID
access must be tested with the real selected identities; the previous startup
QA's UID10001 and host directory bind do not prove named-volume access.
No agent launched, no ownership changed, no permanent container restarted and
no model/chat/PM acceptance claimed by this inspection.

## Trusted Fleet Bridge Consumer: 6 October 2026

Optional private bridge_controller pins exact container/image/service. Fleet
calls captured Base attach_controller after preparation, before DB launch claim
and start, and on endpoint resolution. Original creation intent includes the
controller selection. No public API, migration, SDK pin or installed runtime change.
The fixture proves unknown attachment has zero launch/start; success precedes
claim/start. Prepared recipe drift now includes the controller alongside earlier
credential/process checks. Fixture start remains a fake unresolved ACK, not health.

Full Rust1.88/Linux/PG project `sdlc-qa-fleet-container-control-e398f08cd693` PASS:
504 cases/0 failures/29 ignored; fmt, locked offline all-target check/strict Clippy.
All288 captured inputs unchanged. Source/log/report SHA256:
`9f1df38614ecec519a4fe7607a0c2371c20a7177d3b0e9a55839c7aa1013e784` /
`e2f7eeba28d73b3d3843d40e6bf9c31ae72049ce3f43a4b1b310c2cef005e18f` /
`5ac04d43eba4cae93db1d16e88026552f460b5f30a9977be16882f784c2be4c4`.
Artifacts: ignored `.local/fleet-container-control-checks-9852af80b644` in workspace.
Cleanup0 and independent exact-project container/network inventories empty.

Base nativea208dec88b45 separately proves original trusted-peer attachment/replay
before start, real Hermes health/required capabilities/cross-token denial and two
namespace_exited receipts.13770 pinned files, nine unchanged executable inputs,
cleanup0/empty inventories/own tags removed. Report
`e7bea6bf0cfe9bf5f6752ceb66a5ff0324683a3e2ae3172136de7f2d3688b60d`.
First7ae QA bytes-ID failure was cleaned; only harness ASCII decode changed.
Host Base362:351 PASS/11 skips after normal mainefa21dc history merge. PR150 at
0e14ddf was ready/mergeable at that check; nine exact-head CI37456080881 jobs successful after
ready and reviews/threads empty. Prior604 CI is not evidence for these bytes.

This certifies Base network/startup and fake Rust consumer separately. It does
not certify actual Rust Fleet supervisor/model/chat, daemon-path/UID, config
activation/rollback, Docker logs/controller takeover or PM admission/resume.
No live browser/screenshot capture; no merge/install/accepted resources change.

## Prepared Container Recipe Guard: 6 October 2026

The follow-up reconstructs the current automatic recipe using the original
generation/operation IDs before claiming a saved prepared generation. It checks
the complete creation intent and reuses the original preparation validator for
registration/policy, allowing only the allocated network ID. Obsolete process
settings or derived runtime API credentials cannot reach observe/claim/start.
Missing intent or disabled automatic provisioning also holds an existing
automatic generation; explicit legacy operator documents remain compatible.
The original private files are never rewritten to match new settings.

The new Linux/PostgreSQL regression exercises token, image, entrypoint, user,
memory, network and CORS changes after successful preparation. Every changed
case has no start effect or DB launch, while original files/generation remain
unchanged and the unchanged recipe still validates. Base is a subprocess fixture,
not actual Docker/Hermes, and this does not attest model secrets or loaded config.

Fresh Rust1.88/PostgreSQL full workspace gate passes503 tests,0 failed,29 ignored,
fmt, locked/offline all-target check and strict Clippy. All288 frozen inputs are
unchanged. Project `sdlc-qa-fleet-container-control-044c488dcee1` exits0 with exact
cleanup; independent container/network queries are empty. Evidence SHA256:

- Source manifest: `8fe0184e88c6908c84b77f45b097673b3e3fcd7f9f280c149915f071be29019b`.
- Full log: `2c3215d3431bea57b7d2c10a61f8a7c71b21ac39aedd0f1e6af2809418e54397`.
- Report: `f3a6ff4dda0763a3e58c44906a6a250468b505854a8a79454f714b3725974165`.

The first focused attempt, projecta98fc10bcfce/report directory0a611e612c89,
failed compilation because the regression could not access a private method.
The method is now parent-module-visible, not a public runtime API. That attempt
cleaned its own resources; it is not evidence of a behavioral pre-fix failure.
Host loader/README/native harness safety suites total64 PASS. README and the
117-file local Markdown checker pass;135 existing fixture screenshot hashes
verify across three viewports. No new browser/live UI evidence is produced.
Docker audit has no violations on desktop-linux38/sdlc2-runner0, but sdlc1-runner
is unavailable: complete=false. Accepted images/services, SDK, public schema and
migrations are unchanged. Fleet container model/chat, loaded revision,
drain/activation/rollback, admission/PM and complete SDLC remain required.

## Captured Base Source Loader Candidate: 6 October 2026

Published hardening compiles the three captured SHA256-verified Base sources,
not a second filesystem import or cached bytecode. Captures are bounded to1 MiB
per file and all source-path components reject symlinks/junctions. The synthetic
package has no checkout search path; the existing binary-stdin Base contract,
64-KiB request/output bounds and60-second deadline are preserved. The already
resolved base640.22 dependency moves from dev-only to normal infra dependencies;
no lockfile, SDK pin, migration or public DTO changes are required.

`python -B -m unittest scripts.tests.test_container_control_loader -v` verifies
six behaviors: binary stdin/pinned imports, valid poisoned bytecode (the old
import path is independently proven to read it), replacement after capture,
package initializer exclusion, unpinned module rejection and source encoding.
The exact Rust-owned bootstrap is exercised, not a duplicate Python loader.
The same test command is added to CI. This is host bootstrap behavior, not Rust
client compilation or Docker/Hermes acceptance.
All six loader behaviors pass independently on Windows and WSL/Linux. The
Windows loader/README/native protocol/native supervisor suites total64 passing
tests; Rust1.88 fmt and README structural verification also pass. These suites
do not run the new Rust client tests or confirm previous QA resource cleanup.

An independent host smoke loads the actual three Base PR150 executable sources
and reaches its expected typed invalid-request rejection with no stderr or Docker
effect. Their hashes match the published files:
`a20cee93e7e6c27ba542872f208424facc15d030758e9c02d0f104c152aa109a`,
`64d8f829aced830058134c90bd36f7263e8607dbceb85cda64692961f0f9986e`,
`8b8537cb450b7c6bfdf55e1c5773756b3115d8686cd2939d27b6d0ffb1d2dd3a`.
This import/rejection smoke does not test a successful container operation.

Four new Linux Rust regressions cover poisoned cache, missing package authority,
hash/path drift and an oversized hash-pinned source. All four pass in the final
fresh full Rust1.88/PG gate, together with history-ordinal restart and three new
project/Java pre-effect guards. The final gate passes502 tests,0 failed,29 ignored;
fmt, locked/offline all-target check and strict Clippy pass. At the
initial check, desktop-linux reported a missing pipe. The original Engine then
returned with identity `16c44abc-0244-4ba4-879a-b3df5140ef02`; exact-project
recovery revalidated manifest/owner/purpose and cleaned only
`sdlc-qa-fleet-container-control-3b96063dfdea`. Independent container/network/
volume inventories are empty; permanent sdlc1/sdlc2/sdlc-common lifecycle states
match before/after cleanup. Recovery report SHA256:
`8ef8785b5d6b45f8abae927502c0b02763e742cd115f0b0a4321ecfa7bf31fa0`.
The interruption below remains historical evidence, not a successful full gate.
The final project `sdlc-qa-fleet-container-control-1436cae1a772` exits0 and cleans
its own containers/networks; independent exact-project inventories are empty.
All288 frozen source inputs match the working bytes. Evidence SHA256:

- Source manifest: `8c6ccacfaf750d63e650aeaa2e3854d0e4d7f98b60dc2a1c0b24b64d7ed1b900`.
- Full log: `972e72354818faaa52e8df41973b257006033462a6baa921691739de32b45978`.
- Final report: `903199385c57d7efa7aa78f7caf995bc855cb2ba084e094af0d5e8e2abfefac0`.

Published integration commit `f0d2839` preserves all eight changed backend inputs
byte-for-byte against the tested manifest. The all-input Git comparison has281
raw-exact blobs and seven pre-existing checkout CRLF-only differences: Cargo.lock,
the two backend Dockerfiles, dev users.sql, two .gitkeep files, and Base Auth's
0001_users_sessions.sql. Normalizing only CRLF to LF proves those seven equal;
there are no other differences. The502-case gate uses its frozen working bytes,
not an assertion that all288 release blobs were byte-identical.

Agent project validation rejects sdlc-common/demo/build and invalid QA names
before intent/create/launch. Java with Docker configuration returns the existing
Unavailable503 error before files/process/DB effects; native Java remains legacy
only with no Docker configuration. No public501/not_implemented DTO is introduced.
These are Rust/PostgreSQL and fake-Base client tests, not real Docker/Hermes,
loaded config, PM or SDLC acceptance. Do not reuse494-case evidence for these
changed bytes or declare full merge readiness from this component gate.

## History-Ordinal Restart Candidate: 6 October 2026

Unpublished follow-up prepares a different generation after confirmed original
namespace exit using an immutable DB-history ordinal. Old intent/prepared files
are preserved; unknown preparation/start cannot advance the ordinal. Fresh
Rust1.88/PG project3b96063dfdea completes fmt, locked all-target check/strict
Clippy and174 infra unit cases with1 ignored. The new regression
`container_restart_requires_original_exit_and_preserves_previous_generation`
passes with PostgreSQL and fake Base; it is not actual Docker/Hermes acceptance.

The full workspace gate did not complete. Host C reached zero free bytes;
Docker observations stalled and the log stopped during a later dispatch test.
Own CLI processes were interrupted, driver exit1. Cleanup's Engine identity
read timed out after90 seconds, leaving `cleanup-required`; final ps stalled
and was interrupted. No cleanup/absence/whole-suite success is claimed. Resolve
only exact project `sdlc-qa-fleet-container-control-3b96063dfdea` using its private
Compose metadata after Docker recovery, before any new QA. Do not restart or
prune accepted workspace resources. The local driver now bounds final read-only
observations and records unknown, not false emptiness, on observation failure.

All288 frozen source inputs were independently revalidated after interruption.
Source manifest SHA256:
`d633aed1470d9e4b370f5db85011d76954927a023129f782047fab8866589824`.
Partial log SHA256:
`2fe96231726ff11f6dce3e10185fa9262dc9a66ba4a784f726459037d40216e3`.
The post-interruption record is separate from the missing original final report.
The previous published494-case gate remains valid only for its earlier source.

Independent Base native project5feb2f40bc15 failed its existing30-second
first-stopped HTTP probe before reaching replacement. Exact cleanup exit0,
empty ps/networks and independent ps were verified. Report SHA256:
`6b0e5ddde773b51212bf47b21ee7c54d13916f59a661d84d2576e3fb064363a1`.
The QA follow-up probes Base's sealed RFC1918 endpoint rather than unbounded DNS
resolution; separate DNS-isolation assertions and all original deadlines remain.
Its final status is assigned only after replacement checks. Those changes need
a fresh native gate; the observed timeout's cause is not claimed proven.
Base scoped runtime92 cases pass91/skip1. These results do not close bridge
attachment, daemon paths, file access, config drain/rollback or full SDLC gates.

## Automatic Container Preparation: 6 October 2026

Fresh project `sdlc-qa-fleet-container-control-4f2035449da9` passes Rust1.88
fmt, locked/offline all-target workspace check, strict all-target Clippy and
`cargo test --workspace -- --nocapture --test-threads=1`:494 passed,0 failed,
29 ignored. All288 frozen inputs still match the integration worktree and the
exact pinned Base SDKcbb4e99 checkout, not the newer Base runtime utility branch.
Explicit disposable PostgreSQL exercises normal repository/HTTP fixtures.
Ignored native/producers, dedicated large-keyset, central-profile/directory,
migration-lineage and message-order profiles are not certified by this run.

The four additional tests cover container listener/path rendering, strict
prepared-policy readback, automatic creation with a DB binding before unknown
start, and unknown preparation recovery with changed-credential rejection.
The last two use a fake Base subprocess and actual PostgreSQL; they are not
Docker/Hermes acceptance. A private intent pins original generation, credentials,
paths, process and source before Base prepare. Changed input holds; no native
fallback, unknown-start retry or agent-ID substitution is introduced.

| Evidence | SHA256 |
| --- | --- |
| frozen source manifest | `362ed864a55c8db9006e9e0c216195bde79c63b6624c11812250825a8a7d67ce` |
| full workspace log | `5efc9bb6cef73689f39e572f04a8ad34712146321086929866d427a63991c1bb` |
| full workspace report | `0f654631eb3de7ec9ea0d43150e1fe8482965f0f71436c19ad4b281f2c20c2af` |

The preliminary project952029dc5d9b remains FAILED: a test-fixture byte-budget
expression used Python-style exponentiation in Rust and failed compilation.
Only that expression was corrected to integer multiplication; production guards,
deadlines and assertions were not relaxed. Both exact projects are cleaned;
independent Compose ps is empty. No UI/screenshot/browser evidence is added.

Base [PR150](https://github.com/FerrPOINT/services-base/pull/150), exact head
`1d191055bb88eae4cacc8aeb6642e1a3882f90d6`, is independently ready/CLEAN with
[nine successful CI jobs](https://github.com/FerrPOINT/services-base/actions/runs/37432291311).
Its native automatic preparation gate uses projectf419b751ba90: two isolated
synthetic HTTP agents, cross-token denial, original endpoint/start/stop/sibling
checks, plus actual controller exits before and after create. Before-create
recovery holds; after-create recovery registers the same never-started container.
Both retain the original claim/key and never execute the agent or repeat create.
All six executable fingerprints match the published raw Git blobs; the report's
base_head5c06b7a records its precommit baseline, not the final certified source.
Native v1 bootstrap project77631c0e9015 separately verifies unchanged launch
and start-crash behavior with all five source fingerprints matching final bytes.

| Independent Base utility report | SHA256 |
| --- | --- |
| native automatic preparation and creation-controller faults | `868628d2a799b73c78c143b5e9daf8d2407617ef8e209e7f8a37127597088c49` |
| native v1 bootstrap and startup-controller faults | `c48b86dd32b981c16250e0cfacb70052463bf285150e106a584602b854b70f1e` |

Base host329 cases pass318/skip11; scoped runtime92 pass91/skip1. Exact owned
cleanup is verified. These utility gates do not prove Fleet Rust plus real Hermes,
controller bridge attachment, daemon path mapping, UID/file access, Docker logs,
new-generation restart, loaded-config attestation or config drain/rollback.
Task admission/first step, PM tools/resume, Forge receipts and seven-agent
deployment acceptance remain required. No merge, install, public API/schema,
SDK/image pin or accepted runtime change belongs to this packet.

## Historical Full Workspace Consumer Gate: 6 October 2026

Fresh project `sdlc-qa-fleet-container-control-b1b8968dd75f` passes Rust1.88
fmt, locked/offline all-target workspace check, strict all-target Clippy and
`cargo test --workspace -- --nocapture --test-threads=1`:490 passed,0 failed,
29 ignored. The explicit disposable PostgreSQL URL exercises normal repository
and HTTP integration fixtures. Ignored native/producers, dedicated large-keyset,
central-profile/directory and migration-lineage/message-order profiles were not
run; this is not those profiles' acceptance or a Docker/Hermes/PM live gate.
All288 frozen inputs match final source bytes; SDKcbb4e99 and installed resources
are unchanged. The earlier narrow component packet below uses the same manifest.

| Evidence | SHA256 |
| --- | --- |
| frozen source manifest | `297e6ed8d066bbde379400e68248a32362e991792f0e7ef4a63a9d15c2d19913` |
| full workspace log | `bb074f3f8d68b3f5d8181b27530fad6f464a51054e2a8bef0b454e8cd26f76d2` |
| full workspace report | `d5f061758ec2afb74d7e2f2d95af11c92e3f2fed8d3986117563f8091d3e1d31` |

Terminal exact-identity regression passes both in this full run and separately
in project366e7cf4d4b3, isolated log
`7e0f06b77d8c5518acb3c73bc514027a31b1d4b524d720be703a32098892bb2f`.
The former b2e5493b7087 attempt remains failed:1200-second driver timeout and
a terminal failure without a final panic summary. These later passes do not
prove the old failure's cause. Only the QA driver budget/temporary DB capacity
increased; no production timer, assertion, guard or Rust source changed.
Finally cleanup and independent exact Compose ps are empty. Current117 Markdown
files pass link checks; no UI/screenshot change or new browser evidence is claimed.

## Opt-In Container Supervisor: 6 October 2026

The integration candidate connects Docker start/stop/health, launch generation
and every Hermes HTTP endpoint to the private Base protocol. A controller-only
prepared generation binds original container, paths, effective config revision,
Docker context and source hashes. Fleet commits the immutable launch before the
single start effect. Unknown acceptance cannot repeat start or fall back to
native Hermes. Stop closes the launch only on original namespace-exit evidence.
This is not yet automatic creation/provisioning or installed enablement.

Fresh Linux/PG project `sdlc-qa-fleet-container-control-9276912e36c3` passes
fmt, locked/offline all-target workspace check and strict Clippy, eleven
container tests and eighteen original launch-journal tests. PostgreSQL fixtures
ran with an explicit disposable database, not their no-database skip path.
The remaining infra tests were filtered in this narrow run; the full workspace
run above is separate evidence, not inferred from these selectors.
Seven container tests validate protocol shapes/hashes; four exercise the actual
supervisor/repository against a fake Base subprocess, not Docker/Hermes.

| Evidence | SHA256 |
| --- | --- |
| frozen 288-file source manifest | `297e6ed8d066bbde379400e68248a32362e991792f0e7ef4a63a9d15c2d19913` |
| gate log | `e890d03013f8c2e688ba4348270e7e0cd7794b5f769672aa67b1e326dd9052f7` |
| report | `44b40dabc7d297f3b016a5e574e7b90b77211a0ed97f5687ed96669c97ccf2be` |

Fresh target compilation used Rust1.88 and unchanged SDKcbb4e99. Finally cleanup
removed only this project's resources; independent exact Compose ps is empty.
Earlier compile/fixture failures are retained as failed evidence: b43e0d47f3df
required an explicit tuple type; 4a5845ebf819 and7471b9e32830 exposed the test
Python fixture's incorrect `Path.parent()` call, fixed to `Path.parent`.
No production guard or deadline was relaxed to obtain a passing result.

The broader workspace attempt b2e5493b7087 did not pass: its QA driver hit
1200 seconds before the suite finished and recorded a terminal integration
test failure without the final panic summary. The driver's first cleanup
observation also found a remaining one-off; subsequent independent exact
Compose/engine container and network queries were empty. Retain this failed
report rather than reclassifying it as success. The isolated uncaptured terminal
test and complete broader run are recorded above; the product's live acceptance
and ordered release gates remain incomplete.

Base PR150 is independently ready/CLEAN at `4cfdfa9e45216c6c499580541eb8c3d1bb37ccc8`
with nine successful CI37420102732 jobs. Its endpoint readback requires original
running ACK, exact bridge/EndpointID and private IPv4, then a second readback.
Synthetic native HTTP utility tests are not this Rust consumer's live acceptance.
No main merge, SDK/image pin, public API/schema, UI or installed runtime change.
Compose rendering/create, controller bridge attachment, daemon path mapping,
logs, new-generation restart, config activation/rollback and loaded-generation
attestation remain. Task admission/first step, PM tools/resume, Forge receipts
and seven-agent deployment acceptance are still required for the full objective.

## Historical Private Container Client

Container architecture is fixed: one agent/container in existing sdlc1/sdlc2,
without a new user-facing controller service. The new private Rust client
consumes Base's register/start/observe/stop stdin/stdout protocol; it is not
yet routed from LocalRuntimeSupervisor. Buttons still use native spawn. See
[contract and remaining wiring](contracts/CONTAINER_CONTROL_V1.md).

Fresh Linux Rust1.88 project `sdlc-qa-fleet-container-control-700d2eab3989`
passes fmt, locked/offline workspace all-target check, strict all-target Clippy
and six client units. The159 other infra units are filtered, not rerun here.
The units cover original/foreign/held receipts, never-started versus ACK,
closed required snapshot fields, original hashes/network versions, pinned
configuration and bounded output. They do not execute the Rust client against
Docker or prove supervisor lifecycle.286 frozen source files match final bytes;
the target directory is new, not a previous compiled binary. SDKcbb4e99 is unchanged.

| Evidence | SHA256 |
| --- | --- |
| container_control.rs | `1a121eeb773c0de287e1cef09329bbe31b3a4c02dc2e7d1507c53102e738dd97` |
| runtime/mod.rs | `2ca797ba04de966b9ce6f21e45319d273eb6131c2b03aafd27fcda6dd285806d` |
| frozen source manifest | `5950e570436c52bdd52b98817fb50a5984b755c2690acc7f98d4859b6d964d8c` |
| final gate log | `3a96627a5e200b934f00052fe75828d936bcdaa39a26bcc0f81effeb2bde3e1c` |
| final report | `fac595659e4e0a4c2f8209c54bdcadbba487d9cfd41f9c867e9db7e2d4c7ca46` |

Own finally cleanup and independent exact Compose ps are empty. Preliminary
driver failures remain recorded: d429c2af091a generated CRLF in gate.sh;
9b4fe849eed1 omitted source include files needed by native test compilation.
The final driver uses LF and all required input files; no production assertion,
Cargo target freshness or command deadline is weakened.

Independent Base PR150 source57d717f has nine successful CI37417100487 jobs,
ready/CLEAN, no reviews/threads and no merge/install. Its native two-container
protocol and v1 startup/crash gates pass with matching source fingerprints.
Those synthetic HTTP peers do not certify this Rust client or actual Hermes.
No schema/public API/SDK/image pin, UI or installed runtime is changed. Compose
provisioning, DB-before-start binding, trusted network access, stop/health/logs,
drain/rollback, assignment admission, PM resume and full SDLC remain incomplete.

## Historical Native Publication 2293862

Final publication update: integration source
`22938621568e4ab3d812fb506226551fe506d9bd` is pushed normally with clean worktree;
47 runtime/test/harness fingerprints are checked against staged Git bytes.
Base release additionally preserves accepted maina3d6a79 at
`d2c8ef60ec4b9204d7c8232888b81022315c14b3`, all nine CI37409952184 jobs successful,
local281 host cases270 PASS/11 skips and README/hub/mirror manifest PASS.
PR144 is CLEAN/MERGEABLE without human review or merge. Its unchanged boundary
source does not certify the newer Fleet integration or PM/SDLC flow; a0f7044
release observations below remain dated history.

## Original Launch Liveness And Health Observation: 6 October 2026

The integration follow-up atop `4d4307d9e92ac9c05f1ea11548d090b0b2c2a504`
closes three confirmed defects: a retained exited child with a cached PID cannot
authorize generation/dispatch; foreign-controller health cannot overwrite the
original agent/runtime/launch; HTTP observation cannot publish a nonpersisted
health-transition alert. The request audit retains observed/persisted statuses.
No schema, SDK, UI, installed runtime or feature flag changes are made.

Actual PostgreSQL/native-child counterfactual project9302027c3f3f first fails
both custody/liveness assertions (16 PASS/2 FAIL,4.40s), log
`e4869e3307e1ed162914794319e167b01e03d0afd10e7008d412c888fd8adffc`.
The separate HTTP-route regression0b0ed1592c97 first fails on a false agent-down
alert (17 PASS/1 FAIL,8.40s), log
`fcf1c8fb7006187c9a9c3c36296322e273e3f58a9a9568776bfd211a1c09cba5`.
These are retained regression evidence, not acceptance.

Final isolated Linux/PostgreSQL projectcfceb2ce2f86 passes Rust1.88 fmt,
locked/offline all-target check, strict all-target Clippy and byte-exact
generated OpenAPI. It executes229 distinct cases: API44, infra158, original
dispatch journal21 and prepared HTTP/recovery6; synthetic renderer export1 is
ignored. Infra duration47.26s, journal10.66s, recovery18.34s. Log SHA256
`f87e71e7f5236535e5c3e51c402b2a118482e7d3c8d9a10aee831928086fd747`.
Finally cleanup and independent exact Compose ps confirm an empty project.
This focused gate does not rerun the entire foundation/migration/central-profile
suite or native control/approval selectors.

Two preceding broad runs4e7b13e83f53 and3933dc0bfdb0 fail the existing5-second
activation-file setup wait. They remain failures, not green acceptance:
logs `d11edf96ac50a242cb2612b71bd06496a5d3754e403eef6dac0278c7305d34e6`
and `d702303ffbe78e19a4d6e28904456a179584d6d4b9d1cf956695b8a01a78d98a`.
The isolated unchanged assertion passes in1.81s; the final suite preserves the
deadline and lock assertions, adds exact claimed-agent/revision checks and safe
PostgreSQL wait-state diagnostics. The earlier timing variability is not
silently classified as a product fix or stable repeated acceptance.

Native QA now refuses an existing compiled target and uses a fresh owned-project
directory. This exposed the Swagger build script's hidden network dependency:
project42dfb94c9b80 fails before any native scenario. Matching5.17.14 archive
bytes are now copied/readback-checked against SHA256
`481244d0812097b11fbaeef79f71d942b171617f9c9f9514e63acbe13e71ccdc` and consumed
locally; the internal network is not opened. Host harness tests pass38 cases.
Interrupted5aaf3276d125/bb8f0de9ad6c have no final acceptance; their exact owned
resources were independently inspected and cleaned before new attempts.

Renewed actual native projectccce318d5021 passes the two-HOME/SOUL/model,
cross-token, prepared-claim/original-controller recovery, idempotent messages,
terminal mirrors, restart-readback and tracked-parent stop case in59.28s.
The build passes fmt/all-target check/strict native-test Clippy; compiler JSON
has `fresh=false` at its unique target directory. Native binary SHA256
`f495b4bb62c870f94ab78b284b274d42b2665694abc45a91b14a24d2a6b2071e`,
log `cc0f91dbd38ed91c35f6a8e4dd536efdd8af5d016e4eae95aa8ba12430132eff`,
report `9a94d630575d878fb60f2f31e6335707f7289dbe1a4d26ea3f3f5fd7354493da`.
All34 runtime/six native-test/seven harness fingerprints match current bytes;
13770 Hermes bbaf7af files, accepted SDKcbb4e99 and Base launcherbe04b61 are
verified. Model inference is deterministic loopback, not an actual PM flow.
Exact finally cleanup0, independent empty ps and both temporary tag removals
are verified. No Hermes SQLite is edited by Fleet.

README/all115 Markdown files and existing135 screenshot/nine controller/three
control fixture hashes verify, without new UI/browser capture or live-PM claims.
Post-cleanup Docker audit checks desktop36 and sdlc2-runner0 with no violations,
but sdlc1-runner is unavailable: complete=false, exit1, not global acceptance.
Accepted images/HOME/volumes/pins/secrets/flags and other repositories are unchanged.

Independent release evidence is separate: Fleet PR47 head5f20540 passes all5
jobs in CI37403348790 and remains Draft; Base PR144 heada0f7044 passes all9 in
CI37406268902, ready for review but not approved/merged. They preserve accepted
main changes and do not certify this newer integration packet. Older release
head/conflict observations below are historical. Base host-boundary consumption,
loaded generation/descendants, Tracker/Workflow first-step admission, PM tools/
resume, ordered runtime releases and seven-agent Forge/deployment remain open.

## Prepared Original-Controller Recovery: 6 October 2026

Source packet atop `f6e856cb7fd054227abf8877ad20cf2ecf8103dd` fixes two
managed free-chat recovery defects without API/schema/SDK/flag changes:
pre-submission errors no longer terminally fail a pending message with a saved
prepared intent, and fresh native facts are compared with the private Fleet
generation only after original retained-child custody verification. Claim and
sender still recheck that generation. Already submitted/unknown acceptance never
gets another permit; historical failed messages are not reopened.

The new actual PostgreSQL regression first fails with Failed vs Pending in0.46s,
project `sdlc-qa-fleet-prepared-launch-1cc7e7dc6762`, log SHA256
`8d93c80198f7aa8fffada01b57db98829349914cb9970548a4ee719165991b21`.
It is retained failure evidence, not acceptance. Preliminary focused project
`sdlc-qa-fleet-prepared-launch-c348e038b013` passes Rust1.88 fmt/check/strict
Clippy/OpenAPI equality and16 launch,20 journal and6 prepared HTTP/PG cases;
log `19fef67277ac7aa909095601457b01ff1198a835f770b07f57a4d25a0062254c`.
The subsequent historical-failed guard has its own regression in the final
full suite. Both preliminary projects clean up with independent exact ps empty.
Final Linux/PostgreSQL project `sdlc-qa-fleet-prepared-launch-142ca30faa65`
passes Rust1.88 fmt, locked/offline all-target check, strict all-target Clippy
and generated OpenAPI byte equality. It executes469 distinct component cases:
API44, app21, domain26, infra156, managed-settings1, foundation206, shared14
and migration-registry1. Foundation duration843.09s; both new delivery
regressions execute against PostgreSQL. Cargo reports477 passes, but eight
migration tests return early without their dedicated DB and are not acceptance.
Twenty-nine opt-ins are ignored. No clean CLI/up/down, populated historical
upgrade or central-profile gate is rerun here; their previous final-source
evidence remains separate. Log SHA256
`62be9d9a6e7e315cdafa8247aae4320f81e7719bb81dd359265324d2e2b0ee89`.
Finally cleanup and independent exact Compose ps confirm an empty project.

Renewed actual native lifecycle project `sdlc-qa-fleet-native-6886dc92f33e`
passes in81.44s. A disposable Fleet PostgreSQL trigger refuses the first chat's
submission claim after intent commit. Pending/error, no submitted timestamp or
native ID and no model request are observed. A second controller leaves the
same prepared request held. Removing only the QA fault allows the original
child-owning controller to submit with unchanged run/key/bytes/hash/origin/
credential/capabilities/deadline and launch/PID. The existing two-HOME/SOUL/cwd/
ports, cross-token denial, single inference/run/mirror, native restart history
and tracked-parent stop assertions also pass. No Hermes SQLite is edited.
Log SHA256 `31ee536b9319e8e333124a4dbf2bccb00865c714a2b47d672cf8e6ec5238195d`;
report `01a40269c748490b4d44c26531f65fc021f1d5f614059ee1c0f0bdee86e3d5aa`.

The separate native lost-ACK regression project
`sdlc-qa-fleet-native-00eea27e5d7a` passes in26.80s on the same binary SHA256
`728ed4e34c5b172965e28d8637a5bbd7f94935190353b411a3588c5b78ac5a6a`.
Two actual Fleet processes and one surviving Hermes gateway recover the already
terminal run by original-key GET: one observed native POST/inference/mirror,
unchanged dispatch journal and no SSE or redispatch. The four committed Base
recovery files and13770 pinned bbaf7af source files are verified before execution.
Log `11b9f24fed1a409cbe748992f90e4e85b918b9989c9124bb4b231b645efca05e`;
report `899570f2c3e5c300b01719ea2c5dc413218c6c778164e5abc04360c966530281`.

Both cases use accepted SDKcbb4e99, launcher Base5b7c569 and deterministic
loopback inference only. All33 runtime and six test-source fingerprints match
current source bytes. Both own projects have cleanup0, independent exact ps
empty and their temporary image tags removed. Full control/approval selectors,
frontend/browser/screenshot captures and migration upgrade gates are not rerun
by these two native scenarios; their previous evidence remains separate.
Host harness units pass36 cases. README and all115 Markdown files validate;
the existing135 screenshot manifest, nine controller fixtures and three composer
fixtures verify without recapture or a claim of live PM acceptance.

This is original-controller prepared recovery and submitted-terminal GET
recovery, not managed custody transfer after Fleet death, loaded-generation/
host-container/descendant proof or task/PM admission. Existing Fleet PR47 still
has one new000010 migration and current main conflicts; its old green CI does
not certify this source packet. Ordered release/exact-head CI/reviews,
compatible Tracker/Workflow producers, PM tools/resume and seven-agent Forge/
deployment acceptance remain open. No accepted runtime resources are changed.

## Main Reconciliation And First Prompt: 6 October 2026

Candidate integrates Fleet `6135afd027f58b25b778742859822c8f7996972c`
and accepted main `3c6b8ef7bdb08f799ca30e0a5d7537914ce40ab6` by a normal
merge, not history rewriting. The accepted SDK is
`cbb4e99230420dc2659431b1c9fb5090e5c940f0`. Verified current names and
central-profile role/identity preservation coexist with Fleet's verified human
and central-subject markers. All14 historical migration files match main;
canonical18 and split21 registries append the same8 runtime releases. Nine
lineage cases (eight actual PostgreSQL) exercise populated upgrades, ledger/data
preservation, mixed/unknown denial and latest-step down/reapply. Three actual
profile cases verify no identity/role transfer or unnecessary profile write.

Managed outbox selection now excludes another controller's open launch and a
still-claimed/non-ready launch before mutating delivery. The deterministic
PostgreSQL case verifies unchanged pending snapshots, owner-only concurrent
claim and no second claim. It does not adopt a child from a persisted PID.
Final Linux/PostgreSQL project `sdlc-qa-fleet-main-lineage-f53eede92e82`
passes Rust1.88 locked/offline all-target check and strict Clippy, final fmt
check, generated OpenAPI byte equality and467 distinct component cases:
API44, app21, domain26, infra156, managed-settings1, foundation204, shared14
and migration-registry1. Foundation duration725.69s. The focused owner-queue
case is a duplicate, not added to that total. Eight actual populated lineage
cases and three central-profile cases execute separately; the registry case
in the focused lineage suite is already counted above. Historical central-
subject/email coexistence also executes against PostgreSQL in0.44s.

The same project verifies clean CLI up/status/latest-down/reapply/status in
one separate empty database and actual000017 legacy-preserving upgrade,
nonempty downgrade denial and empty down/reapply in another (0.87s).
Seven other broad migration cases return without their dedicated variables
and are not upgrade evidence. Twenty-nine broad-suite opt-ins are ignored;
the eight lineage and three profile opt-ins are explicitly run above.
Terminal exit0 and finally cleanup0; independent exact-project ps is empty.
Full log SHA256
`7eb0153afe22da10cfbab981dd641b598e4722c6286e45adff8f2b6048ec031b`.

Native project `sdlc-qa-fleet-native-3181fe8bcf28` passes actual two-home
Hermes API/AIAgent lifecycle in65.74s, binary SHA256
`2f9b37ca82d475a087bd5a1ba7edf56b83582d0b7e85a86a530ecdbea4fff0bf`.
It verifies13770 pinned bbaf7af source files, accepted SDKcbb4e99 and launcher
Base5b7c569, with a deterministic loopback model only. Distinct HOME/SOUL/model/
workspace/ports, cross-token denial, idempotent messages, single terminal mirrors,
history after restart and tracked-parent stop pass. Ten other native selectors
are not rerun. Log SHA256
`b5b8bc3ebb3b32fd0af92106b7f98e38223a00cd791f41d3b0d6152ea47a3a2a`;
report `72047722c5a810167f295aa194bd4adffd06f309eeec62df682992cb1da5d88f`.
Cleanup0, independent exact-project ps empty and both temporary image tags
removed; accepted runtime/cache resources remain untouched.

Earlier nativee171afd71d94 fails in48.06s because a second controller claims
a message it does not own. Its report
`104bb11660fcd1a1734537c6416f022396354c4dd5e358036aa267eb25fec5ae`
and log `41ae841917e78fb8a7c1c0cfa869640ad86558496ef5fd883a71d029e7f2319b`
remain failure evidence. The owner-aware atomic queue selection fixes this
without weakening the retained-child/generation check. The preliminarya6cfb79
component gate then exposes a regression-test setup error: a spawn observation
correctly sets Starting, so even the owner cannot claim before readiness.
The test now explicitly asserts that hold before setting Running. Both own
projects are cleaned; no failed run is counted as acceptance.

Legacy composer now reads authoritative chat controls instead of treating an
unused placeholder as active. Pending/unknown, task gate, read-only, unreadable
controls and witnessed pending remain held; running steer requires matching
active ID and permission. Frontend source is byte-identical between integration
and the isolated QA clone. Node22.20/pnpm10.28 frozen dependencies,
OpenAPI generate/check/compatibility, typecheck/lint/semantic/format,243 tests in31
files, build,38-route UI contract, chat contract,135 historical fixture verification
and115 Markdown links pass. Full frontend log SHA256
`f07eaa16ee1c41b20860d6069412e9aa0c13966839830b1f3fed293fd44f785f`.
Earlier test-only type/mock failures are retained, not passed evidence.

All42 Chromium/Firefox/WebKit fixture cases pass in3.3m without retries;
27 live opt-ins are skipped, not live acceptance. Browser log SHA256
`3f13f89c4b0d26709ecda30df4b2939054dfed301181ef2aeebecfd3c3d97c4d`.
The corrected QA origin is localhost, matching Base SSO canonicalization.
The earlier127-origin route-loss failure and then three actual legacy-composer
failures are retained separately. The new first-prompt/reload case captures all
three viewports per browser. Its publisher checks all9 PNGs and stores3 Chromium
full-page images with route/dimensions/hash and liveAcceptance=false in the
[separate manifest](assets/design/session-composer/manifest.json), SHA256
`b2eabe3f645e1004be7d1a89f83fbcd08cc0f7e1573a0a70334549df2cb6e51d`.
Desktop/mobile images are visually inspected; own preview55173 is absent.
Historical135 screenshots are not claimed regenerated.36 native harness safety
units also pass. No SDK dependency lock, public API or historical migration drift.
All32 native runtime fingerprints and five test-source fingerprints match
the staged source bytes; all40 staged files match the checked worktree bytes.
The local native report records the pre-commit parent plus these fingerprints,
not an exact-head CI result for the subsequent source merge.

Ordered single-migration release, exact-head CI/reviews, installed rollout,
online host boundary/loaded config/safe descendants, producer admission, PM
resume and seven-agent Forge/deployment acceptance remain separate open gates.
This source merge is not a main deployment or 100% SDLC readiness.

## Pre-Spawn Launch And Dispatch Generation: 6 October 2026

Source packet atop Fleet `856f94db16b2b080d09f733e2c1413f49a0ff5c1` adds
internal additive000017, immutable pre-spawn agent/config/controller binding,
atomic gateway observation/runtime metadata, retained-child original ACK
readback and generation-pinned free-chat dispatch. The private closed
`fleet_launch` binding cannot be supplied by Hermes metadata or translated to
a replacement runtime. Late stopped/old-PID metadata cannot clear a new
outstanding launch. This is not a container/loaded-generation/admission receipt.

Final Linux project `sdlc-qa-fleet-runtime-launch-44dbd3b86e57` passes Rust1.88
fmt/check/strict all-target Clippy, generated OpenAPI byte equality and462
distinct executed component cases: API41, app21, domain26, infra155,
managed-settings1, foundation204 and shared14. Focused15 launch and three
generation-journal cases are duplicates, not added to that total. Foundation
duration712.11s. Eighteen opt-in cases are ignored; eight broad migration cases
return without their dedicated database variables and are not upgrade evidence.

The same disposable project creates two separate empty databases for clean
CLI up/status/latest-down/reapply/status and actual000017 upgrade/down/reapply,
legacy-row preservation, immutable history/PID/null-identity/delete/truncate
guards. The dedicated migration test executes in0.54s, not an early return.
Terminal process exit0 and finally cleanup0; independent exact-project ps is
empty. Full log SHA256
`126f3030ee2451aa53a2f6f08d53e0b62a90a4078466b4c24a8bfeee7756a513`.

Earlier9a99ef6d2bd3 fails10 cases at the NOT NULL capabilities column; the
atomic observation now clears it to an empty JSON object, not SQL NULL.
3da2aa9b7fab passes focused15+3 but is intentionally interrupted after finding
the late metadata race, not counted as a full PASS. f1e819319e33 fails compilation
on a missing test closure type annotation; the final source corrects it. All
three preliminary projects are cleaned and independently absent.

Native project `sdlc-qa-fleet-native-8d421ac15eb2` passes the actual two-home
Hermes API/AIAgent lifecycle test in81.97s, binary SHA256
`bb82f253e2165430ae0271979e802945537e1b72560e016abff8decff0513d6b`.
It verifies13770 pinned bbaf7af source files, SDK9408802 and launcher Base
`5b7c5693d67c80d2252aa64c0948d0445e36dc7a`; only the model is deterministic
loopback. Activation, distinct HOME/SOUL/model/workspace, cross-token denial,
idempotent prompts, single terminal mirrors, original history after restart
and tracked-parent stop pass. Ten other native selectors are not rerun here.
Native log `28694aa710479a38fdcf11b67edb9e8c6aef4f4286890aa8a099305b3f82ee9b`;
report `f0228bb7adb7a22a6d4d4c958faf793d988b2e4abfd5fe6683e3dac52bddf3a4`.
All31 recorded runtime-source and five native-test fingerprints match current
executable bytes. Own finally cleanup0, independent ps empty, temporary source/
dependency tags removed; accepted image and shared caches remain untouched.

Earlier nativea292f09460c3 fails at Compose preparation's120s deadline before
Rust compilation/native execution, despite the captured Healthy line. It is
not runtime acceptance. Its cleanup0/empty ps/tag removal are verified; report
`1f1dee45610be5020aafd41f15c7599909316b8eca0636dffa1fc4febfffa78e`,
timeout log `7c4db45922d76cf517e6b7a98151a7a7a1b06bc522f45a6fca30fccf2716d45d`.
The successful retry changes neither source, timeout nor assertions. The first
timeout is not claimed diagnosed or fixed.

| Executed source | SHA256 |
| --- | --- |
| app/runtime_launch.rs | `e6a48cecd493f062821e04a2cf131ca401ca7f2c50278f2654a8eda3187e4650` |
| infra/runtime_launches.rs | `b0d5a9a0596a35d29d504ef9708661b14e5bb1ec955caf50dba6d2e870620b25` |
| infra/runtime/launch_journal.rs | `e9d945a614f73c155470c8890edfb3d1d1443dac6237acc6dde6adbc3aa0275f` |
| infra/runtime/mod.rs | `27def64ad64050ac6bcd586775f8b1148b00991961bb2022a6272a694cd1598c` |
| infra/hermes_dispatch_journal.rs | `7be9dfb477116cafc9b1c0799086f5a06cb590f483d5b129ee0847ae7ed22bc1` |
| migration000017 | `6563cf747e4bd473bdaecff0dd8b2d94f85ceafc0a179a339c86db17982570e6` |

Run the documented `scripts/native_supervisor_live/run.py --scenario lifecycle`
with the immutable dependency image
`sha256:aeb97055b0f5aee433e29998eeafd8065b81e70d1fcb69345c520c2bfbf23777`,
the exact checkouts above and preserved caches `pm-credential-target-20261002-04c0cf46`,
`pm-base-reconcile-cargo-20261002`, `pm-tracker-lease-rustup-20261002`.
Fixture isolation, default-free-chat compatibility and a parent wait are not
host namespace quiescence, loaded-generation admission or real PM acceptance.

Windows Node22 typecheck,31 Vitest files/235 cases,36 native harness safety
units, README validator,115 Markdown links and26-entry ADR index pass. Screenshot
verifier9 units and135 existing fixture screenshots/three viewport hashes pass;
no new browser capture or live PM screenshot is claimed. CI now explicitly
creates an empty launch-upgrade database and runs that test; YAML/command
validation passes, remote exact-head CI is still pending.

Fresh remote Base PR144 remains ready/main/CLEAN with all9 checks SUCCESS on
`dd2d0755266ef9081528702767006ba08e32d841`; it is not installed, Fleet-consumed,
merged or human-approved. Tracker PR114 `8c80a41` and Workflow PR90 `e4fba60`
remain unchanged Draft producer dependencies; task dispatch stays closed.
Fetched Fleet main `3c6b8ef7bdb08f799ca30e0a5d7537914ce40ab6` includes accepted
auth/profile and split/canonical migration-lineage preservation not yet reconciled
in this integration branch. Original migration bytes and both lineage upgrade
gates must be preserved in normal ordered release, not replaced by fresh-DB PASS.
Host boundary/boot/loaded config, safe descendants, PM tools/resume, producer
admission, Forge receipts, seven-agent live acceptance and exact-head release
remain required. PR47/PR140, main, SDK9408802, accepted images/mounts/secrets/flags
and read-only sibling repositories are unchanged by this packet.

## Private Controller Activation Storage: 5 October 2026

Source packet atop Fleet `e7379489f166bd2e5ac1f83c9ae4686c480173e2` moves new
activation backups from agent config into explicit operator-owned Linux storage.
Version 2 freezes canonical agents/config paths; exact owner/mode/link guards,
exclusive reservation, bounded backups and file/directory persistence apply.
Legacy v1 documents remain untouched and block before stop/file mutation. Empty
or unsafe storage returns unavailable, preserving drain. Root bootstrap, Windows
ACLs, crash takeover and same-UID agent isolation are not implemented.

Final owned project `sdlc-qa-fleet-controller-3f4e10a0ac13` passes Rust 1.88
fmt/check/strict all-target Clippy and 444 distinct executed component cases:
API41, app21, domain26, infra140, managed-settings1, foundation201 and shared14.
This includes ten journal filesystem cases and six PostgreSQL lifecycle cases.
The focused lifecycle diagnostic also passes; duplicates are not counted.
Eighteen opt-in cases are ignored. Seven migration tests return without their
dedicated DB variables in this broad run and are not counted as migration proof.
Completion log SHA256
`bf9e4464ff8f826b7a808a79e4db6bd81ff695ddae3395eaff9e30b031101d01`.

Separate project `sdlc-qa-fleet-controller-meta-6d93c2d6e6af` uses two clean
disposable databases: migration CLI up/status/latest-down/reapply/status and
the actual time-order migration upgrade/guard-preservation regression pass.
Generated OpenAPI matches tracked bytes; fmt check passes. Log SHA256
`2e663e8ed877be536a03d82b58e80b119c020bd29efb1a6b66e10c576f2d9e35`.
No applied/historical migration source was changed.

The first gate fails because the clock regression tests latest-migration down
instead of the named time-order migration after newer migrations were added.
The test now calls that registered migration directly and retains its nonempty
downgrade denial and unchanged-history assertions. Failure log SHA256
`8a9f212ee24a4311634704cc82519e48bb8b8aed3601b51d6f5cb55f02305a79`.
A second gate times out at the lifecycle SOUL observation. Early worker-exit
diagnostics were added without extending its five-second timeout or weakening
lock assertions. Focused and full final runs pass; the earlier timeout is not
claimed diagnosed or fixed. Retained failure log SHA256
`d3050cea4d108381ec51f935053210639dac100016cf08db159356f2b8566509`.

Native project `sdlc-qa-fleet-native-dab82533251d` passes one real two-home
Hermes lifecycle/restart test in 54.87s on binary SHA256
`5c64650c7f5818457155d94edaece238c2fa42b04c45da8e28c59bdee377d964`.
It verifies all13770 pinned bbaf7af native files, Base SDK9408802 and launcher
source e6dcb3c, actual activation/dispatch/mirrors, cross-token denial and
restart history. The model is deterministic loopback. Ten other native selectors
were not rerun for this packet; their fixture helpers now provision private
controller storage, without changing producer/control behavior. Native log
`6ebf1f91013065b98664c6fd09c6269c341ee8096e30de24a731e3be57429b3f`;
report `46d13a41b910cc104dbdb1b23f7d5d9f8ad6b4b68b35e561d7646aa573f60949`.

Executed source SHA256: journal
`37ab380f884c542f448434add57c654e1382bbdaf9e4945328102019318327f5`;
supervisor `6013f2599bed7dc9550d7d0d0244c08f8e9e9146e23e68f30da41d5f0f8d23b8`;
shared config `451873ae2c4853da21160572b33915c46623f4e2669b289c04404ffb669c72b7`;
native fixture `d1c7b3a659b72c6133c38c376db507dcae9ed5eceb190ca0ee59998fb229a897`.
All owned Compose cleanups pass with independently empty ps. Disposable native
tags are gone; shared caches, accepted images, mounts, secrets and pins remain.
README/links,36 native harness units and135 existing screenshot hashes pass;
no UI/browser capture or live-provider/PM/SDLC acceptance is claimed.
Per-agent OS boundaries, safe descendants, loaded generation, full native matrix,
producer admission, PM tools/resume, Forge integration and ordered exact-head
release/CI remain gates. Integration-branch evidence does not update PR47/PR140.
Fresh workspace Docker-group audit is complete: desktop60, sdlc1-runner0 and
sdlc2-runner0, with no violations. This checks resource grouping, not SDLC success.

## Combined Native Recovery Extensions (5 October 2026)

Baseline Fleet d310b430ce6f73a57591ba1b75958dc5982fe343 is fetched and matches
its remote integration branch. This packet changes tests/harness/docs only,
not runtime code, public API, migrations, Base SDK9408802 or installed flags.
Base cee96da617492e943ce1eb9b51f34b075f30c66d supplies the exact committed
launcher and both four-file plugins. Producer/launcher bytes are unchanged.
PR47/main remains Draft5240107, dependency PR140/main remains177edb8; their
older CI does not certify this integration head. Read-only producer PR1148c80a41
and PR90e4fba60 remain unchanged and incompatible with full admission.

Both final native cases pass on binary SHA256
`24fe62959ee049e5e79ab64aadb2005ee261d71869c29a99b7e7e4e0d0248ecc`:

| Scenario | Owned project suffix | Duration | Native log SHA256 |
| --- | --- | --- | --- |
| `combined-controls` | `e103cf635886` | 43.63s | `43c78fb868bf249d58cbbd928235a6018b5afebf75debbe0d17e9d12f782eb23` |
| `combined-recovery` | `cf05be15f9cb` | 41.14s | `bb0a468123a713ad397c640486210708fd22f7fd7183e023a123b0e17e5565f3` |

Each verifies13770 exact Hermes source files plus both committed four-file plugin
inventories before execution. The real native initial202 transport is lost; the
same original key/raw request hash recovers the native run through read-only
`POST /fleet/v1/recovery/lookup`, never a second `POST /v1/runs`. Subsequent real
steer/stop or approval ACK loss recovers through `GET /fleet/v1/controls/lookup`.
Read-only run lookup is POST; command outcome lookup is GET. No synthetic Hermes
handler, approval request or witness substitutes for either. Only the model is
loopback. Three distinct Fleet OS processes and two verified SIGKILLs preserve
one actual gateway, original dispatch/context and terminal history. One native
run/control or decision POST/ACK/audit and one inference/final assistant where
applicable are asserted; controls never become false assistant completion.

Controls require accepted/pinned local Running and actual inference observation
before steer. Their run-observation root is separate from the held control GET
root. Preliminary projectaa8422e1962f FAILED103.20s because the new combined
test accidentally used one `hold-lookup` marker for both independent recovery
channels. Initial run recovery was blocked before steer. The correction separates
the QA roots; no production behavior, deadline or Running assertion is weakened.
Failed native log SHA256:
`eebc14b753cb25d3380a14aee812a86001e61d5ef82ebc889c2d9fa40ea4eb4c`.
Its exact finally cleanup0 and independently empty Compose ps are verified.
An earlier preliminary combined-approval pass used different source and is not
counted as final-source acceptance.

Final focused project5981f85bb9c9 PASS: Rust fmt/all-target check and10 HTTP/PG
consumer cases72.98s; one separate keyset target is ignored, not accepted here.
Log SHA256 `278fc3dc18daa4e91273dcb57f4747f84a2295b6ee744e35d67ac75c6b1483f5`.
This is not a rerun or recount of the previously published450-case broad gate.
Both final native builds pass fmt/all-target check/strict native-target Clippy.
Reports are verified against current raw runtime/test/harness bytes, exact native
log hashes and the same binary. Own cleanup0, independent empty Compose ps and
both own image aliases removed are verified; shared caches and accepted images,
data, runtime HOME and pins remain intact.

Three separate final-source regressions also pass on that same binary, each with
raw source/log verification, cleanup0, independent empty Compose ps and removed
own aliases:

| Scenario | Owned project suffix | Duration | Native log SHA256 |
| --- | --- | --- | --- |
| `control-restart` | `aa330833f0aa` | 39.44s | `e7880dcd60a5f6a2a25b7b8455743c322d372934d4ef8c74d323d2f4689e2bbd` |
| `approval-restart` | `18ac1f602aab` | 34.71s | `415891ed03334d19a798ee101de61a5e8a9a1153f1bc1d6f68e99a2498607cc6` |
| `recovery` | `fbac54196071` | 22.41s | `45ebdf5641fc12486ef0538b25340d02b635bb4f95e46237ed3fca93a84b99b4` |

Host36 harness/selector/plugin-matrix cases,3 README cases and README validation
pass. Markdown links cover113 documents including the new proposal. Existing135
three-viewport screenshot hashes and9/3 fixture manifests pass; there are no new
UI captures. Post-cleanup Docker audit checks desktop63 and sdlc2-runner0 with no
violations, but sdlc1-runner is unavailable: complete=false/exit1, not a global
Docker PASS. Own exact cleanup is verified independently. No foreign resources
are removed. The recorded VM clock discrepancy remains an infrastructure risk.

Final raw source SHA256:

| Path | SHA256 |
| --- | --- |
| `backend/infra/tests/support/native_control_restart.rs` | `e7c6bc1ae63053a8d982f926c44a6ac495d1520f6788a6ae4620e64978396c3a` |
| `backend/infra/tests/support/native_approval_restart.rs` | `487db4d98be6cf3c19146d48a9e86fd9bf55bf9f1702eca891d5a3bf95cd9f9c` |
| `scripts/native_supervisor_live/run.py` | `7ce454a6fb1402f5f66d0724f78ac1cc97b4ee39364bc8eed8292555c422cce4` |
| `scripts/native_supervisor_live/preflight.py` | `50edfacb5c35beca0856aea29f378b208de240cde744bc2192816b4a943a4657` |
| `scripts/native_supervisor_live/test_harness.py` | `e6448259d7ce5a9ade593c09ebd91147b26b6e65202fe86ca59afd79867536c8` |

This closes combined free-chat run/command plugin compatibility under these
faults, not installed rollout, safe descendants, native loaded generation,
fenced task admission, PM tools/resume, production Chats/Forge, ordered migration
release/exact-head CI or seven-agent SDLC. The
[containment proposal](design/RUNTIME_CONTAINMENT_PROPOSAL.md) is proposed only;
no boundary has been implemented by this test packet. Historical sections below
retain their earlier scope. No new UI screenshots or live browser acceptance
are claimed for this backend test-only change.

## Approval Outcomes After Fleet SIGKILL (5 October 2026)

Baseline Fleet f7bbfe61ff3978fc65ff842f8cb2a34a3dcede77 is fetched and matches
the remote integration head. This packet adds native tests/harness/evidence only;
runtime code, public API, migrations, Base SDK9408802, producer and accepted flags
are unchanged. PR47/main remains Draft5240107 and dependency PR140/main remains
177edb8. Their older green checks do not certify this integration candidate.

Canonical `approval-restart` projectea8cbb20dbb5 PASS37.97s: three distinct
Fleet OS processes, two verified SIGKILLs and one surviving actual Hermes gateway.
The owner HTTP once decision executes a real terminal chmod and loses its actual
ACK transport. A loopback model holds the second inference until both deaths;
it is not a Hermes handler/tool/approval/witness substitute. The second Fleet
process replays the same durable uncertain receipt and must perform a new
original GET, measured after the first process died. The third first mirrors
native terminal completion, then settles the decision through released original
GET. One POST/native ACK/audit, original UUID/store epoch/raw hash, same gateway,
immutable context/dispatch/terminal timestamps, one prompt/assistant/run/request,
owner replay and changed-payload409 are asserted. No destructor-only recovery
or second decision/run dispatch is accepted.

Source preflight verifies all13770 exact Hermes files and4 committed Base control
files from e514f5a6510cb98776547b58d2dfb6cffa62ee8c. Producer/launcher bytes match
the previous packet; SDK9408802 is unchanged. Native binary SHA256:
`51533c920430ead59b65976f81bbdac40363cb4a6453acd325865b630883e962`.
Native log SHA256:
`5aa8d31127d0f0772a7cf3a86ff9f60e464587d86b8b6bef995162a84b3b8499`.
Build fmt/all-target check/strict native-test Clippy PASS. Exact finally cleanup0,
independent empty Compose ps and removal of both own QA aliases are verified;
accepted images/data/HOME and shared caches remain intact.

Separate same-binary `approval-outcomes` regression projecte8979ba4373b also
PASS25.96s with exact cleanup0/independent empty ps/own aliases removed. Log SHA256:
`36d3974ec498613bacfc5055b5a8188105469672fd227309733326f05f9f6625`.

Two further same-binary regressions PASS: legacy `approvals` project7ff1fa5ffc3e
13.38s and current waiting `approval-recovery` projectfd90821eb59c31.74s.
Both have cleanup0/independent empty ps/own aliases removed. Native log SHA256:

| Scenario | SHA256 |
| --- | --- |
| `approvals` | `f0020e576ba8479b714b7f7ed63d8755e60bdb97c5f93fc7f5feace92ded69f4` |
| `approval-recovery` | `e745bf328814ef9bebbbff31e3c27a4324f84b9e0791c69dbc322df107094702` |

All four reports are checked against current raw test/runtime/harness bytes,
native log hashes and the same binary, not merely report status. Host36 harness
and3 README cases, README/112 Markdown links, existing135 three-viewport screenshot
hashes and9/3 fixture manifests PASS. No new UI captures or live browser acceptance
are claimed for this backend test-only packet. Post-cleanup Docker audit checks
desktop41/sdlc2-runner0 with no violations; sdlc1-runner is unavailable and the
global audit remains complete=false/exit1. Own exact cleanup is verified separately.

Preliminary focused projectsaf2bbb8bcb8a and a4b8a60909f0 fail compilation for a
wrong event type and ambiguous UUID type in the new helper. Corrected types retain
all assertions and deadlines. Both failed projects have finally cleanup0 and
independently empty ps. Failed log SHA256 respectively:
`280ddaec7116910e30d2e2bb13d4cc409faf1a7c53f657d439d7eb0d244871d2`,
`c8ddf6a0f42ebc776209104c45c24de2c6be41814b886d91045b90da1cf16050`.
Focused projectecfc794674ad passes10 HTTP/PG cases67.32s, then the test barrier is
strengthened to require a new GET after real process death. Final focused
project602817e2da4a passes fmt/all-target check and the same10 cases71.31s,
with cleanup0/independent empty ps. Its log SHA256:
`5ac6dfbfa8d3d110277d08987e76bbde692807d0bfa4881ead2aa16a8334138a`.
Repeated cases are not added to the previously published450-case component gate.

Final raw test/harness SHA256:

| Path | SHA256 |
| --- | --- |
| `backend/infra/tests/support/native_approval_restart.rs` | `1a34d91f0df4d76e1b73f4c0921958660aae5592a74478dfe76eab79be49d8ab` |
| `backend/infra/tests/support/native_approvals.rs` | `e432eec67924895eaeaf74ef9444194973eb9524e403f555f8e718ee86f6fb58` |
| `scripts/native_supervisor_live/run.py` | `3a0bed30b5387bc04275cb9f73de0db6762b90236979e5b1f3268a7c27df4aa4` |
| `scripts/native_supervisor_live/preflight.py` | `81140e07da9c6b65413cf9d2e6dedc98dd0ea80dbb06388fbc3f5c136d5bf7fc` |
| `scripts/native_supervisor_live/test_harness.py` | `46399820329c7287222b7154a7e9cfd6bf04ff91c2dbde14814d105759cad510` |

This closes the separate approval OS-process-death acceptance, not combined
extensions, loaded generation/safe descendants, task/PM/production Chats/Forge,
ordered release/exact-head CI or seven-agent SDLC. Orphan gateway cleanup belongs
only to the disposable Compose namespace, not a safe-stop proof. Historical
sections below retain the limits of their earlier packets.

## Native Original Approval Outcomes (5 October 2026)

Published Fleet consumer baseline
`9576201d03031b6c5ded78c965d8fea3578e5a23` is verified on its remote integration
branch. This new packet changes native tests/harness/docs only, not runtime
code, public API, migrations, SDK9408802, Base producer or accepted runtime flags.
The common approval scenario retains the legacy case and adds the distinct
`--scenario approval-outcomes` with committed Base control plugin inventory.

Canonical project `sdlc-qa-fleet-native-3eca3eb7b248` PASS25.54s: actual Hermes
terminal guard emits the request, Fleet owner JWT HTTP sends once/deny, observed
file permissions match each decision and one exact POST/native ACK/audit exists
per decision UUID. The QA-only observer closes the real lost HTTP ACK and holds
original GET. Native terminal completion leaves the receipt uncertain; releasing
the hold settles it through the original GET with no second POST, context/dispatch
change or terminal timestamp mutation. Replay returns the same decision and
changed payload conflicts. The deterministic loopback model is the only inference
fixture, not a Hermes handler, approval, native effect or witness substitute.

Preflight verifies all13770 exact Hermes source files and four committed Base
control files from a1aeaec83d92afbbd9579b932ea205b3d42dcf79. Native binary SHA256
`5a5a0c98a956e5cdde9901af3a7f2d2afa408f5747cc19c0e6661f840e8a868a`;
native log SHA256
`adf67133224aa894b812d0a174bbf0fc6a93af97cde808b93bf87d4a9f4d5084`.
Exact own cleanup exits0, independent Compose ps is empty and both own image
aliases are removed. Existing dependency image/shared caches/accepted resources
are preserved. Native build fmt/all-target check/strict native-test Clippy PASS.

An initial compile project1db270f95fa6 failed because the test attempted to clone
the mock-enabled non-Clone SeaORM connection. It was corrected to an independent
connection, with no production/assertion/deadline changes. Finally cleanup0 and
independently empty ps are verified for that failed project.
Failed compile log SHA256:
`8681fbdef494eef79043db752546d584b82c3b9c490c1c6e7bbd5782f45c127d`.
Subsequent project
59c621c85382 passes fmt/all-target check and10 HTTP/PG consumer regressions,
with exact cleanup0/independent empty ps. Its log SHA256:
`85b9eda61de969e4c9f6f4d8e07b89d64efcd954273b2ee512dbc008cfac0571`.
This is a focused regression, not a recount of the published450-case gate.

Three separate native regressions also PASS on the same binary/current source:
legacy approvals projectb497ec9b0db1 (23.46s), current waiting approval recovery
project3dfc2f186b8d (32.05s), and control SIGKILL project4ea1a3a025ef (31.95s).
The latter confirms two SIGKILLs/three Fleet PIDs for steer/stop, not approvals.
Every project has cleanup0, independently empty exact Compose ps and removed own
source/dependency aliases. Native log hashes:

| Scenario | SHA256 |
| --- | --- |
| `approvals` | `69457684eea6ffd35fc55a21f403c9a103d4a5e68b9f369af41cb6734378f656` |
| `approval-recovery` | `470e0cb46861e1aeaaecafae545d99b92cac9f086a7885e28a6ac6eb8e04329d` |
| `control-restart` | `9ad2b57066821cb84d34232beaffde51e113167129ea13670e6d39708f5629da` |

Final raw native source/harness SHA256:

| Path | SHA256 |
| --- | --- |
| `backend/infra/tests/support/native_approvals.rs` | `717990c3f18b254df1a7c6d3e8c868fe7f448e3d37f02f122b9c200756d15c82` |
| `scripts/native_supervisor_live/run.py` | `6f10f8fd91de1266bbe6238a72195e65ac58593e40f28064cbbafe518b9c1c0d` |
| `scripts/native_supervisor_live/preflight.py` | `4e486f38f5f32151be3ddd3a82a79423a863bb966af1a0f7981fff3fe823ae5d` |
| `scripts/native_supervisor_live/approval_fault_plugin.py` | `e5623b7b26b68ca20d666e5b86ff8c5ef1c1cd41195d4ccf478c819a9d62eb52` |
| `scripts/native_supervisor_live/test_harness.py` | `011b1790679a5fcdcac1bdb631a56acea9a1ce841989e80312b5654f5fe283a8` |

Host36 harness and3 README cases, README/112 Markdown links, existing135 screenshot
hashes and9/3 fixture manifests PASS. No new UI capture/browser acceptance or
rerun of the earlier450-case component gate is claimed by this test-only packet.
Post-cleanup global audit: desktop48/sdlc2-runner0 checked with no violations;
sdlc1-runner unavailable, complete=false/exit1. This is distinct from exact own
cleanup; the previously observed foreign journal violation is absent now.
Approval Fleet OS-death, combined extensions, loaded generation/safe descendants,
task/PM/production Chats/Forge and ordered exact-head release remain full-goal gates.
This case does not declare the whole SDLC merge-ready or enable installed runtime.

## Original Approval Outcome Consumer (5 October 2026)

Baseline journal packet `317c326f3323d9dac35ea808c279a74ca29f2b0e` is published
regular fast-forward and its exact remote head was verified. Main d752a041,
Draft PR47/main5240107, Base a1aeaec and dependency PR140/main177edb8 are
unchanged. Fresh Tracker PR114/main8c80a41 and Workflow PR90/mastere4fba60
remain read-only and incompatible with complete admission. Forge task2's
ahead-27 working history is preserved, not pushed or changed here.

Implemented: opt-in API fixes original mode before reservation, supervisor
verifies the exact native waiting request, commits original context/one claim,
then sends exact bytes/decision UUID/store headers once. Legacy dispatch is
disabled in original mode; preparation failure never falls back. Bounded
UUID-keyset GET recovery authenticates saved origin/scope/epoch/native run/
request/choice without new POST or capabilities. Normal/lost/DB ACK settlement
is atomic; historical cancelled requests and terminal runs remain unchanged.
Public DTOs exclude the context, mode/claim flags and private fingerprints.
No migration, generated API, SDK/lockfile, Base producer, UI or installed flag
is changed by this consumer packet.

First focused Linux Rust1.88/PG gate exec7536/project
`sdlc-qa-approval-outcome-consumer-7c852c467cb1`:10 HTTP/PG cases PASS with
workspace all-target check/format. Exact finally cleanup exits0 and independently
empty Compose ps are verified. Startup spent time in Docker network/container
creation; the original live process was observed and not replaced or duplicated.
This first snapshot predates the separate keyset case and final wire/scope guards;
it is not final-tree evidence.

Final Linux Rust1.88/PG17.6 gate exec46521/project
`sdlc-qa-approval-outcome-consumer-b5bffd65bc9c` exits0:450 distinct component PASS
(236 library,201 foundation,2 separately executed keysets,1 approval SSE,
7 isolated migrations,3 supplemental PG). Focused10/17 and lock repetitions
are not additional cases. The separate approval keyset holds101 records with
the valid UUID strictly after100 invalid scope contexts; recovery uses original
GET and zero POSTs. Synthetic renderer export remains ignored. Format/all-target
check/strict Clippy, source-generated OpenAPI byte equality, clean migration CLI
up/status/down-one/up/status on17 versions and doc-test invocations PASS.
Exact finally cleanup exits0 and independently empty Compose ps are verified;
shared caches, accepted runtime HOME/images/data and foreign resources remain.
Full log SHA256:
`771899d0115199b055ab30861662661a7d943b33a0a8a566b9457142f583ecf2`.

Host34 native-harness safety cases and3 README tests PASS; README,112 Markdown
links, CI YAML/isolated keyset target, existing135 screenshot hashes and9/3
fixture manifests PASS. No UI changes, new captures or browser acceptance are
claimed. Three20-23s VM clock regressions remain an infrastructure risk; no
assertions or timeouts were weakened. Post-cleanup Docker audit: desktop35
checked, sdlc2-runner2 checked, sdlc1-runner unavailable/complete=false/exit1.
A foreign rootless CI-QA transfer project lacks its job journal reference;
no foreign resource was removed or repaired here.

Final raw source SHA256 (paths relative to `backend/`):

| Path | SHA256 |
| --- | --- |
| `api/src/routes/approvals.rs` | `6bf07c59c53dcbf31b303dd2d1dd082daea4f5a96ae2a916d5f82e554ebf4867` |
| `app/src/lib.rs` | `02d0c38d1e2e59e70b64175d119a8f3b64334be14cbd88c36226109fc1b482b4` |
| `infra/src/runtime/approval_outcome.rs` | `8153cf4779d57c479960cad1ceca3705284f2ff10ff7d6d8dda1b00cf12e1c54` |
| `infra/src/runtime/targeted_approval.rs` | `632814be1e10922f6d696662894f9cc0d4861126538bb0eb18d41be38f56818c` |
| `infra/src/runtime/control_outcome_wire.rs` | `94f5f24174ad5d5e55efbf48d01bc6b8a1fd620b15a342a2bf1925dcabdeb52e` |
| `infra/src/runtime/control_outcome_wire_tests.rs` | `c5cd2bdf9e46c50e322b2567505f7e8444f5492db8ee3d2ae08c015d9c21805c` |
| `infra/src/runtime/mod.rs` | `1b5eda1c033e62866010bfe608d1f951b614cb7b079f112367569938d88c0ebf` |
| `infra/tests/support/runtime_control_outcome_http.rs` | `76811db66fb27b9dcbf70f808234a01974c33935686cae2502ffccad5ee924e7` |

All19 registered migration source files remain byte-identical to the published
journal baseline. No remote exact-head CI, actual native Hermes approval decision
recovery, full-SDLC or release readiness is claimed by this component packet.

## Original Approval Outcome Journal (5 October 2026)

Baseline Fleet integration HEAD04df3107c07203a7060ef2f4506428f222455f39,
remote/main d752a041 and SDK9408802 were rechecked before this packet. PR47
remains Draft/main on5240107; its existing CI is not this integration tree.
Base integration a1aeaec and dependency PR140/main177edb8 are unchanged.
Tracker PR114/main8c80a41 and Workflow PR90/mastere4fba60 are unchanged and
remain read-only; their admission/release incompatibilities stay open.

Implemented: additive000016 fixes original mode at reservation, persists exact
closed once/deny action context with one claim, and atomically commits witnessed
delivery/audit/events. Legacy uncertainty cannot be backfilled. Fresh claim
rechecks actor/owner, accepted concrete primary Hermes and active free-chat scope.
Late ACK preserves cancelled request and terminal run history. A claimed unknown
decision cannot become failed or be removed. Private context has no public DTO,
Debug, token or audit payload. Sender/GET-worker wiring is not part of this
journal packet; existing HTTP approval behavior and installed flags are unchanged.

Final Linux Rust1.88/PG17.6 gate exec41989/project
`sdlc-qa-approval-outcome-journal-6dd13c1522e2` exit0:438 distinct component PASS
(235 library,191 foundation,1 separate keyset,1 approval SSE,7 isolated migration,
3 supplemental PG). The preliminary seven approval cases and corrected lock
case repeat the foundation cases; they are not additional counts. Synthetic
renderer export remains ignored; native supervisor scenarios were not rerun or
recounted. Format/all-target check/strict Clippy, source-generated OpenAPI equality,
clean migration CLI up/status/down-one/up/status on17 versions and doc-test
invocations PASS. Empty/legacy up/down/reapply and nonempty downgrade denial are
verified for000016 in its own disposable database. Exact cleanup exits0 and a
separate Compose ps is empty; shared caches and accepted runtime resources remain.
Final full log SHA256:
`1a8d7047c0b779755dd8f9a9c7d8efc59c1e33b1e16b9e3f3259018307ec4a1f`.

Preliminary projects cc5270700154,0aeb8a5fb33b,a00ce420866f failed on a test method
name, a fixture incorrectly assuming terminal commit cancels requests, and an
omitted fixture resolve_all field. Actual repository cancellation is now exercised
separately. First full project10f3b87ccdc8 found the old lock test waiting for a run
blocker after reservation became session-first:190/191 foundation PASS, gateFAIL.
Its log SHA256 `5418099458ae0ab0ac5b2f5986eea31d516188bde76a7c770bd5d98419128889`.
The corrected test observes terminal waiting on the reservation's actual session
lock through pg_blocking_pids, with unchanged deadlines and outcome/event/replay
assertions. It passes both its separate run and the final whole foundation gate.
All four failed projects also have finally cleanup and independently empty ps;
their logs are retained locally, not represented as accepted runs.

Host:34 native-harness safety cases and3 README tests PASS; README,112 Markdown
links, CI YAML isolated migration target validation, existing135 screenshot hashes
and9/3 fixture manifests PASS. No production UI capture or browser acceptance is
claimed for this backend packet. The CI source now explicitly creates the approval
migration database and removes an incorrect backend-only directory override from
the prior outcome migration step; remote integration CI remains outstanding.
Observed20-22s VM clock regressions remain an infrastructure risk, not an excuse
for weakened assertions. Global Docker audit is recorded separately from own
cleanup and must not be described as green when a runner is unavailable.
Post-cleanup audit: desktop-linux39 and sdlc2-runner0 checked, no violations;
sdlc1-runner unavailable, complete=false/exit1. An earlier foreign rootless QA
project lacked its job journal reference; it is absent in this final inventory.
No foreign resources were removed by this packet.

Exact raw SHA256 of the final source files (paths relative to `backend/`):

| Path | SHA256 |
| --- | --- |
| `infra/src/approval_outcomes.rs` | `ba7936c4da1dbcb59df7395d18fed175e27f61a45e60e505e327779978e5b9a8` |
| `infra/tests/support/runtime_approval_outcomes.rs` | `6930e11649f9b137d8fd0d061164e70ea14b8277968443971f6e7b04cef1abea` |
| `migration/src/m20261005_000016_runtime_approval_outcomes.rs` | `511c035a267de883c61f39552b0739e884b5f3becf0a7021718d2369e6842e59` |
| `migration/tests/runtime_approval_outcomes.rs` | `3347268362b73bbfbd84e5105249155ab1b8c55f927e1df8739a353c84b77f0a` |
| `infra/tests/support/runtime_terminal.rs` | `aac094c3e40607caf50228f5c2e6aa50f1f2a5034008f5fba166a264895d3a70` |

Applied predecessor bytes, generated public API, SDK/lockfile pins, Base producer,
runtime HOME/images/data and sibling working changes are preserved. Original
approval sender/GET recovery, positive/lost-ACK/Fleet-restart native decisions,
combined extensions, loaded generation/safe descendants, fenced admission/PM/
production Chats/Forge and ordered one-migration PR release remain requirements
of the full goal. This packet does not claim whole-SDLC merge readiness.

Date: 2026-10-01. Status: verified foundation, incomplete approved vertical slice.
No real PM publication/resume or live Backlog acceptance is claimed.

## Control Outcomes After Fleet SIGKILL (5 October 2026)

Parent Fleet b525f9fac7e3fb85b2d97fdac41c280070af227e and committed Base
0b13102dec14d2447cb91f4b005b4c2d8d1a5270 identify this test/evidence-only packet.
Runtime code, public API, migrations, SDK9408802 and accepted pins are unchanged.
The new canonical `--scenario control-restart` uses three Fleet OS processes,
two SIGKILL exits checked as signal9, and one surviving real Hermes gateway.
No supervisor clone or graceful shutdown substitutes for process death.

The first process reaches an actual model-request barrier, dispatches steer,
loses the real native HTTP200 ACK, persists original context and is killed.
The second replays that key while lookup is held without sending another POST,
restores steer through GET, dispatches stop, loses its real ACK and is killed.
The third independently reads the interrupted native terminal state before
lookup is released, then restores the late stop ACK without changing terminal
receipt state/timestamp. Each command has one POST, one real ACK and one audit.
All private contexts and dispatch identity remain unchanged; the gateway PID,
native run/session and single loopback inference survive, with no false assistant.
Approvals, OS-descendant containment, central identity/task admission and complete
SDLC are not attested by this case.

Final exec88140/projectcbc280c0aa37: 1 named native PASS, 0 failures/ignored,
20.95s, with exact source preflight13770 files and committed control plugin4 files.
Binary SHA256 `dc297749ca40766f1ee0a1eaceefa9aefc3d6ccd556224902767c1171ed3ed92`.
Native log `1082669ce9b5487c090f564ea06c718444beb67f6734d94dcc450e699441d2f2`;
build log `0fab2380047cbb418eccd0c83b72337bfe6cdcb68446f5e5bfae04809b7fc020`.
New Rust test source `f4c101fc8c17a18563ac3a7390c3cffc45140b7ca04f7d426759487045ac5bb8`.
Recorded runtime/test/harness fingerprints match executed worktree bytes.
Linux Rust1.88 fmt, locked/offline workspace all-target check and strict native
test Clippy pass. Host harness34 PASS; README validator passes. Previous430
component cases are unchanged baseline evidence, not re-run or counted here.

Preliminary executions are retained, not acceptance:
- exec14269/project3e47f71f1e45 failed compile: SeaORM connection is not Clone in
  the test's feature set. A separate connection now reads the audit.
  Build log `aeb52fd1d7661968e243aeea1753d2901ced7ab6b43dfd7a4bd6c6ffb25d7125`.
- exec26773/projectb76fc7d48255 failed its inference assertion before the parent
  observed the actual model request. The explicit observed-model barrier fixes
  synchronization without changing deadlines or acceptance requirements.
  Native log `6bacc06b763d1c761bdca7954a3c9031ec8a0b85d49a98ac09199c5e430969bb`.
- exec88978/projecte43454ca4f43 timed out in Compose PG preparation even after
  healthy output; no Rust/native acceptance ran. Timeout log
  `4bea26fbe3c0b038864a489446619a3401ba84c489df925c6a34200cad48547b`.
- exec35740/projectfb6a678909bf passed its native assertions but the final gate
  rejected missing control-plugin preflight for the new scenario. The selector
  now includes both outcome scenarios and has a host regression. This run is
  not accepted. Log `bfd63c689397d237ea5456ec41df43d57b3c05e74190d497f3336759a399f30c`.

All five exact projects report cleanup0, independent ps empty and own QA image
aliases removed. Shared caches, source dependency image and accepted resources
remain. PR47/main5240107 and PR140/main177edb8 are unchanged; this is not
release-head CI or permission to enable the control plugin on installed agents.

Renewed regression exec30404 uses the same current Rust/test/harness bytes and
binarydc297749: legacy `controls` projecta589cccffb67 PASS13.43s, log
`0975849c4233a4dd08a4dd242d7dd4fd545dd2976addf3bf9b73fdbe9b4614bf`;
`control-outcomes` project7192fe5af2fa PASS25.82s, log
`f29fd74b0c8803b26864798f5119f4363a0c9869258d65d16c02370ca03c62e6`.
Both cleanup0/independent ps empty/own aliases removed. All three current native
reports and their log/test/runtime/harness hashes have been checked against
the final executed bytes. The preceding six-scenario baseline below remains
historical: the other four scenarios are not re-run in this test-only packet.
111 Markdown files, README and existing135 screenshots/9 chat fixture/3 control
fixture hashes pass; no new browser capture or frontend test/build is claimed.


## Original Control Outcome Consumer (5 October 2026)

Parent Fleet b753c5f7f137adcd612eedb5f5e02ee675d9b597, unchanged Base launcher/
control producer a48e53ee37a5a38b189f8a9cce84686d18fc5c83 and SDK9408802
identify this supervisor packet. Applied migrations000013/14/15, generated
public API and sibling services are unchanged. The startup flag defaults false;
accepted agents/images/pins have not been modified or enabled.

After authoritative human/accepted free-chat/native preflight, context and the
single-use permit commit before one saved-byte POST. Original UUID/store headers
and exact closed HTTP200 ACK are required. HTTP/DB ACK failure retains unknown
acceptance; caller replay cannot POST again. The separate five-second worker
scans UUID-keyset pages of100, checks current native pins/original credentials
and adopts only a matching saved-context GET witness. It never refreshes the
original epoch, backfills legacy commands, grants actor mutation rights or
reopens terminal execution. ACK/outcome/audit/events commit atomically. Approval
decision outcomes and fenced task/PM controls are outside this sender's scope.

Failure history, not passing evidence:

- Owned projectdbf70527ecb7 failed test compilation because receipt has no
  PartialEq. The test compares its complete serialized value without changing
  the domain type. Target projectedeab7b2a272 then passed13 wire and9 HTTP/PG cases.
- Project82053a7106fb failed two timed HTTP fixtures after the 101-record test
  left100 intentionally invalid immutable contexts in their shared database.
  The keyset case itself passed; it now has its own empty disposable DB and a
  required isolated CI invocation, without deleting history or relaxing timers.
- Projectec578ca03e00 passed targeted13 wire,9 regular HTTP and the isolated
  keyset case, but broad foundation failed1 of184 cases. The held normal ACK
  waited for global queue traversal beyond the original10s HTTP deadline.
  Keyed authenticated GET now synchronizes DB/normal-ACK settlement directly;
  the separate worker recovery assertions remain intact. Strict Clippy also
  rejected one needless format call; it is removed, not suppressed. A fmt check
  caught the newly edited test before formatting. Supplemental gates did not run.

All those owned projects reached exact finally down and independent empty ps.
Fresh supervisors/repository connections in component cases are not a Fleet
OS-process restart. The native `control-outcomes` scenario is separate: only
actual execution, matching source hashes and cleanup can certify its result.
32 host harness safety tests pass, including fault authentication, exact real
handler ordering, unknown/error preservation and observation redaction. Host
units do not constitute native runtime acceptance.

Final component exec1851/project `sdlc-qa-control-outcome-consumer-51205b478016`
exits 0. All 430 distinct cases PASS: 235 libraries, 184 foundation, 1 isolated
keyset, 1 isolated approval, 6 isolated migrations and 3 supplemental PG cases.
Targeted13 wire/9 HTTP cases were repeated by broad suites, not counted twice.
The keyset case is ignored in the shared suite but actually passes on its own
empty DB. One native renderer export remains ignored and is not counted.
Locked/offline all-target check, strict workspace Clippy, fmt, generated OpenAPI
equality and clean migration CLI up/status/down-one/up/status on16 registered
versions PASS. Doc-test commands run zero examples. Exact finally down and
independent empty ps pass; shared caches and accepted resources are preserved.
Completion/PG diagnostics SHA256:
`737cc1478b10c5631619b211c61b76a7b7d7f367ccf10b8174fd13e675c106d5`,
`0680cd2928ca80bdffeae554cf94a572ccc0e0f7874b57787e8fda30a8cfdecf`.

Node22.20 typecheck/generated-client equality, README and111 Markdown links
PASS; existing135 screen/9 chat/3 control fixture hashes verify. There is no
frontend source change, new browser capture or repeated235 Vitest/build claim
for this backend packet. VM clock regressions remain an infrastructure gap.

Native attempt `sdlc-qa-fleet-native-34150db88db4` FAILED, not acceptance: the
QA fault opt-in existed only in the parent process and was correctly removed
by Fleet's sealed child environment. Its hook therefore did not install and
the real steer ACK was not lost. The disposable config now explicitly supplies
that opt-in, and an authenticated held-lookup check proves fault installation
before any model/control action. Production environment sealing and failure
assertions are unchanged. Exact cleanup, removed own image aliases and empty
ps passed. Failed native log SHA256:
`0be1ed8cea2952e79bd3bb39ffe66c741af48dbfff4c3831c9970e1682b1249a`.
Only the native test setup changed after the 430-case gate; production/test
component source is unchanged. The renewed harness must compile/check all
targets, fmt and strict native-test Clippy against the revised setup.

Native attempt `sdlc-qa-fleet-native-5150fb2a8f50` also FAILED: the fault hook
installed, but its outer observer read the body before the control wrapper's
bounded request clone and no original ACK was recovered within the unchanged
90-second test budget. The observer now follows that wrapper and uses its
cached body; a host regression prevents inserting it before existing middleware.
Neither producer guards nor recovery criteria/timeouts were weakened. Exact
cleanup and empty ps passed. Failed native log SHA256:
`a80f7552aa1ef4ee8749e5e5f30548df7167b7304a51a75eab48f740cf5fd7db`.
This failed attempt is not native acceptance.

Renewed actual managed `control-outcomes` projectc0ddba5857b4 PASS: 1 named native
case, zero failures/ignored, real Hermes CLI/API/AIAgent and loopback model.
The fault observes the real bounded wrapper/native handler; HTTP replies are
lost, not invented. Original GET restores steer ACK and late stop ACK after a
real gateway PID restart. One POST per UUID/body hash, one inference/run,
unchanged original context/epoch and preserved terminal state/timestamp are
asserted. Legacy `controls` project69905fb0784d separately PASS with the flag
off. Both use binary SHA256
`94f5bea77e118e6f37bd3929ffb21f9c3da0101280f52a98f6c9a6ec0dc21de0`.
Hermes bbaf7af archive571fba49/13770 tracked file hashes, SDK9408802,
Base a48e53ee launcher75ad258e and complete four-file committed control plugin
inventory are verified. Every runtime/test/harness source hash is in the reports;
there is no mutable Hermes mount or accepted-image replacement.

Native completion log SHA256 (outcomes, legacy controls):
`ce9a9e0a16e47da42b82bbcd832d1182177674162da53aba5fce6e79798e6d94`,
`28fe67293dd9a422865c23571b89d2aad2dfb49bd337968f51668a1e437d1afb`.
Both exact cleanup/empty ps and removal of their own image aliases PASS; shared
caches/dependency image remain. This is not Fleet OS-process restart for control
outcomes, approval decision recovery, combined extensions, central identity,
loaded config generation, task/PM admission, safe descendants or installed
enablement. Current local candidate evidence is not release-head CI.

All four preceding native scenarios also re-run and PASS on that same binary:
`lifecycle` (623172fa7e58), `recovery` (ffe69e33c2a0), `approvals` (f4bfeeb41fdb)
and `approval-recovery` (4e97ba3675fb). Together with both control cases this is
six current actual managed scenarios, separately counted from the 430 component
cases. Recovery uses two Fleet OS processes for prompt mapping; approval-recovery
uses two Fleet processes for the currently waiting request. Neither proves
Fleet OS-restart recovery for the new control-outcome journal. Existing approval
cases do not enable the control plugin or certify combined extensions.
Every report verifies exact source hashes, zero failures/ignored tests,
cleanup/empty ps and removal of its own QA image aliases. Their completion logs:

| Scenario | SHA256 |
| --- | --- |
| lifecycle | `de00468d226200562d37e22a7b1fa910b24462da84a81ba2702daca10be38d4d` |
| recovery | `660a4ec88bff0825ea884386c5155281b20a9f81060b2568613e1b4e0301dcbc` |
| approvals | `576e2ae2871ba4e32fb469fda88ef254d1bcd7a1bbf7855e8ba331db6d58019a` |
| approval-recovery | `6e62ab950b554f8c4de8628d4d19cd721ff3fd5db07618ee71d3fc9ef7972128` |

Current raw source SHA256:

| Source | SHA256 |
| --- | --- |
| `runtime/control_outcome_wire.rs` | `44855833421d7192e818ce7d7a821f97c78f32e552b1af4bd72175fd0529de31` |
| `runtime/control_outcome_readback.rs` | `652597340b69614a6b49788ae2d7609ce9b15d1a17b9e930421ac34722e4dac3` |
| `runtime/run_control.rs` | `bf3cfe80d202181a57b9aea6b703ac5fbd974794b62c6844028fbdb6cc6ffddc` |
| `tests/support/runtime_control_outcome_http.rs` | `cd2ee33b00f5ec7a6b740536e8e12a00bb3345873c768ff13ad367e7f70acd09` |
| `tests/native_supervisor_live.rs` | `d811551d323de67d2883f0774587fd38b5d36ef7e7392f7223dcfeddeff03e7e` |
| `shared/src/config.rs` | `26567cf245b21688d249cbf76632102138b79db2e9f98cfad6b8ed28700dd74e` |
| `control_fault_plugin.py` | `e2ff97ff3f9fec5c103e0777fbce77880c19576beb07d1b4842b5e833188b913` |

Final Docker grouping audit remains incomplete (exit1), not green: desktop35
and sdlc2-runner0 containers checked without violations; sdlc1-runner unavailable.
PR47/main5240107, dependency Base PR140/main177edb8 and both accepted main branches
are unchanged. The integration branch has no release PR. SDK/package pins,
applied migration bytes, sibling services, installed HOME/images/secrets/data
are preserved. Ordered one-migration release packets, exact-head CI/reviews,
approval outcome/combined compatibility, Fleet OS control recovery, config
generation/safe descendants and producer first-step/PM/Forge/live Chats acceptance
remain required before full merge readiness. No 100% readiness is claimed.

## Original Control Outcome Journal (5 October 2026)

Parent Fleet71eb9d6a758f66d326d2d0a359398a2130161c13 and unchanged Base
a48e53ee37a5a38b189f8a9cce84686d18fc5c83 identify this internal journal packet.
No source-pinned runtime is enabled, no POST/GET worker is added and no approval
decision recovery or task admission is claimed. SDK pin9408802 is unchanged;
applied migrations000013/000014 and read-only sibling services are untouched.

Additive000015 stores an immutable one-to-one original stop/steer context with
the submitted permit in one transaction. Claim revalidates actor, accepted
free-chat dispatch/native pins and exact semantic payload; the saved context
retains raw action bytes/hash and epoch/capabilities, not a bearer token. It
has no Debug/public DTO. Legacy submitted commands cannot be retrofitted.
Exact witnessed ACK commits outcome/receipt/stopping/audit/durable event
atomically. A late ACK preserves an independently observed terminal receipt,
run and transcript; it does not reopen execution. Deferred guards deny missing
required context or split ACK/receipt, identity changes and history deletion.

Seven actual PostgreSQL cases PASS: concurrent single-use claim/new repository,
closed context and original payload/identity, actor revocation and no legacy
backfill, exact idempotent ACK while capacity stays held, late terminal ACK,
audit-failure rollback and direct database guard denials. Isolated migration
upgrade/down/reapply preserves legacy users/commands and restored transition
logic; nonempty outcome downgrade fails without losing the version/history.
These are repository tests, not production Fleet/native recovery acceptance.

Failure history: owned project6ac7a7bda5b0 failed all-target test compilation
(DatabaseConnection is not Clone); new-repository test now opens a real fresh
connection. Project96932cfb2709 compiled but its seven setup attempts failed
the existing accepted-context guard because the fixture's explicit origin did
not match the agent port. Correct the fixture port, not the guard/assertions.
Both reached finally down and independent empty ps. The corrected target run
1a05e785e6bc passes all seven cases plus the isolated migration case.
Final exec77063/project`sdlc-qa-control-outcome-journal-cbceeaa01951` exit0:
417 distinct component cases PASS (232 libraries,175 foundation,1 isolated
approval,6 isolated migrations,3 supplemental PostgreSQL cases). The seven
targeted journal cases were repeated by the foundation suite, not counted
twice; one native renderer export remains ignored and is not counted. Locked
offline all-target check, strict workspace Clippy, fmt and generated OpenAPI
equality PASS. Clean migration CLI up/status/down-one/up/status covers all16
registered versions; doc-test commands pass with zero examples. Exact finally
down and independent empty ps confirm disposal; shared caches are retained.
Completion/PG diagnostics SHA256:
`a991b62d7f7c6301d2b07564163f71738fc6ad7d83c0253746dc071878c166ce`,
`99336f5171cc4fd9b2a30d13606f6f90b9bf99c37420ade2fa3f8f22c718a718`.

Node22.20/pnpm10.28.1 typecheck and generated-client equality PASS;111 Markdown
files, README and existing135 screen/9 chat/3 control fixture hashes verify.
No frontend source changed, no new browser/native capture ran and the previous
235 Vitest cases/build were not repeated for this repository-only packet.
Docker grouping audit is incomplete, not green: desktop40 and sdlc2-runner0
containers have no violations; sdlc1-runner access fails. Repeated VM clock
regressions also remain open infrastructure evidence. Base, SDK pin, old
migrations, public generated API, accepted resources and sibling trees are
unchanged. The integration branch has no release PR; PR47 and dependency PR140
remain untouched. Current local gates are not evidence of release-head CI.

The first broad run1a05e785e6bc is FAILED, not green:232 library cases,
173 of174 foundation cases, the isolated approval case, all six migration cases,
all-target check/strict Clippy/fmt and OpenAPI equality passed. Invalid/foreign
SSE settlement timed out. Its PostgreSQL log identifies a real session/run FK
deadlock, not an assumed clock artifact: run progress holds its row while its
event FK waits for the journal's exclusive session holder; that holder waits
for the run. The supplemental/clean-CLI stages did not run after this failure.
The isolated unchanged stream case2be2b34cf62f passes but does not resolve the
race. New deterministic case2321fad60c4e fails on old progress locking with
`deadlock detected`; after session-before-PM/run ordering,26db6d8929f2 passes.
No deadline/assertion was weakened and no retry was added to the runtime.
The seed scope is rechecked under the run lock; existing PM proof ordering and
native pins stay intact. All four owned projects reached exact cleanup/empty ps.

Pre-fix regression completion/PG log SHA256:
`2fc6c675fcdcab726dde558a5d215468aaa8ddaf7579ad0b897a74908e3fafda`,
`b730ad54b4129d3b71ccd8410335d1f2aa062e70fc2b950cdb0be22a62f17b37`.
Post-fix regression completion/PG log SHA256:
`71e3a714c603db914fa4468a61441f1d511a3a741bf2499a7f8ba6769d48d292`,
`d774b5138084c0597fd4faac0b8e87874da31672ed585370a337fe8c8e8b4b7d`.

Final journal source SHA256 (raw working-tree bytes after formatting):

- `backend/infra/src/lib.rs`:
  `ec68e2a10f76407d8ac1d6137d05ad402b3fe7606725ba9373202fda326bfb5c`
- `backend/infra/src/runtime_controls.rs`:
  `f12278e4193b596cbdaf5aa902e86c36240e7ad0f83c2816635322848eca12f8`
- `backend/infra/src/runtime/control_outcome_wire.rs`:
  `b4ef19f1e8b5fbe41772aa29bc38ab0fe5ace63cf0db65041a5b1868666ae5a5`
- `backend/infra/tests/support/runtime_control_outcomes.rs`:
  `91d9d1077eabc8e3f77e1b7a54a0589616d08141c3ee1481b93a17e1d46b4b8a`
- `backend/migration/src/m20261005_000015_runtime_control_outcomes.rs`:
  `322ae3c2ce4e980faeea4e023e0a46155627d2198f34223a4af4084356c7fe81`
- `backend/migration/tests/runtime_control_outcomes.rs`:
  `914ca9c7e5f9b072b8c229c8174bf53a590050d7826642a231d5484ab784f02f`

Remaining: connect supervisor exact-byte POST/producer headers and bounded
original-context GET worker to this journal; implement approval delivery fact
separately, including terminal-cancelled requests; prove concurrent normal ACK
and recovery plus actual native/Fleet/gateway restarts and combined extensions.
Keep the plugin disabled on installed agents. First-step/PM admission producers,
safe descendants, loaded generation, Forge handoff, seven-agent acceptance and
ordered exact-head release remain gates.

## Original Control Outcome GET Wire (5 October 2026)

Fleet parent49fddedf2511cc54ca247347ee66b135b82616ae and Base producer
a48e53ee37a5a38b189f8a9cce84686d18fc5c83 matched their remote branches before
this packet. Own Fleet source changes only the new control-outcome wire module,
its tests and module declaration; no supervisor call, API DTO, schema, SDK pin,
accepted runtime, Base source or read-only producer checkout was changed.

The wire module prepares exact serialized action bytes and a closed original
capability/epoch/origin/credential context. GET revalidates that saved context,
sends five exact query parameters without body and rejects malformed/foreign
witnesses. Eleven tests cover all three ACK types, uncertain hold, immutable raw
body roundtrip, duplicate JSON fields, version/scope/epoch/source drift, exact
command UUID identity, missing/error/redirect/MIME/encoding responses,
truncated/chunked bounds and encoded request size. Python record-separator
whitespace semantics are included; stop is empty bytes, not an empty JSON object.
Fixtures are loopback HTTP packets, not a real production Fleet recovery worker.

Final exec69253/project`sdlc-qa-control-outcome-wire-61c5bf77f6e8` exit0:
232 library cases (41 API,21 app,26 domain,132 infra,12 shared), including all11
new wire cases; strict locked/offline workspace all-target Clippy and fmt PASS.
The five lifecycle unit cases used the project's own disposable PostgreSQL.
One native renderer export is ignored, not counted. Completion log SHA256:
`6054d20ab49d5fab4528538a9b65a679d6edad21df06242ea682060270400196`.

Broad exec41134/project`sdlc-qa-fleet-approval-context-93fe21be9996` exit0:
167 foundation PG/HTTP cases,1 isolated approval event,5 additive migration
cases and3 supplemental PG cases PASS, clean migration CLI up/status/down-one/
up/status on15 registered versions, all-target check/strict Clippy/fmt and
generated OpenAPI equality. Its earlier library run preceded the final new-wire
whitespace/encoded-bound refinements; final232 libraries above verify those
current bytes. Together these cover408 distinct component cases, not408 new
cases or a single final-source native/installed acceptance run.
Broad completion log `e8b629379cb98b5a7f53991d0b4c3add3e9998195dd01b4f5aa225b50cead2b6`;
PG diagnostics `3f62c65392ae7ba4e6770cc85e5517fd5c3bbe3f0816fc3a8f37881b21211543`.
Repeated20-22second VM clock regressions remain an open infrastructure gap.

Failure history remains explicit: project0422bcae7f0f failed Compose validation
before creation (missing inherited postgres definition); project66be64124ed4
failed test compilation (mixed borrowed/owned streamed chunks); both corrected.
Project0845e58a3fcc passed the earlier11-case wire/Clippy scope. Projectc08eba2c441b
attempted the library scope without starting its declared PG service: five lifecycle
tests failed DB DNS,127 infra cases including all11 new cases passed; Cargo
stopped before the shared library suite. The final
61c5bf77f6e8 run starts a separate fresh PG and passes all232 without skips for
those failures. No failed run is counted green and no assertion was removed.
All own projects reached exact finally down; independent ps checks are empty.

Final source SHA256:

- `control_outcome_wire.rs`: `946e2f04c41511e05d66f67e0738e60499cc5880f0a25deae3363b347220490c`.
- `control_outcome_wire_tests.rs`: `c9e94f6c6ddaf4b6eaeb2470a5294905966e180d49917399e50d82106933cde7`.
- `runtime/mod.rs`: `224b37aade34065501ddcda20db5dcf7a646d3ed8cff5bc3206f4c2b91658360`.

Node22/pnpm10.28.1 exec39053 exit0: typecheck/lint/format,235 Vitest cases,
build and generated client equality PASS. Existing135 production-page fixtures,
nine chat-controller fixtures and three control fixtures retain verified hashes;
nine screenshot-verifier negative cases and111 Markdown file links PASS. No UI
change, new browser capture or live screenshot acceptance is claimed. The
706.74KiB Vite chunk warning remains.

PR47 is still Draft/main5240107 with five SUCCESS; Base PR140/main177edb8 and
the unchanged producer/source branch are separate packets. Their CI cannot be
attributed to this integration branch. Tracker PR114/main8c80a41, Workflow
PR90/mastere4fba60 and Forge task2 have no new compatible release/handoff in this
audit. The full goal remains incomplete: durable command/decision context and
single-use permit binding, atomic GET ACK/audit/event recovery without losing
terminal history, actual combined native approvals/Fleet restart acceptance,
safe descendants/config generation, first-step/PM producers and seven-agent
flow, ordered release/exact-head CI. See the
[consumer integration requirements](contracts/HERMES_CONTROL_OUTCOME_V1.md).

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
