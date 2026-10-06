# Environment

Prefix: `FLEET_CONTROL_`.

## Docker Controller Configuration

The optional operator-owned TOML table `[fleet.container_control]` has python,
base_root, context and source_sha256 (three exact hashes in boundary/bootstrap/
control order). A selected Docker controller never falls back to native Hermes
when preparation, transport or receipt is unavailable. Missing table preserves
legacy native deployment, not container readiness. These fields are deployment
configuration, not user-editable agent settings; Python/Base paths must be
protected operator executables. No example signing key or runtime token is needed.

Preparation currently requires a closed private document in CONTROLLER_ROOT;
the exact shape and held states are described in the
[private contract](contracts/CONTAINER_CONTROL_V1.md#supervisor-binding).
Automated rendering, controller bridge attachment, daemon mount translation and
new-generation config rollback remain open. Keep installed enablement off until
real Fleet/Hermes container acceptance; this packet changes no accepted mounts,
images, ports, flags or secrets.

## Private Controller Storage

`FLEET_CONTROL_FLEET__CONTROLLER_ROOT` is operator-owned deployment configuration,
not an agent setting or a managed settings UI field. It defaults to empty; new
configuration activations stay held until an existing private directory is
configured. It must be outside `agents_root` and must not contain it, resolve
through links/junctions, or be mounted into any agent. Linux requires controller
UID ownership and exact mode0700; activation documents use exclusive mode0600.
The controller does not silently create, chmod or adopt an existing directory.
Windows ACL/directory durability is not certified; this operation fails closed
there pending platform integration. Linux remains the authoritative gate.

Preserve this storage across Fleet restarts and backups. Existing v1 journals in
agent config are not moved or deleted automatically; they block new activation
until reviewed reconciliation. New private documents are v2, include the original
agent/revision/config location and keep sensitive file backups. No automatic
crash takeover, reset or per-agent container enablement follows from this setting.
Installed mounts/Compose/image pins have not been changed. See
[operations](OPERATIONS.md#private-activation-recovery-storage).

## Hermes Original-Key Recovery

Stop/steer outcome recovery has its own default-false flag:
`FLEET_CONTROL_FLEET__HERMES_CONTROL_OUTCOME_ENABLED`. Explicit `true` requires
the committed Base `fleet-hermes-controls` plugin and pinned default-profile
capabilities at command preparation. It uses exact saved bytes and original
UUID/store headers only after atomic claim; background recovery is GET only.
Missing/uncertain/foreign context never falls back, resets a key or releases
capacity. Old submitted commands have no backfilled context. This flag enables
no approval-decision recovery, task admission, Java chat or safe OS stop. It is
read at Fleet startup; shipping plugin files is not enabling them. Keep installed
enablement off until native/release acceptance. See
[control outcome contract](contracts/HERMES_CONTROL_OUTCOME_V1.md).

`FLEET_CONTROL_FLEET__HERMES_RECOVERY_EXTENSION_ENABLED` defaults to `false`.
Explicit `true` requires the opt-in Base `fleet-hermes-recovery` plugin and its
source-pinned authenticated capability before a new free-chat dispatch. Missing
capability fails closed; no fallback POST. Freeze those facts before dispatch;
legacy intents are not upgraded. Plugin files shipped in an image do not enable
it: use an explicitly validated/activated config revision with
`plugins.enabled: [fleet-hermes-recovery]` only after installed acceptance.
Original submission carries the verified store epoch header, not a new secret.
This flag does not grant task/PM admission, model permissions or Java chat.
See [the contract](contracts/HERMES_RECOVERY_V1.md).

Hermes renderer 2 persists managed `API_SERVER_HOST=127.0.0.1`, assigned
`API_SERVER_PORT`, derived `API_SERVER_KEY`, HOME, native API enablement and CORS
into private files, because native dotenv/config loading can override launcher
env. Config env inputs cannot replace these fields. No new Fleet env variable
is needed. Legacy snapshots retain renderer 1; upgrade through an explicit
configuration revision, never by rewriting effective files or hashes.

Optional PM gateway uses `FLEET_CONTROL_TRACKER__URL` (fixed HTTP(S) root origin;
no credentials, query, fragment or path) and `FLEET_CONTROL_TRACKER__INSTANCE_ID`
(stable instance identity, matching Tracker config). Redirects are refused; bearer
credentials go only to that operator-configured origin. Configure an internal trusted
origin/TLS as appropriate. Task-bound transcript reads also require current Tracker
project access; an unavailable integration cannot bypass that check. Unbound private
transcripts remain independent. These variables do not enable autonomous PM execution.

## PM Draft Creation

Disabled by default. `FLEET_CONTROL_TRACKER__PM_DRAFT_CREATION_ENABLED=true`
enables only the creation endpoint, not runtime dispatch. Also configure the
explicit compatible test-project allowlist in the Fleet TOML configuration:

```toml
[tracker]
pm_draft_project_ids = ["00000000-0000-4000-8000-000000000001"]
```

Replace the example with an actual authorized Tracker project UUID. An empty
allowlist permits no creation. Tracker URL/instance and fresh verified human
credentials are required; root/machine PATs cannot create on behalf of an owner.
The coordinator does not persist bearer secrets or retry without a human request.
Disabling leaves task/chat receipts readable but rejects new continuation POSTs.
Each continuation POST also requires a fresh namespace ownership read from the
fixed `FLEET_CONTROL_FLEET__PROJECT_WORKFLOW_URL` root. The concrete PM agent's
namespace must be a canonical positive decimal ID, already provisioned in
Workflow for this exact Tracker instance/project. Configure these server values:

- `FLEET_CONTROL_PM__NAMESPACE_READ_PAT`: dedicated Base PAT with
  `project-workflow:read` and `project-workflow:namespace-owner:read:<namespace>`.
  Do not reuse catalog/callback/JWT/runtime-signing secrets; never expose it to runtime/UI.
- `FLEET_CONTROL_PM__NAMESPACE_AUTHORITY_ISSUER`: exact trusted Base issuer saved
  in the ownership mapping (HTTP(S) root without trailing slash).
- `FLEET_CONTROL_PM__NAMESPACE_PROVISIONER_SUBJECT`: canonical non-nil UUID of
  the trusted original namespace provisioner, not the current reader.

Workflow freshly introspects this PAT at Base. Missing/invalid config, denied
access, redirects, outages, invalid/oversized responses and authority mismatch
fail closed. A mapping for another Tracker project returns conflict. The
namespace guard does not provision mappings or mint PATs. Receipt GETs remain
read-only and do not perform this continuation check. Mapping creation time is
not an execution lease; passing this guard never grants runtime dispatch.
Admission/native bundle/workspace readiness still must be implemented and verified
before any PM runtime dispatch.

## PM Credential Preparation

Disabled by default, independently of Draft creation. Enable only on compatible
source versions with Base delegation policy and Tracker's receiving-service PM
grant restriction; SDK pins alone do not install those server endpoints.

- `FLEET_CONTROL_PM__CREDENTIALS__ENABLED`: default `false`.
- `FLEET_CONTROL_PM__CREDENTIALS__AUTH_URL`: fixed root HTTP(S) Base origin.
- `FLEET_CONTROL_PM__CREDENTIALS__MACHINE_SUBJECT`: canonical non-nil central UUID
  matching the frozen PM assignment, with active Tracker user/project membership.
- `FLEET_CONTROL_PM__CREDENTIALS__PARENT_PAT`: server-only root PAT with exactly
  `task-tracker:read` and `task-tracker:write`; never an agent env/UI setting.
- `FLEET_CONTROL_PM__CREDENTIALS__TTL_SECONDS`: default 300, permitted 1..1800.

Use the configured Tracker root origin. Base must opt in exact machine delegation
of the `task-tracker:sdlc:pm:` prefix. The coordinator persists original command,
parent fingerprint and origins before mutation, then rechecks parent/child and
Tracker context on every continuation. Rotated parent, changed origin or changed
TTL conflicts with an existing journal. Expiry/revocation needs explicit recovery;
it does not silently change key or renew a child. ACK metadata survives Tracker
failure; raw secrets are neither serialized nor persisted. Preparation does not
claim a lease, hand secrets to Hermes or enable model dispatch. Disabling prevents
new credential preparation while historical creation reads remain available.

## Tracker Metadata Polling

Disabled by default; configure only for a compatible test project with immutable
bindings and the `metadata_v1` producer. These are deployment-owned values, not
UI settings or agent runtime environment:

- `FLEET_CONTROL_TRACKER__EVENTS__ENABLED=true`
- `FLEET_CONTROL_TRACKER__EVENTS__AUTH_URL`: fixed root HTTP(S) Base origin.
- `FLEET_CONTROL_TRACKER__EVENTS__MACHINE_SUBJECT`: canonical non-nil Base UUID.
- `FLEET_CONTROL_TRACKER__EVENTS__READ_PAT`: dedicated server-only `sdlc_pat_`
  token with exactly `task-tracker:read`. Never put it in a URL, screenshot or repo.
- `FLEET_CONTROL_TRACKER__EVENTS__POLL_INTERVAL_SECONDS`: default 5, range 1..300.

The existing Tracker URL/instance values are mandatory when enabled. The machine
account must be active and an explicit member of relevant Tracker projects. Base
introspection and Tracker project access are checked each cycle. A root/admin or
PM-write PAT is not a substitute for the read-only token. Invalid enabled config
fails startup; disabling leaves history/cursors and active runtimes intact.
Token rotation requires an explicit deployment-secret update and Fleet restart.
No automatic PM credential issuance, business transition or dispatch is enabled.

October additions: `FLEET_CONTROL_SECRET__<REFERENCE>` supplies secret refs used
by config revisions; values are resolved only into per-agent managed `.env`.
`FLEET_CONTROL_AUTH__BOOTSTRAP_ADMIN_SUB` optionally names an exact verified
central subject for initial admin provisioning, only while no active local admin
exists. Remove it after bootstrap; other central users retain stored local roles,
never implicit admin. Existing runtime derivation key:
`FLEET_CONTROL_FLEET__RUNTIME_TOKEN_SECRET`.

Required production values:

- `FLEET_CONTROL_DATABASE__URL`
- `FLEET_CONTROL_JWT_SECRET`
- `FLEET_CONTROL_AUTH__MODE=hmac`
- `FLEET_CONTROL_AUTH__JWT_ISSUER=fleet-control`
- `FLEET_CONTROL_AUTH__JWT_AUDIENCE=sdlc`
- `POSTGRES_PASSWORD` when using Docker Compose

Important runtime values:

- `FLEET_CONTROL_FLEET__AGENTS_ROOT`
- `FLEET_CONTROL_FLEET__BASE_PACKAGE_CHECKOUT`: optional operator-owned local Git
  object cache for the private canonical Base repository. Empty disables package
  draft preparation. Fetch the exact package commit through normal Git auth
  outside Fleet; mount this cache read-only. Fleet never fetches, uses worktree
  files or changes the SDK `.base-revision`. Do not place access tokens in this
  setting, agent environment, URL, browser or repository.
- `FLEET_CONTROL_FLEET__HERMES_SOURCE`
- `FLEET_CONTROL_FLEET__HERMES_COMMAND`
- `FLEET_CONTROL_FLEET__JAVA_AGENT_SOURCE`
- `FLEET_CONTROL_FLEET__JAVA_AGENT_COMMAND`
- `FLEET_CONTROL_FLEET__AGENT_PORT_BASE`
- `FLEET_CONTROL_FLEET__AGENT_PORT_STRIDE`
| `FLEET_CONTROL_FLEET__RETENTION__STALE_ARCHIVED_DAYS` | 30 | Stale threshold (days) for archived agent folders |
| `FLEET_CONTROL_FLEET__RETENTION__REVIEW_INTERVAL_SECS` | 3600 | Scheduled stale-folder review period (seconds) |

## SDLC Configuration Observation

Disabled by default, with no automatic runtime admission:

- `FLEET_CONTROL_SDLC__CONFIGURATION_READBACK_ENABLED=true`
- `FLEET_CONTROL_SDLC__AUTH_URL`: fixed root HTTP(S) Base origin, no credentials,
  path, query or redirect. Private network/TLS boundaries remain deployment-owned.
- `FLEET_CONTROL_SDLC__CONFIGURATION_READER_SUBJECT`: exact canonical non-nil
  Base subject registered for the machine reader, not a browser administrator.
- `FLEET_CONTROL_SDLC__CONFIGURATION_READER_AGENT_IDS`: comma-separated canonical
  non-nil UUIDs, without spaces, duplicates or wildcards (4096 bytes maximum).
  This server-owned allowlist binds the reader to concrete agents, not a role.

The caller supplies a dedicated Base PAT with exactly `fleet-control:read`.
Its registered subject and the server-side agent allowlist are both mandatory.
Introspection is fresh, bounded to
16 KiB and five seconds, without proxy/redirect/retry or human auth fallback.
The PAT is not stored in Fleet settings or passed into Hermes. Never reuse
namespace, Tracker metadata, JWT, runtime or PM callback credentials.
The operator-owned Base package cache and actual active files are required;
this source-only observation does not enable scheduling or deployments.

## SDLC Workflow Binding Readback

- `FLEET_CONTROL_SDLC__WORKFLOW_BINDING__URL`: fixed root HTTP(S) Workflow
  origin, no embedded credentials, path, query or fragment. Empty by default.
- `FLEET_CONTROL_SDLC__WORKFLOW_BINDING__READ_PAT`: dedicated server-only Base
  PAT with exactly `project-workflow:read`, whose subject is registered as the
  Workflow Base catalog reader. It is omitted from serialized config and redacted
  in Debug. Never send it to Hermes, browsers, logs or audit.

The owner endpoint is `/internal/runtime/base/namespace-bindings/{namespace_id}`.
Readback is fresh, bounded to five seconds/16 KiB, without proxy, redirects,
retry, legacy catalog credentials or human authorization fallback. The owner
must have the compatible v3 candidate actually installed; accepted v2 is not
silently converted. Enabling these settings does not enable SDLC execution.

## PM Callback Credential

- `FLEET_CONTROL_PM__READBACK_TOKEN`: dedicated machine callback secret, 32..512
  ASCII graphic bytes without whitespace; unset by default. Must differ from JWT signing,
  runtime derivation and Workflow catalog secrets. Never expose it to agents or
  browsers. Configure the same value as Workflow's
  `PROJECT_WORKFLOW_PM_READBACK_TOKEN` and its fixed callback URL as
  `http://<fleet>/internal/runtime/v1/pm/runs` (Workflow appends the run UUID). Enabling the
  callback alone does not enable PM dispatch or automatic assignments.

Default ports:

- backend: `23801`
- frontend: `23802`

## OIDC mode (auth.mode=oidc)

- `FLEET_CONTROL_AUTH__MODE=oidc` — включает RS256/JWKS-валидацию access-токенов; локальный HMAC и local login отключены fail-closed.
- `FLEET_CONTROL_AUTH__OIDC_ISSUER_URL` — issuer провайдера (обязателен в oidc-режиме); `iss`-клейм проверяется строго.
- `FLEET_CONTROL_AUTH__OIDC_JWKS_URL` — переопределение JWKS (default `<issuer>/keys`).
- `FLEET_CONTROL_AUTH__OIDC_AUDIENCE` — ожидаемый `aud` (пусто = не проверять).
- `FLEET_CONTROL_AUTH__OIDC_ROLE_CLAIM` — клейм роли (default `role`; admin→Admin, operator/maintainer→Operator, прочее→User).
- `FLEET_CONTROL_AUTH__OIDC_JWKS_REFRESH_SECS` — интервал обновления кэша ключей (default 300; принудительный refresh при неизвестном `kid`).
