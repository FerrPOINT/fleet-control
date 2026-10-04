# Gap Register

## October SDLC Gate

Automatic SDLC remains blocked. See the exhaustive owner/stage table in
[SDLC_IMPLEMENTATION.md](SDLC_IMPLEMENTATION.md). Foundation changes do not close
machine identity/project authorization, task binding, PM requirements/publication,
11 workflow modes/receipts, assignment leases/fencing, crash recovery, Hermes
acceptance proof, exact-SHA CI/deployment or seven-agent end-to-end acceptance.

Open gaps:

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
| Targeted tool approval UI and context evidence | Exact human-only backend decisions and integrated UI have PostgreSQL/authenticated fake-runtime and three-browser fixture evidence. Legacy run-wide approval is additionally denied inside the adapter. Independent unknown-outcome reconciliation and native approval/live evidence remain; native steer/stop acceptance does not close this gap |
| Native run control command recovery | Original accepted journal, fresh native capabilities/run/session, bounded exact ACK and no automatic retry are implemented; seven PG/HTTP cases and a separate real AIAgent steer/stop case pass. Durable command idempotency/receipt, unknown-outcome readback and concurrent actor/fencing reconciliation remain. Task-bound controls fail closed pending verified admission; interrupt ACK is not safe process-tree stop |
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
| Migration ownership and publication ordering | Release blocker | Fleet/Forge maintainers | Release Fleet 000010/000011 before the journal packet 000012, and Forge 39 before 40, with at most one new migration owned by each task PR. Preserve historical/applied bytes and validate each exact published head; the old Draft PR CI is not evidence for local commits. |
| Local legacy auth retirement and machine/project scopes | Security blocker for automatic SDLC | Backend | Verified assignment identities, project scopes and audited retirement of local fallback |
| Runtime OS/tool isolation is not implemented | Security blocker for hostile/untrusted workloads | Runtime/platform | Verified identity/container mounts, host-secret, cross-agent filesystem/network and shared-agent cross-user SessionDB denial tests |
| Central Auth loading/error screenshots and live identity acceptance | Evidence gap | Frontend/identity | Verified SSO client, failure/loading captures and cross-service subject/access tests |

Chat/session gaps:

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
