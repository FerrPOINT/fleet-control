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

| Gap | Exit criteria |
| --- | --- |
| Draft/assignment/chat/initial dispatch creation saga | Persisted operation state and restart-safe actual PM start after links and owner checks |
| PM structured tools and runtime-scoped machine credentials | Server-only bounded Base delegation client implemented; coordinator issuance ledger and runtime tool handoff remain. Real Hermes publishes questions/revisions through assigned machine API, no prose parsing. |
| Tracker outbox -> Fleet inbox/mirror projection | Transactional inbox/cursor/mirror repository implemented; authenticated background poller and live crash/reconnect acceptance remain. Projection is not PM delivery. |
| Bounded Tracker event projection | Before polling, add a versioned metadata-only projection with a serialized byte-budgeted contiguous prefix. Legacy events can contain a whole requirements document or task aggregate; reducing event count cannot make a single oversized event safe. Pin projection version per inbox stream and verify large/multibyte/replay cases. |
| Answer delivery and workflow continuation | Readback callback implemented; wire it to actual PM dispatch/checkpoint/rebind and one new run; late replies rejected |
| Readiness verifier integration | Trusted checklist/prerequisite receipts for exact revision/hash, no false Backlog |
| Server chat search/pagination/aggregate counts | Own-database and Chromium/Firefox/WebKit fixture acceptance passed in working branch; release review and live acceptance remain |
| Live acceptance and production screenshots | Real PM/owner/Tracker/Workflow flow, restart/denial tests; fixture screenshots stay separately labeled |
| Targeted tool approval UI and context evidence | Exact human-only backend decisions and integrated UI have PostgreSQL/authenticated fake-runtime and three-browser fixture evidence; independent unknown-outcome reconciliation and live evidence remain |
| Assignment replacement quiescence | Old runtime confirmed terminal/safely stopped before replacement; final authorization recheck does not replace a distributed fencing protocol |

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
