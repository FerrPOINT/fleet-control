# Threat Model

Status: 2026-10-01 foundation review. These controls do not establish complete
automatic SDLC or hostile-code isolation. See [Security](SECURITY.md) and
[SDLC implementation](SDLC_IMPLEMENTATION.md).

Primary risks:

- accidental sharing of one `HERMES_HOME` between agents
- path traversal from workspace/config inputs
- leaking model/API tokens through env previews or logs
- stale process status after backend restart
- starting Java Agent before its isolation policy is complete
- showing private executor sessions to a leader without explicit selection
- allowing a leader to write into an executor session outside its team binding
- ordinary user expanding session filters to other users
- accepting forged or cross-service JWTs with the wrong issuer or audience
- duplicate prompt delivery during browser retries
- role/settings/deployment changes without audit evidence
- direct mutation of Hermes SessionDB instead of using the runtime boundary
- agent code running with the Fleet OS identity reading another agent's files
- external symlink/junction replacement between path validation and file access
- unknown runtime acceptance or stream EOF being mistaken for successful delivery
- desired config being exposed as effective after an unverified activation
- a valid central login accidentally acquiring infrastructure admin privileges

Controls:

- sequential DB identity with unique paths
- guarded path helpers and folder markers
- per-agent runtime API keys derived from
  `FLEET_CONTROL_FLEET__RUNTIME_TOKEN_SECRET`
- redaction at write time
- status reconciliation through health checks
- existing Java jar lifecycle retained; chat/control/config activation and
  automatic SDLC admission remain phase 2
- backend current-user filtering by default
- admin/operator-only expansion to all/multiple users
- backend RBAC for all fleet infrastructure routes
- strict issuer/audience validation for fleet-claim JWTs and legacy fallback
  only for compact local tokens without `aud`/`iss`
- `leader_executors` validation before leader assignment
- idempotency keys and payload hashes for session/message create
- deployment/settings/role changes recorded in `audit_log`
- `session_messages` as Fleet mirror and runtime supervisor dispatch boundary
- transactional prompt outbox and capacity reservation; no unknown redispatch
- terminal status readback after EOF; interrupted runs recorded as failed
- desired/effective revisions, activation drain and fail-closed rollback
- verified central subject resolves to the stored local role, without admin bypass

## Residual Trust Boundaries

Managed runtimes are local child processes, not per-agent OS sandboxes. Separate
directories, environment and bearer tokens prevent accidental configuration
sharing; they do not stop arbitrary agent code with the same OS identity from
reading sibling files or host secrets. Unix `0600` protects against other OS
users, not another process under the same identity. Path guards protect Fleet
managed operations, not every filesystem action an agent can execute.

Fleet ownership checks protect HTTP and stream access. They do not by themselves
prevent a runtime tool from inspecting another user's history in a shared agent's
SessionDB. Private chat therefore does not yet establish end-to-end hostile-tenant
isolation; runtime tool permissions and per-assignment storage boundaries also
need verification before such a claim can be made.

Until an OS identity/container policy is verified, only trusted local operators
and trusted runtime code are within this deployment's trust model. Hostile
multi-tenant execution is unsupported. Untrusted automatic workloads require
isolated identities/mounts, restricted network and verified workspace boundaries.
External filesystem races and activation crash recovery remain open gaps.

Machine assignment identity, project scopes and fencing are not yet implemented.
Automatic SDLC readiness remains blocked rather than granting these capabilities
implicitly. Fixture tests/screenshots do not close these security boundaries.
