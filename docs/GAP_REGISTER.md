# Gap Register

## October SDLC Gate

Docker is the selected runtime architecture, not an open user choice. The
private [container control client](contracts/CONTAINER_CONTROL_V1.md) is added,
but the supervisor still uses native spawn. Do not count the client or Base
protocol QA as completed container lifecycle. Guarded Compose create/rendering,
Fleet DB binding before start, trusted bridge access, receipt-based stop/health/
logs and config drain/rollback are required before replacing that path.

Current Base protocol source is PR150
`57d717ff50b49133e12f5e2761141f72078859b1`, ready/CLEAN, nine green CI37417100487 jobs;
the following176e ready/green observation is historical. Full Fleet remains
not merge-ready; no installed image, migration or automation is changed here.

Base PR144 is now merged into main63fff28. New
[Base PR150](https://github.com/FerrPOINT/services-base/pull/150), head
`176e5d2265a330316af75dac9432dd1003a79121`, adds explicit sealed bridge v2 and
frozen Docker context/environment while retaining offline v1. It is a separate
ready-for-review candidate with [nine green exact-head CI jobs](https://github.com/FerrPOINT/services-base/actions/runs/37413423353),
not an installed Fleet consumer. Native authenticated HTTP, original stop and
sibling tests pass, alongside repeated v1 bootstrap/crash/descendant gates;
synthetic HTTP is not Hermes/model/PM acceptance. The source ledger records
the final bytes and failed preliminary checks. Fleet DB pre-exec binding,
loaded generation, atomic drain/activation/rollback, admission and PM resume
are still required. This doc-only packet does not change infrastructure/pins.

Before that merge, Base PR144 was reconciled at
`d2c8ef60ec4b9204d7c8232888b81022315c14b3`, CLEAN/MERGEABLE with nine successful
jobs in CI37409952184. This supersedes a0f7044 as current release head, without
closing Fleet consumption, installation or full PM acceptance.

Earlier release reconciliation has independent exact-head evidence: Fleet PR47
`5f20540ee33ff9451196e0cbe0149c337c35ae42` passes five CI jobs and remains Draft;
Base PR144 `a0f7044a95838504394d31bcf3f8d6cc682600c9` passes nine jobs and is
ready for review, not human-approved or merged. Both preserve current accepted
main changes. This supersedes older head/conflict observations below, not the
remaining ordered runtime releases or full PM/SDLC acceptance. Base boundary
still is not consumed by Fleet; Java chat/control remain phase two.

Automatic SDLC remains blocked. See the exhaustive owner/stage table in
[SDLC_IMPLEMENTATION.md](SDLC_IMPLEMENTATION.md). Foundation changes do not close
machine identity/project authorization, task binding, PM requirements/publication,
11 workflow modes/receipts, assignment leases/fencing, crash recovery, Hermes
acceptance proof, exact-SHA CI/deployment or seven-agent end-to-end acceptance.

Open gaps:

The source integration now reconciles accepted main `3c6b8ef` with both
historical split and combined foundations. Nine lineage cases, including eight
actual PostgreSQL cases, verify populated upgrades to the same runtime tail;
all fourteen historical migration files remain byte-identical to main. Verified
central names and the accepted SDK pin are preserved together with Fleet human
and central-subject proofs. This closes local reconciliation, not release:
ordered one-migration packets and exact-head CI/review remain mandatory. Do not
put all predecessor migrations into the new single-migration release packet.

The [runtime containment proposal](design/RUNTIME_CONTAINMENT_PROPOSAL.md)
records the recommended per-agent container boundary and delegated-cgroup
alternative. Base standalone original-ID stop/readback is implemented. The
main-based [Base PR144](https://github.com/FerrPOINT/services-base/pull/144),
head `dd2d0755266ef9081528702767006ba08e32d841`, adds pre-exec registration and
single-start/crash holds with actual namespace acceptance and44 Linux tests.
The linked proposal records exact fingerprints, release status and trust limits.
This candidate is not installed and Fleet lifecycle has not integrated its host
registry. Fleet now has a separate pre-spawn native agent/config/controller
PostgreSQL journal ([contract](contracts/RUNTIME_LAUNCH_JOURNAL_V1.md)); it prevents
duplicate controller launches and holds unknown outcomes, but is not a container
or loaded-generation receipt. Host persistence, generation fencing, drain, activation/rollback and
receipt commit/recovery remain. Accepted Compose/images/readiness policy are
unchanged. The accepted profile SDK pin does not install this boundary.
Safe Fleet descendant stop and loaded-generation acceptance
remain open until this integration and its own adversarial tests pass.

## PM Clarification Slice

Implemented: immutable task binding, paginated Fleet history, fixed-origin Tracker gateway,
owner-only structured answer/exact-revision confirmation UI, draft and unknown-outcome guards.
The follow-up branch also implements immutable PM run reservation and fresh
machine-only Workflow readback. These have PostgreSQL/test-runtime evidence,
not actual PM dispatch or resume acceptance.
These do not mean the complete approved plan is done.

Managed free-chat happy-path evidence is now available through the
[native supervisor harness](../scripts/native_supervisor_live/README.md): two
actual gateways, Fleet activation/dispatch/mirror and native restart readback.
This narrows the lifecycle compatibility gap, not unknown-ACK recovery or the
fenced admission, loaded inventory, process-tree and PM continuation gaps below.
No installed runtime or production UI acceptance is inferred.

| Gap | Exit criteria |
| --- | --- |
| Managed prepared dispatch custody/release | Saved prepared intent now keeps a pending message after pre-submission failure; historical failed delivery remains terminal. Fresh native capabilities restore the private launch binding only after original retained-child verification. Actual native fault/foreign-controller hold/original-controller recovery and separate submitted lost-202 GET recovery pass on the same recorded binary; see the verification ledger. This does not adopt a managed child after controller death, attest loaded generation/safe descendants or authorize task/PM dispatch. Ordered release and exact-head CI remain mandatory; existing Fleet PR47 is a one-migration Draft with main conflicts, not acceptance of this integration source. |
| Original approval decision native/release gate | Additive000016 fixes mode at reservation, stores immutable context with one claim and commits witnessed ACK/delivered/audit atomically. Opt-in API, exact-byte sender and original GET worker are connected; journal/migration and separate HTTP/PG evidence are recorded in the verification ledger. Native `approval-outcomes` passes actual once/deny and lost ACK GET recovery. Separate `approval-restart` passes three Fleet processes/two SIGKILLs with a surviving real gateway, new original GET after restart, one POST/native ACK/audit and unchanged context/dispatch/terminal history. `combined-recovery` also loses initial202 and verifies both committed plugins/original run lookup before decision recovery. Ordered release remains open. Legacy decisions cannot be backfilled. |
| Native control-outcome consumer | Base producer and Fleet opt-in stop/steer exact-byte POST/original-context GET worker are implemented with additive000015 journal. Wire/PG/HTTP tests cover single-send, identity/epoch, concurrent ACK, rollback, terminal/revocation and invalid contexts. Managed native transport-loss/gateway-restart and separate three-Fleet-process/two-SIGKILL recovery pass with real AIAgent and loopback model. `combined-controls` adds initial202 loss with both committed plugins, independent lookup barriers and original run identity. Exact source/binary/log/cleanup evidence is in the verification ledger. Release-head CI, installed rollout and safe descendants remain gates. |
| Exact producer release compatibility | Tracker PR114 head8c80a41 fails three of seven accepted generated chat schemas and excludes local Analysis/routing/reservation. Workflow PR90 heade4fba60 excludes local Base admission/binding and retains post-dispatch PM bind. Freeze/reconcile compatible owner release contracts and then perform live admission; divergent local checkout fixtures cannot replace release proof. |
| Configuration persistence and interrupted activation | Linux managed-file rename/unlink/new-directory barriers are implemented. Version 2 activation backups now require operator-provisioned private controller storage outside the agents root, with canonical provenance and owner/mode/link guards; legacy journals remain untouched and hold drain. Post-rename failure injection verifies held journal/drain and unchanged effective head. Automatic crash takeover, operator reconciliation, power-loss recovery, native loaded generation and safe descendant termination remain mandatory; visible bytes alone are not durable success. Private storage does not isolate an agent sharing the controller OS identity. |
| Draft/assignment/chat/initial dispatch creation saga | Owner-only opt-in coordinator and persisted operation reconcile authoritative Tracker Draft/input/initial reservation and create the atomic private PM chat. Recovery by owner/key and continuation by stored ID avoid prompt resubmission after a lost reply. The strict paginated Tracker project directory supplies rollout-filtered choices. Fresh Workflow namespace ownership is mandatory before every continuation, without credential fallback. It stops at awaiting_admission without prompts/runs. Creation UI has an isolated proposal, awaiting explicit approval and live controller integration; actual admission, first-step gate and restart-safe initial Hermes delivery remain open. |
| Fenced predispatch admission | Fresh Fleet effective configuration filesystem verification is implemented; it is not a durable/fenced receipt or proof of runtime-loaded configuration. Tracker PR #114 now implements execution ownership lease/CAS with actual PostgreSQL tests and green exact-tree CI; Fleet consumption remains unwired and dispatch remains false. Fleet chat/task-workspace receipts, Workflow execution claim/native catalog/first-step proof and Base scoped identity must be integrated. Namespace ownership GET is not a lease or runnable admission; the existing post-dispatch running bind cannot prove predispatch readiness. Unknown authority/CAS outcome requires exact readback, not another execution or dispatch key. |
| PM structured tools and runtime-scoped machine credentials | Creation continuation persists original-key/parent/origin credential intent, immutable ACK and fresh Base/Tracker checks; default disabled. Separate actual Base/Tracker producer acceptance verifies issuer replay, scoped context, denials and parent revocation with distinct central/local IDs. This is not the complete persisted creation saga, runtime handoff or admission. Durable renewal/revocation administration and runtime tool handoff remain. Real Hermes must publish questions/revisions through assigned machine API, no prose parsing. |
| Tracker outbox -> Fleet inbox/mirror projection | Transactional inbox/cursor/mirror and opt-in authenticated background poller implemented. Actual PostgreSQL replay and HTTP fault tests passed; live Base/Tracker crash/reconnect acceptance remains. Projection is not PM delivery. |
| Bounded Tracker event projection | Decoder, immutable format pins and byte-budgeted producer are implemented in separate Draft PRs. Actual Tracker HTTP snapshots verify all nine source digests; poller tests cover oversized/invalid pages without cursor progress. Live authenticated large/multibyte recovery remains. No implicit legacy conversion. |
| Answer delivery and workflow continuation | Readback callback implemented; wire it to actual PM dispatch/checkpoint/rebind and one new run; late replies rejected |
| Hermes acceptance recovery | Original immutable request/context journal, one-time submission permit, atomic ACK/terminal and GET-only pinned recovery are implemented. Opt-in Base witness extension and Fleet unknown-ID consumer add authenticated non-dispatch original-key lookup with frozen store/scope/source facts and DB-clock commit guard. Prepared restart delivery now verifies fresh protocol/identity and uses the same atomic permit, never resetting submitted receipts; five new PG/HTTP cases and the 331-case regression gate pass. Legacy records are not backfilled. Separate managed native lost-ACK evidence now covers an already-terminal free-chat run across two Fleet OS processes: one POST/inference/assistant, unchanged journal and no SSE. This is not installed enablement, running/approval recovery or native gateway crash. Missing/conflict/reset/expiry never allow redispatch or capacity release. Missed tool/approval replay, offline controls, native config/admission/process-tree quiescence, multiplex/compression and complete gateway/tool/Fleet acceptance remain open. Task/PM admission stays fail-closed. |
| Analysis projection and routing confirmation | Fleet now supports Analysis context, strict intent/reservation metadata and explicit optional routing-policy version on exact-revision confirmation. Legacy omission/null preserves its wire; summaries do not grant runtime dispatch. Generated API/schema parity, component/UI tests and unmodified Tracker PostgreSQL/HTTP metadata snapshot decoding cover all eleven event types. Reservation consumption/native admission, live service acceptance and a project routing editor remain separate gates. |
| Native event consumer vs replay | Bounded UTF-8/JSON/header/framing and mandatory original run identity are implemented with byte/event/text/snapshot/deadline limits; empty chunks do not extend idle time. See the consumer profile and verification ledger. Retirement retains capacity and does not safely stop a run. Native missed tool/approval history, control outcome readback and expired Fleet-cursor snapshot remain separate release requirements; there is no second native SSE consumer or automatic redispatch. |
| Readiness verifier integration | Trusted checklist/prerequisite receipts for exact revision/hash, no false Backlog |
| Native Hermes loaded-config attestation | Versioned v2 native renderer and two actual Rust-output/native-loader observations PASS; v1 effective history stays unchanged. This closes renderer/dotenv precedence, not an installed process's loaded revision, plugin inventory, model credentials, cross-instance ownership or assignment fencing. Explicit drain/activation upgrade and native runtime attestation remain required. |
| Compatible Workflow build and native skills | Canonical private Base pin `4b9b4c9297a13fb28a6ba2039af2f7cb719f2f58` is available via authorized Git; package hashes/inventory and pinned HOME readback are source-tested. Historical donor GitLab access is not a current blocker. Hermes native discovery/config precedence and frozen assignment/Workflow proof remain unverified; development capabilities 503 or metadata observation cannot grant admission. |
| Workflow mapping vs frozen runtime assignment | Source now uses fresh authenticated v3 owner ID/name/workflow/profile mapping, frozen separately in the configuration; symbolic namespace no longer substitutes for a persisted ID. Scoped Git/PG/HTTP tests cover profile/catalog drift and row-lock identity changes. Actual native profile/config attestation and frozen assignment ACK remain; source metadata is not admission. Accepted v2 is not switched or installed. |
| Server chat search/pagination/aggregate counts | Own-database and Chromium/Firefox/WebKit fixture acceptance passed in working branch; release review and live acceptance remain |
| Live acceptance and production screenshots | Real PM/owner/Tracker/Workflow flow, restart/denial tests; fixture screenshots stay separately labeled |
| Targeted tool approval UI and context evidence | Original accepted journal/run/session/origin/credential context, fresh native approval capabilities/current waiting request and bounded exact200 JSON ACK are enforced. Current waiting approval crash recovery now has native two-Fleet-process evidence; historical queue/tool replay remains. Journaled PG/HTTP cases cover denial and no-retry preflight holds; native once/deny/lost ACK proves one POST and held uncertainty, not outcome lookup. Run-wide approval stays denied. Central identity, installed/task/PM admission, configuration-generation authority and scoped live UI remain; existing UI has three-browser fixture evidence only. |
| Native run control command recovery                        | Migration 000013 adds immutable actor/key/payload/context, one durable claim, atomic ACK/audit/event, scoped readback and terminal-mirror reconciliation. A renewed372-case Linux/PG gate includes actual local JWT HTTP ownership/replay/revocation tests; sessionless human mutations are denied before lookup. Separate native ACK/replay and three-browser unknown-command/reload fixtures remain distinct evidence. Unknown command acceptance itself is not recovered from native key lookup; terminal_observed is not ACK. Live central identity, scoped machine/task control, multi-instance/native unknown-control coverage and release gates remain. Task-bound controls fail closed; interrupt ACK is not safe process-tree stop                                                                                                                                                                                                                                                                                                                                                                                       |
| Database clock regression in component QA | Actual broad approval gate records check4 (accepted before submitted); deterministic pre-fix PG test records check3. Additive000014 preserves logical submitted/accepted order after the unchanged guard; regression and isolated upgrade/down/reapply PASS. Original deadline/key/permit are not renewed. This does not repair host/VM synchronization or attest recovery-horizon clock integrity. The independently fixed PG deadlock is not attributed to clock jumps. See the verification ledger and ADR0023. |
| Assignment replacement quiescence | Old runtime confirmed terminal/safely stopped before replacement; final authorization recheck does not replace a distributed fencing protocol |
| Historical transcript ordering | New messages use immutable database identity allocation order; legacy listing and paginated UI no longer sort by host timestamps. Existing records are backfilled in their former timestamp/UUID order, not reconstructed insertion order. Allocation order is not commit order or the durable SSE cursor. Historical restoration requires independent evidence. |

SDLC send/steer stays fail-closed until verified assignments are integrated. Do not enable
automatic assignments or label this feature production-ready on the strength of UI fixtures.

Locally verified contract foundation: generated Rust response DTOs, seven Tracker wire
schema comparisons and malformed/unsafe-version rejection. CI runs the snapshot comparison;
deployment still requires comparison with the actual compatible Tracker build.

| Gap | Severity | Owner | Exit criteria |
| --- | --- | --- | --- |
| Native Windows Rust linker missing: `link.exe` | Local tooling limitation | Environment | Install MSVC Build Tools for native Windows cargo commands |
| Live seven-agent acceptance not completed; standalone foundation Compose smoke passed | Product/integration blocker | Integration | Actual PM/publication/children/Rework/integration/deployment receipts; not fixture success |
| Java Agent chat/control/config activation are phase 2; jar lifecycle retained | Accepted scope | Runtime | Required chat/control capabilities verified before SDLC admission |
| Scoped credential delegation availability | Merge-order dependency | Platform | Merge/deploy [Base #126](https://github.com/FerrPOINT/services-base/pull/126) before enabling real PM tools; existing UI/auth-core dependency pins do not provide the new runtime delegation API |
| Migration ownership and publication ordering | Release blocker | Fleet/Forge maintainers | Release Fleet 000010/000011 before journal000012, command ledger000013, time-order000014, control outcomes000015 and approval outcomes000016, in separate ordered packets; Forge39 precedes40. At most one new migration is owned by each task PR. Preserve historical/applied bytes and validate each exact published head; old Draft PR CI is not evidence for integration commits. |
| Local legacy auth retirement and machine/project scopes | Security blocker for automatic SDLC | Backend | Verified assignment identities, project scopes and audited retirement of local fallback |
| Runtime OS/tool isolation is not implemented | Security blocker for hostile/untrusted workloads | Runtime/platform | Verified identity/container mounts, host-secret, cross-agent filesystem/network and shared-agent cross-user SessionDB denial tests |
| Lifecycle observation timed out once in the controller-storage gate | Test stability evidence gap | Runtime/testing | Diagnose retained SOUL-observation timeout with the new early-exit diagnostics and exact-source CI; focused and final full runs pass without increasing the timeout, but one successful rerun does not explain the earlier stall |
| Central Auth loading/error screenshots and live identity acceptance | Evidence gap | Frontend/identity | Verified SSO client, failure/loading captures and cross-service subject/access tests |

Chat/session gaps:

Current approval snapshot recovery is implemented and has native two-process
evidence. The native status document is not a complete approval queue: missed
historical questions/tool events and unknown decision ACK recovery remain open.
Parent Child wait/exit is not descendant quiescence; the current launcher uses
exec, not a per-agent cgroup/container/job boundary. Safe descendant termination,
loaded-generation proof and task/PM admission still block full SDLC readiness.

The closed labels below describe the historical baseline, not complete chat acceptance.
The source-reviewed chat gaps `CHAT-01` through `CHAT-14` remain open in
[CHAT.md](CHAT.md). They include central/legacy permission differences,
parent access validation, autonomous leader identity, leader-authored dispatch,
run history retention, durable dispatch, replay/concurrency, SSE recovery,
approval controls and handoff/continuation. Closing a baseline feature does
not close those execution and authorization gaps.

Closed gaps:

- leaders/executors product split
- user-owned private-by-default sessions
- selected leader and delegation model
- session participants/runs
- idempotent session/message creation
- admin/operator/user RBAC model
- deployment job UI/API
- settings UI/API
- generated screenshot manifest
- WSL/Linux backend check, clippy and tests
- OpenAPI source regeneration from Rust
- `services-base` telemetry-compatible local bridge
- fleet-compatible local JWT claims with strict issuer/audience validation and
  legacy compact-token fallback
- explicit archived-agent folder purge with confirmation, marker guard, event
  and audit entry
- agent storage and retention preview before physical purge
- fleet-wide agent storage review with purge candidates and marker/path issues
