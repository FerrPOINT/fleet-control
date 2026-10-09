# Security

## Original-Key Recovery Boundary

Recovery uses the original authenticated origin and credential fingerprint, exact
serialized request/hash/key and closed source/profile/store facts frozen before
submission. It never opens runtime SQLite or posts a second run on unknown
acceptance. Negative/foreign/expired witnesses and invalid terminal evidence hold
capacity. Response sizes and HTTP deadlines are bounded; redirects, proxy-env and
automatic retries are disabled by the supervisor client. Terminal diagnostics
do not echo database payloads, and mirrors are redacted before persistence.
Lookup is not task/PM model authority or proof of safe stop. Capability strings
do not attest installed producer bytes: source/native acceptance remains a gate.

## Persisted PM Credential Preparation

The opt-in PM coordinator pins the exact Base command, parent fingerprint,
machine subject, fixed origins and frozen Tracker assignment before issuance.
Every continuation freshly verifies parent/child scopes and Tracker ownership
context; changed or expired receipts fail closed. Raw bearers stay memory-only,
are excluded from config serialization/Debug and never enter journal/audit/API
payloads. Persisted token metadata is not workflow or model authority. See
[ADR0014](adr/0014-persisted-pm-credential-preparation.md) and
[release limitations](plans/2026-10-09-pm-credentials-release.md).

## Central Private-Owner Boundary

Central service access and an expanded user filter do not expose another user's
private chat. The task directory uses the same owner predicate for result rows,
agent counts and cursor validation. Task history/context/control/gateway reads
reuse the authoritative session read guard; bound operations also validate
Tracker project access. Business answers and confirmation remain owner-only.
This preserves accepted main's central human/PAT policy without reviving the
historical stored-role admin bypass. Standalone legacy roles are a separate
compatibility path, not proof of central privileges.

## PM Clarification Commands

The server-only delegated PM client binds its credential to the assigned task
and permits only enumerated SDLC GET/POST operations. It denies legacy API paths,
other tasks, owner answers/confirmation and verifier/assignment actions before
adding Authorization. This does not constrain a bearer used outside that client:
Tracker must independently deny assignment-scoped PM tokens on legacy APIs and
check current assignment authority. Credential issuance/runtime handoff remains
unwired; this restriction is not PM admission or a completed live security gate.

Creation recovery is owner-only even for operators/admins; key readback validates
the current project before the indexed owner/key lookup. Continuation accepts
only an empty JSON object and cannot replace persisted original input or agent.
Invalid shapes are rejected before upstream calls. Project names come only from
the strict active-central-subject Tracker directory, without a legacy/admin
fallback. Fleet then restricts the page to its rollout allowlist; no readiness
or runtime authorization follows from appearing in this list.

Task binding and owner answers/confirmation require a verified Central Auth subject;
missing central identity returns unauthorized, not legacy fallback. Tracker independently
checks active human session, service grant, project membership and owner subject. Fleet
read-all never authorizes acting for another owner. Only the fixed configured Tracker
origin receives the original bearer; redirects/user-supplied origins are forbidden.
Task-bound chats reject unverified prompt/steer, reassignment and leader controls.
The PM machine and readiness-verifier identities are separate from human consent.
Autonomous rollout remains blocked until scoped runtime tools/readback are integrated.

PM Draft creation additionally requires both fresh verified human-session and
central-subject markers, a project allowlist and current Tracker project access.
Machine PATs and local human markers without verified central identity cannot
create or recover the operation. Even admin/operator can only read their own
creation ledger. Tracker rechecks the owner for every business request. Receipt
readback exposes IDs/progress, not original content, machine identity or tokens.
Human credentials stay request-local; original input remains private DB/backup content.
Creation itself grants no runtime tool rights or admission capability.
Every continuation additionally checks fresh Workflow namespace ownership with
a dedicated server-only Base PAT. Human/session, catalog and callback credentials
cannot substitute for that read PAT. The PM callback also refuses reuse of the
namespace PAT as its readback secret. Namespace reads also refuse reuse of JWT
or runtime-signing secrets. The fixed origin, exact project/instance,
authority issuer and original provisioner are checked; redirects, proxy-env,
automatic retries, schema drift and unbounded bodies are refused. Errors do not
echo upstream bodies or tokens. A saved mapping/chat is not a fenced execution
lease. This check cannot authorize dispatch or bypass missing native readiness.

- Central SSO uses verified ES256/JWKS and central-subject linkage. All active
  central users have equal control-plane access; stored roles are historical and
  are never promoted by SSO. PAT service read/write scopes remain authoritative.
  Standalone legacy
  authentication uses HMAC JWT access tokens and HttpOnly refresh cookies.
- New access tokens include fleet-compatible `aud`, `iss`, `role`, `scopes` and
  `sid` claims. Legacy compact tokens without `aud`/`iss` remain accepted during
  the migration window, but tokens that contain fleet claims are validated
  strictly against the configured issuer and audience.
- Local registration is disabled when Central Auth is configured. In standalone
  legacy mode the first registered user becomes `admin`. Central users are
  created in Central Auth; Fleet role mutation is disabled and the former
  bootstrap-admin subject setting has no effect.
- `SystemRole = admin | operator | user`; `is_system_admin` remains a derived
  compatibility alias for `admin`.
- Filesystem access must be derived from database-managed agent paths.
- Local child runtimes share the host OS identity. Per-agent paths, env and keys
  provide configuration isolation, not a hostile-code sandbox. Arbitrary code
  under the same identity can access sibling/host files. Untrusted multi-tenant
  execution is unsupported until OS identity/container isolation is verified;
  see [Threat model](THREAT_MODEL.md).
