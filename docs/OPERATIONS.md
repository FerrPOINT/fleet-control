# Operations

## PM Stop And Terminal Readback025/026 Candidate

With025, the owner may Stop an originally accepted PM run even when the initial
guidance ACK was lost. Steer still requires acknowledged guidance or a continuation
checkpoint. This is not a dispatch permit: current owner/project/assignment,
original native/request custody and prior-stop checks still apply. Unknown
initial dispatch acceptance does not gain authority from the guidance exception.
Retain the same control key/body for recovery; do not create a new run or clear holds.

The026 source candidate permits explicit owner Stop during accepted-running PM
drain, in repository SQL and the DB trigger. Only Stop skips the PM MCP profile
verifier; Steer still requires that verifier and is blocked during drain. Owner,
project, assignment, coordinator, Tracker/Workflow, native and credential checks
remain unchanged. Pending PM activation409 remains; Stop is not permission to
activate beneath an active run or to start another run.

PM readback uses the same terminal validator as Hermes event/readback handling.
A `completed` status without exact boolean completed/partial/interrupted flags
is not terminal proof; unknown evidence retains the hold. Stop ACK alone does
not release capacity or prove business completion.

Rehearse025/026 on both supported lineages before deployment; candidate counts
are27/30.026 downgrade refuses any retained Stop joined to a PM binding;025
retains its guidance-exception downgrade guard. Preserve
receipts and use forward reconciliation, not history deletion or manual flag edits.
See [026 upgrade/downgrade boundaries](MIGRATIONS.md#pm-stop-under-drain026-candidate)
and [025 custody boundaries](MIGRATIONS.md#pm-stop-custody025-candidate).
The source candidate and authored tests do not constitute PG/native acceptance.

## PM Human Controls Migration Gate

Migration023 requires a clean and historical-data PostgreSQL rehearsal for both
supported lineages before changing accepted runtime. A historical delivered PM
answer is marked continued only from exact durable source-answer, checkpoint,
execution/fence, native custody and subsequent Workflow-gated evidence.

Unprovable history raises `legacy PM answer continuation requires authoritative
reconciliation before migration`. Treat this as a deployment blocker: preserve
the022 database and original receipts, obtain authoritative reconciliation and
repeat the isolated rehearsal. Do not edit answers, delete history, bypass the
guard or manually mark continuation confirmed. A clean synthetic migration is
not proof that an existing database can upgrade. Current-source PG rehearsal
and installed-runtime acceptance remain pending.

## PM Credential Reconciliation

Credential preparation is disabled by default. Enable only with approved Base
delegation policy, fixed integration origins and the exact Tracker machine
subject. After an unknown response, continue the same owner creation operation;
do not change the operation key, parent PAT or TTL to force a retry. Saved
acknowledgement is separate from Tracker verification and model dispatch.

An expired receipt or changed parent/origin/subject is a reconciliation blocker,
not permission to remint or edit the database. Preserve the existing operation,
token metadata and audit history. Downgrade migration000011 only after explicit
reconciliation; populated credential journals are deliberately protected.
Never publish token secrets when diagnosing Base/Tracker failures. See
[configuration](ENV.md#pm-credential-preparation) and
[pending acceptance](plans/2026-10-09-pm-credentials-release.md).

Task-chat rollback supports empty schema only. Populated transcript/allocation
order, bindings, creation, projection, PM run or approval history prevents down
before DDL, under table locks. Retain the schema and use a forward correction or
verified backup/restore; never delete history to make a downgrade pass. See
[migration rules](MIGRATIONS.md#historical-lineages-and-task-chats).

## Effective Configuration Readback

An operator/admin readiness response may contain
`effective_configuration_readback_failed` even when `effective_revision` is set.
The database head is not evidence that runtime files are intact. Inspect the
agent marker, isolated paths and active managed configuration through authorized
tooling; reconcile changed managed skills, secret references and deployment
configuration. Hermes-owned categories and `.bundled_manifest` are not removed
by this check. `runtime_skill_inventory_not_verified` is a separate integration
blocker, not an instruction to delete bundled skills.
Then validate/activate the intended revision using the normal drain/rollback flow.
Do not bypass the blocker, copy credentials between agents or treat a manual file
repair as a Workflow admission. This check does not attest runtime-loaded state
or protect against hostile concurrent host filesystem mutation.

## Transcript Order Migration

Pending migration 000010 backfills existing messages in timestamp/UUID order and
assigns new messages immutable database identity positions. Apply with a verified
backup and migration window: the column backfill, constraint and index creation
can lock or scan the message table. Measure the window on representative data.
This is an unapplied pending migration. If a preview database already applied an
earlier form of 000010, its migration-name record does not prove schema parity.
Preserve and inspect that database; use a fresh disposable QA database or a
reviewed forward upgrade. Never clear data or migration history to force reapply.
The public cursor remains a session-scoped message UUID; do not use the internal
allocation sequence as a commit watermark or an SSE cursor. Sequence gaps are normal.

Down/reapply retains messages but reconstructs positions from timestamps, so it
can change the order of post-upgrade messages after a clock rollback. Use this
cycle only in disposable QA. Production recovery requires a verified backup
restoring the order column or a reviewed forward migration, not blind down/up.

## Heartbeat Incidents

Monitor canonical `heartbeat_stale` incidents in Fleet alerts. Acknowledging
records that an operator saw the incident; it does not assert recovery. A
running agent must report a nonfuture health timestamp within ten minutes to
resolve its heartbeat incident automatically. Missing timestamps, clock skew
into the future, and stopped/degraded/failed statuses retain the incident.
Investigate health collection and clock synchronization rather than treating
unknown monitoring as healthy. Resolution and its redacted audit are one
transaction; an audit/storage failure must be retried, not reported as success.
This health signal does not authorize SDLC execution or prove workflow readiness.

## SDLC Foundation Recovery

PM Draft creation recovery is owner-driven: repeat the same project, agent,
title, original description and idempotency key with a freshly verified human
session. Do not invent a second key after a timeout. Tracker operation readback
precedes same-key writes; unknown acceptance retains earlier local receipts.
If ownership, original input or current assignment no longer matches, stop and
investigate the conflict. Do not delete the creation ledger or rewrite receipts.
GET operation readback is historical only and does not grant runtime authority.
The completed creation response remains `awaiting_admission`; no prompt has been
delivered. Operator/admin cannot resume as the owner or confirm their requirements.

Automatic SDLC is blocked; operator actions do not publish Tracker requirements
or bypass workflow gates. See [current scope](SDLC_IMPLEMENTATION.md).

PM gateway recovery: verify configured Tracker instance/origin and current project membership
before enabling dependent actions. A saved clarification answer is not proof of PM delivery.
After an unknown HTTP outcome, inspect the question/revision snapshot and replay only the same
intent/key; do not manufacture a new runtime run or edit confirmation rows. If requirements
changed, retain the user's draft, review the new document and submit a new explicit intent.
Fleet task-bound prompt/steer remains blocked until verified workflow orchestration is wired.

Save a config draft, validate it, then explicitly activate. Desired and effective
revisions can differ. During drain, do not force changes beneath active runs.
Published unqualified candidatec976c27 returns409 before setting drain while a bound
PM run is pending, even without a dispatch journal or with prepared/unknown
acceptance. Reconcile original custody; do not clear a reservation or issue a
new command to force activation. Accepted-running PM retains normal drain;
activation waits for its terminal proof. This guard adds no migration.
If activation fails and rollback is unconfirmed, keep the drain in place and
inspect the last error. Crash recovery/operator reconciliation is not yet a public
API; do not edit state rows to fabricate readiness.

An `uncertain` dispatch may have been accepted by Hermes. Never automatically
re-send it or clear its capacity hold. Investigate runtime session/run IDs and
acceptance before recovery. EOF without a terminal status remains waiting.

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

Hermes supports start, stop, restart and health. Existing Java jar lifecycle is
retained; its chat/control and config activation remain phase 2.

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
managed in `/settings`. The runtime, ports, integrations, access and retention
tabs edit one shared draft. Operators must review the server-generated field
diff before applying it; confirmation stores an immutable version and requests
a graceful Fleet Control restart. The history tab previews the same diff before
restoring an older snapshot, and rollback creates a new version rather than
rewriting history.

Backend/frontend bind ports, host mappings, database URLs, signing material and
integration tokens remain deployment-owned and are not editable in this UI.
Failed preview, apply or rollback requests keep the draft and confirmation
context so the operator can correct or retry the operation. A stale expected
version is rejected instead of overwriting another operator's change.

## Logs

Runtime stdout/stderr is captured into `agent_logs`. Secret-like markers are
redacted before persistence.

`/logs` has process logs, events and audit tabs. Use audit for role changes,
settings changes, skill/config edits, runtime actions, handoff and delegation.

## Recovery

### Accepted PM Runs

Set `pm.dispatch.enabled=false` to prevent new PM dispatch and continuation,
not to cancel an existing run. Preserve the original runtime credentials and
launch context when restarting Fleet: accepted-run status/event recovery remains
read-only and finishes its mirror without a new prompt. A changed context or
unknown ACK stays held for reconciliation. Do not clear custody or create a new
run to repair a missing terminal message. This behavior has source regressions;
native and current-source PostgreSQL acceptance remain separate gates.

### Tracker Metadata Worker

Enable only via the [deployment variables](ENV.md#tracker-metadata-polling).
The dedicated machine account needs explicit project membership and exactly
read-only Tracker scope. Do not reuse a PM or operator credential.

The worker scans 100 authorized bindings per keyset page with two concurrent
fetches and one bounded source page per binding each cycle. Persisted source
cursors survive a restart; replay is transactional and cannot redispatch a run.
Disabling the worker does not delete history or stop agents.

Warnings use static reason codes, never remote bodies, credentials or URLs:
`credential_revoked_or_expired`, `subject_scope_or_project_access_denied`,
`source_or_projection_reconciliation_required`, `projection_database_unavailable`
and `dependency_or_contract_unavailable`. Restore the correct account/scope,
project access or dependency before retrying. A corrupt/oversized source event
must be repaired at the source; do not skip it, reset a cursor or silently switch
a `legacy_full_v1` binding to metadata. Format conversion needs explicit migration.
After a deployment-secret rotation, restart Fleet and confirm replay without
duplicate transcript entries. These checks are not proof of PM answer delivery.

On backend restart, managed process handles are lost. The health action
reconciles status by marking an untracked Hermes process as stopped.

Use idempotency keys when retrying session/message create calls. If the previous
payload differs, the API returns `409` and the operator should create a new
intent instead of replaying the old key.

## OIDC authentication mode

`auth.mode=oidc` (см. docs/ENV.md): access-токены валидируются как RS256 против JWKS провайдера (кэш в памяти, refresh по интервалу и при неизвестном `kid`); `iss`/`aud` проверяются строго; HMAC-токены и локальный логин (`POST /api/v1/auth/login`) отклоняются — перевод на режим требует выданных провайдером токенов. Роль FC берётся из `oidc_role_claim` (admin→Admin, operator/maintainer→Operator, иначе User). Legacy-фоллбек компакт-токенов в oidc-режиме не действует.
