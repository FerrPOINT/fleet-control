# Operations

## Managed settings restart

`POST /api/v1/settings/managed/apply` and the rollback endpoint persist the
new active snapshot and its audit record atomically, return the accepted
version, then request graceful process shutdown. The server binary exits with
code `75`; the production Compose service uses `restart: unless-stopped` and
starts again with the active database snapshot overlaid on the deployment
baseline.

Before apply, call `/api/v1/settings/managed/preview` and display every changed
field. Clients must send the previewed `active_version` back as
`expected_active_version` and require explicit restart confirmation. A `409`
means another operator changed settings and the preview must be refreshed.

If the supervisor does not restart the process, start it with the normal
deployment command. The active version remains durable. Secrets, database
connectivity and container port mappings are never sourced from managed
settings, so recovery remains possible from the deployment environment.

## Provisioning

Provisioning creates database rows first, then materializes folders. Re-running
provision for the same agent is safe when the marker belongs to the same agent.

## Runtime Lifecycle

Hermes supports start, stop, restart and health. Java Agent operations are
reserved for phase 2 and return a typed validation response.

## Agent File Purge

Default agent delete archives the agent and leaves files intact. Physical purge
is a separate operator action:

1. Archive the agent.
2. Open the agent workspace tab.
3. Review the storage report totals, marker status and retention hint.
   The report also flags `stale` archived agents (older than
   `fleet.retention.stale_archived_days`, default 30) and shows
   `archived_days`; the scheduled stale-folder review logs stale agents
   every `fleet.retention.review_interval_secs` (default hourly), and an
   operator can run a pass on demand via
   `POST /api/v1/settings/retention/review` (operator role, audited).
4. Type the exact `agentN` name into the purge confirmation field.
5. Run file purge.

The backend recomputes `agents_root/agentN`, rejects symlinks, requires a
matching `.fleet-agent.json` marker, removes only that folder and writes both an
event and an audit entry. Purge does not remove database sessions, logs or
audit history.

The storage report is read-only. It scans `runtime`, `config`, `workspace` and
`logs`, counts files, directories and symlinks without following symlink
targets, and reports whether the folder is currently purge-eligible.

The technical `/agents` inventory also has a fleet-wide storage review. It
aggregates the same read-only reports across every managed agent and highlights
archived folders that are purge-ready plus marker/path issues that require
operator inspection before any destructive action.

## Deployments

Provision and runtime update work is represented by deployment jobs. Operators
use `/deployments?tab=jobs` to create, inspect and cancel jobs.

## Settings

Runtime roots, runtime sources, port ranges, integrations and auth settings are
managed in `/settings`. Secret-like values must be returned redacted.

## Logs

Runtime stdout/stderr is captured into `agent_logs`. Secret-like markers are
redacted before persistence.

`/logs` has process logs, events and audit tabs. Use audit for role changes,
settings changes, skill/config edits, runtime actions, handoff and delegation.

## Recovery

On backend restart, managed process handles are lost. The health action
reconciles status by marking an untracked Hermes process as stopped.

Use idempotency keys when retrying session/message create calls. If the previous
payload differs, the API returns `409` and the operator should create a new
intent instead of replaying the old key.


## OIDC authentication mode

`auth.mode=oidc` (см. docs/ENV.md): access-токены валидируются как RS256 против JWKS провайдера (кэш в памяти, refresh по интервалу и при неизвестном `kid`); `iss`/`aud` проверяются строго; HMAC-токены и локальный логин (`POST /api/v1/auth/login`) отклоняются — перевод на режим требует выданных провайдером токенов. Роль FC берётся из `oidc_role_claim` (admin→Admin, operator/maintainer→Operator, иначе User). Legacy-фоллбек компакт-токенов в oidc-режиме не действует.
