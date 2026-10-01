# Gap Register

## October SDLC Gate

Automatic SDLC remains blocked. See the exhaustive owner/stage table in
[SDLC_IMPLEMENTATION.md](SDLC_IMPLEMENTATION.md). Foundation changes do not close
machine identity/project authorization, task binding, PM requirements/publication,
11 workflow modes/receipts, assignment leases/fencing, crash recovery, Hermes
acceptance proof, exact-SHA CI/deployment or seven-agent end-to-end acceptance.

Open gaps:

| Gap | Severity | Owner | Exit criteria |
| --- | --- | --- | --- |
| Native Windows Rust linker missing: `link.exe` | Local tooling limitation | Environment | Install MSVC Build Tools for native Windows cargo commands |
| Live seven-agent acceptance not completed; standalone foundation Compose smoke passed | Product/integration blocker | Integration | Actual PM/publication/children/Rework/integration/deployment receipts; not fixture success |
| Java Agent chat/control/config activation are phase 2; jar lifecycle retained | Accepted scope | Runtime | Required chat/control capabilities verified before SDLC admission |
| Default-branch shared-base availability | Merge-order dependency | Platform | Merge Base #121 before Fleet #44; current Fleet CI pins compatible Base and verifies repo/container gates |
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
