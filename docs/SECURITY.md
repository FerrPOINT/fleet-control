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