- Reject `..` traversal, reject absolute paths outside the configured agents
  root, and verify existing `.fleet-agent.json` markers before provisioning.
- Secret-like env/log values are redacted before persistence and API return.
- New configuration drafts reject plaintext secret-named fields and env injection.
  Configuration reads also mask secret-named legacy database fields, while
  retaining valid reference-only objects for editing. Non-secret task keys are
  not treated as credentials.
  A secret reference object contains only `secret_ref`; raw material comes from
  `FLEET_CONTROL_SECRET__<REF>` and is resolved only into managed runtime files.
  Known-secret suffixes are withheld across streaming fragments before mirroring.
  Unknown arbitrary credentials in free text cannot be guaranteed detectable.
- `FLEET_CONTROL_FLEET__RUNTIME_TOKEN_SECRET` is required. Fleet derives a
  deterministic per-agent `API_SERVER_KEY` from that secret and the agent id;
  the raw token is written only to the managed agent env/config surface.
- Physical folder purge is not part of default delete. It requires
  authenticated central write access (legacy admin/operator), archived status,
  exact `agentN` confirmation, path
  recomputation from `agents_root`, non-symlink folder/marker checks and a
  matching `.fleet-agent.json` id.
- Agent storage reporting is read-only, recomputes managed paths from
  `agents_root/agentN`, and does not follow symlink targets while counting disk
  usage.
- Fleet-wide storage review uses the same guarded per-agent reports and never
  performs deletion or marker repair by itself.
- Session lists default to the authenticated user on the backend.
- Task-bound detail/history/messages/participants/runs/control reads and runtime
  stop/steer require current project access, even for Fleet operators. Stop/steer
  enforce this before the task-bound assignment conflict and reserve no control
  when denied. Historical
  reassignment does not erase read access; it never authorizes fresh commands.
  Directory counts and legacy lists filter by one uncached Tracker project scope
  before pagination. Missing scope never exposes task-bound metadata. Standalone
  unconfigured Tracker lists show only unbound chats; configured failures fail closed.
- Exact approval decisions revalidate project access and current PM assignment
  after reservation lock waits, before runtime HTTP. Pre-dispatch rejection records
  terminal `failed`; unknown HTTP acceptance remains `uncertain` and is not retried.
  This narrows, but does not atomically eliminate, cross-service authorization races:
  assignment replacement must also quiesce the old run before automation is enabled.
- Central users can expand user/session filters without local role grants.
  Private sessions remain owner-only regardless of historical role; shared
  leader-scoped sessions are available to all active central users. List filtering
  happens before the 200-row limit. Standalone expansion retains legacy RBAC.
- Backend authentication, request scopes and ownership checks are authoritative.
  The UI hides sections using `/api/v1/users/me/permissions`; historical human
  roles constrain only standalone legacy requests.
- Session SSE rechecks token validity, active user, ownership and standalone roles
  while replaying events. It also binds the revalidated central profile or legacy
  token subject to the original user before returning queued events. An active
  token for a different user cannot retain a private session stream.
  Global `/api/v1/events` also rechecks the bearer token,
  its subject binding and service read scope before delivering each event and
  once per second while idle. Revocation, expiry, disabled users, Auth outage or
  database errors close the stream without a local fallback. Legacy mode also
  rechecks the current stored operator role. Central private session events are
  owner-only; leader-scoped events remain shared. No bearer token is placed in
  a stream URL.
- Task-bound streams additionally recheck authoritative Tracker project access
  and immutable binding before each emitted event. Revocation or dependency
  failure closes the stream, including queued events. Central control-plane
  access does not bypass task ownership, current assignment or project access.
- Human message requests cannot supply an agent author or runtime message ID.
  A scoped machine assignment protocol is still unimplemented, not a fallback
  permission granted to human or runtime clients.
- `/agents/**`, runtime actions, config, skills, leader team binding, settings,
  deployments, logs and audit log are available to active central users with
  matching service scopes, or legacy admin/operator users.
- Central user management lives in Admin Panel. Fleet synchronizes the central
  directory for every active central user; local role updates are forbidden.
  Standalone legacy user management retains its existing admin/operator checks.
- A session without `leader_agent_id` is private and is not readable as a
  leader-scoped task.
- Selecting a leader for an executor session requires an existing
  `leader_executors` binding.
- Fleet Control stores transcript mirrors and dispatches through runtime
  adapters; it must not write directly into Hermes SessionDB.
- Mutating control-plane actions write `audit_log` entries with redacted
  payloads; message audits store author/type/length metadata instead of a
  second full prompt copy.
- Idempotency keys protect session and message creation from duplicate browser
  submits or retry storms. Reusing a key with a different payload returns
  conflict.

## Browser Authentication Boundary

Every login and logout advances a memory-only authentication generation, even
when the subject and token are unchanged. Query caches and mounted forms belong
to that generation; replacing it clears the previous cache and remounts forms.
Pending central sign-out preserves the current draft until navigation, but
blocks new API requests. Failed sign-out does not silently discard the draft.

Each API request captures its original generation, subject and bearer token.
After response parsing, both success and error paths reject results belonging
to an obsolete login. A late `401` cannot log out the next user. Permission
responses must match the current subject; SSO completion cannot replace a newer
login. Tokens and authentication generations are not persisted in local storage.

An authentication change is not proof that a dispatched mutation was rejected.
`AuthContextChangedError` deliberately is not a definite HTTP rejection and
must not authorize automatic redispatch. Runtime command recovery, server-side
ownership, stream revocation and OS/runtime isolation remain separate gates;
this browser boundary does not replace them or establish full SDLC readiness.
