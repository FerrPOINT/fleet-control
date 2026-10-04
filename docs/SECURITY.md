# Security

Known pinned Hermes free-chat recovery requires the original accepted journal,
origin and credential context before HTTP. It performs GET only; neither a
missing/invalid status nor an expired horizon grants redispatch. Terminal mirror
state/delivery/output and durable invalidations commit under the same row locks;
late delta/tool/approval effects are denied after completion. This grants no
task/PM authority or process-tree isolation. See
[ADR 0018](adr/0018-atomic-terminal-pinned-recovery.md).

## Native Hermes Configuration Rendering

Renderer v2 seals the native API loopback address, assigned port, enablement,
derived credential, HOME and configured CORS. An explicit YAML CORS list is
required even when empty: native dotenv treats an empty value as no override.
Alternative native API aliases and malformed platform structures are rejected
before activation rather than allowing later merge/fallback to replace policy.
The runtime key exists only in private rendered files/launch environment; it is
not added to public configuration snapshots. Legacy v1 output/history are not
rewritten or silently upgraded. An upgrade uses validated drain/activation.

Native loader evidence on synthetic Rust-rendered files is not an attestation of
an installed runtime's effective revision, plugin inventory or model credentials.
Loaded-config proof, OS/tool isolation and assignment fencing remain required.

## Activation Journal

The managed configuration journal is local recovery material, not public evidence.
It contains previous runtime files, including resolved credentials in hexadecimal
(not encryption), and is created exclusively with Unix mode 0600. No Debug/API
representation is provided. Path guards reject links/junctions, foreign/duplicate/
traversal paths and non-regular backup files; reads and total size are bounded.
Partial/existing journals are preserved rather than overwritten. Acknowledgement
requires byte-identical readback after the database result commits. This protects
cooperative managed writes, not hostile host TOCTOU or cross-agent OS access;
physical/runtime isolation and operator reconciliation remain separate gates.

## Hermes Dispatch Journal

Original request bytes are private database/backup material, not public runtime
metadata. The intent has no Debug/Serialize representation, public endpoint or
audit payload. Raw bearer credentials are not persisted: the default-profile
fingerprint binds the original credential context but is not authentication,
encryption or proof of native store continuity. Bound verified capabilities omit
arbitrary upstream metadata; journal SQL errors expose only static diagnostics.

Single submission and ACK persistence share message locks. A late preflight error
cannot mark a submitted prompt failed and reject its real ACK. Recovery checks
original origin/fingerprint before known-ID GET; legacy records are not backfilled
from present credentials. Unknown IDs retain capacity and cannot trigger another
POST after timeout, token rotation, negative lookup or storage loss. Recovery and
operator reconciliation are still separate gates. A populated journal blocks
downgrade; database privileges/backups must protect its original evidence.

## SDLC Configuration Observation

The opt-in machine read requires fresh bounded Base PAT introspection at a fixed
trusted origin, the exact registered subject, only `fleet-control:read`, and a
deployment-owned concrete-agent UUID allowlist. Local/browser admin identity,
email, wildcard or broader PAT grants never substitute. No shadow user is created
and no PAT is persisted. Redirects, proxy-env, automatic retry, encoded/oversized
responses and schema drift fail closed; errors do not echo upstream content.

The response contains verified public pin metadata, not prompts, skill contents,
paths or secrets, and uses `Cache-Control: no-store`. Fresh Git/snapshot/HOME-file
checks and a repeated effective-head read are not a filesystem lock, OS sandbox,
native-loaded attestation or lease. `runtime_ready=false` prevents using this
observation as assignment acceptance. No runtime/Workflow/Forge or business
mutation rights follow from configuration read access.

## PM Clarification Commands

