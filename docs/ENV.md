# Environment

Prefix: `FLEET_CONTROL_`.

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
The coordinator does not persist credentials or retry without a human request.
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
coordinator does not provision mappings or mint PATs. Receipt GETs remain
read-only and do not perform this continuation check. Mapping creation time is
not an execution lease; passing this guard never grants runtime dispatch.
Admission/native bundle/workspace readiness still must be implemented and verified
before any PM runtime dispatch.

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
`FLEET_CONTROL_AUTH__BOOTSTRAP_ADMIN_SUB` is retired and ignored. Central users
have equal control-plane permissions without local role grants; service scopes
and private ownership still apply. Historical roles are not overwritten.
Existing runtime derivation key:
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
- `FLEET_CONTROL_FLEET__HERMES_SOURCE`
- `FLEET_CONTROL_FLEET__HERMES_COMMAND`
- `FLEET_CONTROL_FLEET__JAVA_AGENT_SOURCE`
- `FLEET_CONTROL_FLEET__JAVA_AGENT_COMMAND`
- `FLEET_CONTROL_FLEET__AGENT_PORT_BASE`
- `FLEET_CONTROL_FLEET__AGENT_PORT_STRIDE`
  | `FLEET_CONTROL_FLEET__RETENTION__STALE_ARCHIVED_DAYS` | 30 | Stale threshold (days) for archived agent folders |
  | `FLEET_CONTROL_FLEET__RETENTION__REVIEW_INTERVAL_SECS` | 3600 | Scheduled stale-folder review period (seconds) |

## PM Readback

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
