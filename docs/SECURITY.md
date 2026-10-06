# Security

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
- Central users can expand user/session filters without local role grants.
  Private sessions remain owner-only regardless of historical role; shared
  leader-scoped sessions are available to all active central users. List filtering
  happens before the 200-row limit. Standalone expansion retains legacy RBAC.
- Backend authentication, request scopes and ownership checks are authoritative.
  The UI hides sections using `/api/v1/users/me/permissions`; historical human
  roles constrain only standalone legacy requests.
- Session SSE rechecks token validity, active user, ownership and standalone roles while
  replaying events. No bearer token is placed in a stream URL.
- The global `/api/v1/events` stream currently checks access only at connection
  time. Revoking a PAT does not stop that open stream; this is a release-blocking
  [known defect](https://github.com/FerrPOINT/fleet-control/issues/52), not a
  completed session-revocation guarantee.
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