The server-only delegated PM client binds its credential to the assigned task
and permits only enumerated SDLC GET/POST operations. It denies legacy API paths,
other tasks, owner answers/confirmation and verifier/assignment actions before
adding Authorization. This does not constrain a bearer used outside that client:
Tracker must independently deny assignment-scoped PM tokens on legacy APIs and
check current assignment authority. Issuance is connected to opt-in creation
continuation; runtime handoff remains unwired. Preparation is not PM admission
or a completed live security gate.
The issuer disables redirects, automatic retries and environment proxies. It
validates TTL against acknowledgement time (Base issues after its database locks),
with five seconds of skew, while independently rejecting expired or overlong
credentials. Waiting for the issuer does not permit scope expansion or renewal
under a new key.

Before any Base mutation, Fleet pins the original parent fingerprint, exact
command/TTL/hash/key and fixed origins in the owner-only creation ledger. Both
parent and child pass fresh Base introspection; the subject must equal Tracker's
frozen machine UUID. The child has exactly Tracker read/write plus that assignment
grant. Its authenticated task context must match original human owner, instance,
project/task/root and all assignment fields, without human answer/confirm rights.
An ACK persists before Tracker readback; revoked/expired/stale authorization never
erases it or mints a replacement. Parent secrets are omitted from config JSON/Debug;
child secrets are memory-only, absent from journal, audit and public API. A parent
fingerprint is recovery metadata, not authority, encryption or a credential.
Operator rotation requires explicit reconciliation, not a new parent replay.
SQL shape/monotonicity guards and downgrade refusal protect recovery records.

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

- Central SSO uses verified ES256/JWKS and central-subject linkage. Local stored
  roles are authoritative; successful SSO does not grant admin. Standalone legacy
  authentication uses HMAC JWT access tokens and HttpOnly refresh cookies.
- New access tokens include fleet-compatible `aud`, `iss`, `role`, `scopes` and
  `sid` claims. Legacy compact tokens without `aud`/`iss` remain accepted during
  the migration window, but tokens that contain fleet claims are validated
  strictly against the configured issuer and audience.
- Local registration is disabled when Central Auth is configured. In standalone
  legacy mode the first registered user becomes `admin`; central bootstrap instead
  requires an explicitly configured verified subject and an audit record.
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
  admin/operator access, archived status, exact `agentN` confirmation, path
  recomputation from `agents_root`, non-symlink folder/marker checks and a
  matching `.fleet-agent.json` id.
- Agent storage reporting is read-only, recomputes managed paths from
  `agents_root/agentN`, and does not follow symlink targets while counting disk
  usage.
- Fleet-wide storage review uses the same guarded per-agent reports and never
  performs deletion or marker repair by itself.
- Session lists default to the authenticated user on the backend.
- Only admin/operator users can expand session/user filters to other users.
- Backend RBAC is authoritative. The UI hides sections using
  `/api/v1/users/me/permissions`, but every protected route still checks the
  current role.
- Session SSE rechecks token validity, active user, ownership and current role while
  replaying events. Task-bound streams additionally recheck authoritative Tracker
  project access and immutable binding before each emitted event. Revocation or
  dependency failure closes the stream, including its queued events. No bearer
  token is placed in a stream URL.
- Task-bound detail/history/messages/participants/runs/control reads and runtime
  stop require current project access, even for Fleet operators. Historical
  reassignment does not erase read access; it never authorizes fresh commands.
  Directory counts and legacy lists filter by one uncached Tracker project scope
  before pagination. Missing scope never exposes task-bound metadata. Standalone
  unconfigured Tracker lists show only unbound chats; configured failures fail closed.
- Exact approval decisions revalidate project access and current PM assignment
  after reservation lock waits, before runtime HTTP. Pre-dispatch rejection records
  terminal `failed`; unknown HTTP acceptance remains `uncertain` and is not retried.
  This narrows, but does not atomically eliminate, cross-service authorization races:
  assignment replacement must also quiesce the old run before automation is enabled.
- Human message requests cannot supply an agent author or runtime message ID.
  A scoped machine assignment protocol is still unimplemented, not a fallback
  permission granted to human or runtime clients.
- `/agents/**`, runtime actions, config, skills, leader team binding, settings,
  deployments, logs and audit log require admin/operator.
- User management and role updates require admin, except that operators can list
  users for session filtering.
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
