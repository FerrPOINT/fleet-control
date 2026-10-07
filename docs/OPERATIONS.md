# Operations

## Recovered Namespace Stop

With candidate000022 and compatible original Base source, the normal authorized
runtime stop action can contain the original recovered namespace. It requires
fresh current DB/native custody; configuration drain protections still apply.
A saved stop intent/claim is not confirmation that the process exited. Preserve
the intent, custody envelope, Base stop journal and original launch/registration.
Never reset/delete a claim, issue a new operation ID or invoke a manual Docker
kill to manufacture a successful receipt.

After a lost reply, repeat the authorized action only for read-only original
namespace reconciliation. If it remains running or unavailable, the action stays
held and does not send another stop. An expired lease cannot authorize new stop,
but does not invalidate independently confirmed exit of a previously claimed
operation. Confirmed original exit commits
one outcome and stopped runtime metadata. This does not complete the chat/task,
resolve a pending tool approval or release unknown message/control acceptance.
Keep these blockers visible; automatic restart/configuration takeover and
restored PM workflow remain separate gates. No installed enablement follows
from source tests. See [contract](contracts/CONTROLLER_RECOVERY_V1.md#recovered-namespace-stop-candidate).

## Controller Recovery Reservation

`original controller recovery remains fenced` is not a successful restart. Keep
the original command, launch/endpoint, Base mapping/journals and any activation
checkpoint. An expired `reserved` epoch may still have an unknown native outcome;
do not allocate another command, delete it, extend it manually or restore the old
controller UUID. Readback is historical and does not renew its30-second lease.
The candidate has no public recovery/force-unlock endpoint. Its default-off
custody worker now reconciles the original native command and dual heartbeat;
it does not authorize restored execution. Do not populate the table manually
to enable execution.
See [required native integration](contracts/CONTROLLER_RECOVERY_V1.md#required-native-integration).

## Custody Heartbeat Hold

`controller recovery cycle remains held` does not mean that native work stopped.
Preserve the immutable initial delivery/receipt, current DB lease and private
native journal. A missing reply may follow a committed heartbeat; the next cycle
must catch up the exact stored version before another renewal. Do not delete
history, reset the original dispatch claim, edit a deadline or allocate a fresh
recovery ID to bypass uncertainty. A historical positive ACK cannot revive either
expired lease or attest namespace exit.

Keep `FLEET_CONTROL_FLEET__CONTROLLER_RECOVERY_ENABLED=false` in accepted
deployments until actual Fleet/Base/ongoing-Hermes acceptance. Candidate enablement
requires the compatible original trusted Base source and bridge registration;
it is not a source upgrade or force-adoption command. Disabling the worker stops
future custody cycles after the next Fleet backend restart, not the agent process;
already persisted history and unknown outcomes remain. No model, queue, control
or activation permission is granted by a successful heartbeat. See
[environment](ENV.md#controller-custody-recovery) and
[dual lease contract](contracts/CONTROLLER_RECOVERY_V1.md#dual-lease-heartbeat).

## Controller Restart Observation

### Isolated Custody Acceptance

The owned `scripts/container_supervisor_live/run.py --controller-recovery` mode
uses two actual Docker Hermes agents and a deterministic local model response
that requests a real terminal approval. No approval is granted. It restarts the
same Fleet controller container twice, checks startup heartbeat, waits for real
lease expiry and checks historical readback without renewal. Original agent
containers, launch/PID, dispatch identity and transcript must remain unchanged.

This mode is separate from `--readiness-rollback` and `--log-readback`; mixing
them is rejected before resource creation. The driver requires exact immutable
Rust, Docker CLI, Hermes dependency and PostgreSQL image IDs and clean compatible
Base checkouts. It removes only its own temporary Compose resources in `finally`.
Private runtime configuration and native receipts stay in the protected QA
controller volume; public evidence contains only identifiers, hashes and checks.

A passed custody test is not a resumed model run, approved tool execution, safe
Fleet stop, interrupted activation settlement, PM acceptance or SDLC success.
The final Compose teardown is QA cleanup, not a successful recovered runtime
stop. Keep accepted deployment flags and image pins unchanged. Test evidence
belongs in [verification](CHAT_CLARIFICATION_VERIFICATION.md), not in readiness
or business completion badges.

`Controller restart observed for original agent namespace; ownership transfer
remains required` is degraded diagnostic evidence, not running/readiness or
permission to resume. Preserve the original mapping, Compose, private SQLite,
launch and endpoint records. Do not restore an old controller UUID, rewrite
mapping hashes, kill by PID or issue a replacement generation. The new private
observer is available only from a compatible exact Base source; no installed
pins or rollout flags are changed by this candidate. Full fenced ownership
transfer and interrupted-activation recovery remain open in
[GAP_REGISTER](GAP_REGISTER.md).

## Endpoint Custody Conflict

`Hermes origin is not bound to the current runtime` means the current original
launch/PID, sealed Base endpoint and dispatch generation do not agree. Preserve
the launch, endpoint and command journals; do not edit/delete a sealed row,
replace the origin, mark an unknown submission pending again or retry its POST.
Check original controller/Engine/network custody and durable readbacks first.
Endpoint loss or drift requires reconciliation, not fresh endpoint discovery as
authority. A new generation is permitted only after confirmed old namespace
termination and the normal configuration/start gates. Installed Docker rollout
still requires the remaining recovery acceptance in [GAP_REGISTER](GAP_REGISTER.md).

## Container Configuration Replacement Candidate

Use the existing draft/validate/activate flow; changing a stopped configuration
does not start a runtime. A running agent drains and waits for run/outbox
quiescence before the original controller stops its namespace and starts a
new generation. A readiness failure restores prior files and starts a distinct
rollback generation, not the exited container. 31 focused Linux/PostgreSQL
cases and strict Clippy pass. The subsequent owned real Docker gate verifies
the controlled boot-timeout rollback, exact previous files and loaded SOUL,
peer isolation and once-only follow-up chat; see
[verification](CHAT_CLARIFICATION_VERIFICATION.md#actual-docker-readiness-rollback-6-october-2026).
This does not prove interrupted activation or controller takeover.
Do not enable the installed Docker path from this document.

Unknown prepare/start/stop or a foreign controller retains drain and the private
activation journal. Keep candidate files, pre-create/launch rows and original
Base journals together; do not clear heads, delete fences, rewrite files or
spawn a replacement manually. The standard runtime actions remain blocked by
drain. Complete operator reconciliation/takeover remains an open acceptance
item, not an implemented recovery command. Healthy HTTP alone cannot release
that hold. See [ADR0029](adr/0029-container-configuration-generation-replacement.md).

## Lost Container Preparation Files

With000018, automatic preparation commits an immutable DB fence before Docker
create, including the original controller/generation/operation and private
intent hash. Missing intent or the entire private controller directory must
remain held; do not delete the fence, reset the ordinal, switch to native mode
or generate a new preparation. Restore only the exact original private files
from a protected backup and reconcile Base's original create receipt. Same-
controller readback is supported; replacement-controller takeover and complete
journal/backup restore remain separate acceptance gates. The migration cannot
recover a pre-upgrade unrecorded intent: inventory outstanding preparations
before enabling automatic creation. Never expose secret-bearing files in reports.

## Runtime Ownership

For a journaled launch owned by another controller, health returns observational
degraded without changing persisted runtime/heartbeat/capabilities. Inspect the
original controller rather than treating that response as a failed process or
permission to restart. Its audited observed/persisted statuses stay distinct;
the observational response must not create an `agent_down` transition alert.
The recorded owner must still retain its actual live
child; `try_wait` rejects an exited retained process before dispatch. A numeric
PID, HTTP health or a missing controller handle never authorizes adoption.
Do not clear the launch hold manually to bypass reconciliation. A waited parent
still does not prove that all descendants or remote side effects have stopped.

## Unknown Runtime Launch

With migration `000017`, a controller can leave an original `claimed` or
`gateway_started` record after death or failed acknowledgement. Do not delete it,
edit its binding/PID, reset Ready metadata, downgrade the schema or retry under a
new launch key. A different Fleet process cannot adopt a numeric PID or use HTTP
health to assert ownership. Stop/activation/replacement stays held without the
original child. The owning controller can record its positively observed gateway
exit, but that does not prove safe descendant or remote-job termination.

Operator force-release/recovery is not implemented. Isolated production rollout
requires verified original host-generation/empty-boundary evidence and protected
config state first. See [the launch contract](contracts/RUNTIME_LAUNCH_JOURNAL_V1.md).

## Private Activation Recovery Storage

Before requesting a configuration activation, explicitly provision a dedicated
Linux controller directory outside every agent mount and set
`FLEET_CONTROL_FLEET__CONTROLLER_ROOT` for the Fleet process. The controller UID
must own it, mode0700. Do not reuse arbitrary existing directories or credentials
stores. The application verifies but does not create/chmod/adopt this root.
An empty/missing/unsafe root prevents file/runtime mutations and keeps the claimed
activation drained for reconciliation. Do not resolve this by releasing its DB
claim or retrying under a new revision/key.

Private `<agent-uuid>.activation.json` v2 documents contain original locations,
revision, expected hashes and previous runtime files, including resolved secrets.
Include the directory in protected backups; exclude it from API, logs, agent
mounts and screenshots. Preserve partial/retained documents. A successful DB
result precedes byte-identical acknowledgement; changing owner/mode, linking or
rewriting the document prevents deletion. Another agent has an independent file.

Any `.fleet-activation-journal.json` v1 in agent config remains a blocker: do not
auto-move it to the new directory, delete it, rewrite it or infer a successful
rollback. A reviewed operator recovery must establish original DB/files/runtime
state and safe descendant/remote-effect cessation first. No repair/reset endpoint
or automated takeover is introduced by this packet. Windows ACL durability and
actual Fleet container-boundary integration remain separate release gates.

Installed Compose/mounts/images are not changed. Existing non-activation reads,
legacy chat history and runtime controls do not gain task admission from this
setting. The private root is infrastructure configuration, not a model parameter.
Do not change the root or mount mapping while a journal or activation is unresolved;
an empty replacement is not recovery. A reviewed root relocation must preserve
all original documents and their protected backup provenance.

Original approval outcomes have immutable storage (000016) and an opt-in bounded
GET-only recovery worker, but no public repair/reset/delete endpoint.
Do not manually give legacy uncertainty a context or release a claimed decision
as failed. A late witnessed receipt preserves cancelled request/run history;
it does not permit a new action. Database backups include private contexts.
Empty rollback is tested; any original decision history prevents downgrade.
Installed flags/plugins remain unchanged pending ordered release and native
approval/combined-extension acceptance.

The same default-off control-outcome flag selects original-mode reservation for
new exact-request decisions. A failed preflight keeps an unsubmitted uncertain
receipt; replay does not retry preparation or POST. A saved claim cannot be
released on missing, uncertain or invalid lookup. Changed origin/credential/store
requires reconciliation, not adoption of fresh metadata. Read the public decision
status after lost replies; no new key, bulk decision or direct SQLite edit is safe.

## Runtime Control Reconciliation

Steer/stop requires a stable authenticated actor-scoped `Idempotency-Key`. Read commands through
the session/run controls GET before deciding what happened. Identical replay
returns the stored receipt; changed payload/key scope conflicts. Never rotate a
key to retry an unknown effect. `reserved`, `submitted` and `uncertain` hold new
controls for that run. A process restart does not resend them. A preflight failure
known to precede POST records `rejected`; any unknown POST/ACK remains held.

`acknowledged/stopping` is a request to interrupt, not completion. The periodic
reconciler may retire a held command only against independently persisted accepted
run, prompt delivery and terminal mirror proof. `terminal_observed` means the run
is terminal, not that the command was accepted. It is not OS-descendant quiescence,
assignment release or a successful SDLC receipt. There is no public blind-reset
repair endpoint. Targeted tool approvals remain a distinct protocol.

`FLEET_CONTROL_FLEET__HERMES_CONTROL_OUTCOME_ENABLED` defaults to false. Enabling
it requires the reviewed compatible Base control plugin and a normal validated
config activation, not an in-place HOME change. Original context is saved with
the single-send claim before POST. Only matching authenticated GET witnesses can
restore an unknown ACK. Missing/conflicting/foreign epoch or rotated credentials
retain the original receipt; do not rotate a command key, reset a store or
backfill legacy commands. The worker paginates retained contexts by UUID and
repeats readback, not the original POST. Actor revocation denies new commands but
does not erase an already applied historical effect. Approval decision outcomes
are not covered by this flag. Keep it disabled on accepted installations until
the ordered release and exact-head acceptance complete.

Migration 000013 must follow the released 000012 journal; that release order is
not yet completed. Its dedicated PostgreSQL gate verifies empty down/re-up,
preserved legacy rows and nonempty refusal. Nonempty command history deliberately
blocks downgrade. Preserve history during rollback and use forward fixes rather
than deleting receipts.

Original-key recovery is an [explicit opt-in](contracts/HERMES_RECOVERY_V1.md),
not an installed default. Ship verified plugin files, validate/activate a new
Hermes config revision through normal drain, and enable the separate Fleet flag
only after managed native/Fleet acceptance. Do not rewrite legacy journal facts,
copy a SQLite store into another HOME, purge witness tombstones or rotate keys to
escape uncertain acceptance. Missing/conflict/reset/expiry/status unavailable
retains capacity for reconciliation. Saturation forbids new admissions while
existing lookup remains readable. No public operator reconciliation endpoint or
automatic safe-stop/process-tree proof is introduced by this candidate.

For existing Hermes agents, activate a new renderer-2 configuration through
draft/validate/drain/activate/readback. Do not edit historical snapshot versions,
file hashes or private dotenv in place. A still-active/unknown run must finish or
be safely stopped before activation. Provisioning does not overwrite old files;
the Base `serve -> gateway run` wrapper is still required for managed launch.

## PM Credential Recovery

Credential preparation is opt-in and still ends before admission/dispatch.
Unknown issuance leaves an immutable intent; acknowledged issuance followed by
Tracker failure keeps the child receipt. Use the owner-only original creation
continuation after restoring dependencies. The same command under the same
original parent is replayed, then fresh child/Tracker authorization is checked.
Do not rotate parent, change TTL/origins or invent a new key to clear a blocker.
An expired/revoked child remains historical; renewal/handoff recovery requires a
separate reviewed operation, not editing the journal. Audit shows once-only intent
and ACK, never parent fingerprints, bearer secrets or source response bodies.
Migration 000011 preserves legacy operations and historical 000010. Its downgrade
refuses any retained journal, including pending intent; use reviewed forward
recovery rather than removing records or disabling triggers in a live database.

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

## SDLC Foundation Recovery

Runtime stop/restart returns unavailable if an active runtime has no owned Child
handle, or tracked kill/wait cannot confirm exit within ten seconds. Its PID and
running metadata are not cleared to manufacture success. Do not signal a recorded
PID blindly: it can belong to a different process after reuse. Reconcile actual
runtime ownership before replacement. Parent-process exit does not prove that
all tools/descendants ended; assignment release still requires trusted quiescence.

Lifecycle operations and configuration apply/rollback are serialized per agent
inside one supervisor. Requests waiting behind activation recheck drain; do not
retry them under another key to bypass an activation hold. A failed HTTP health
probe preserves unconfirmed ownership and desired state. Java readiness headers
and body have a three-second bound and 16 KiB ceiling; startup readiness for both
runtime types has a 60-second total deadline, including all probes and sleeps.
Rollback refuses to restore files until any owned replacement process has been
confirmed stopped. None of these local guards implements distributed ownership.
Physical purge also propagates a failed stop before deleting the marked folder
or recording success. This refusal is not an atomic distributed purge lease;
archive/start/health coordination and descendant quiescence remain release gates.

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
Before stopping a tracked runtime or changing managed files, activation creates
`<controller_root>/<agent-uuid>.activation.json` v2 exclusively, persists previous
bytes, canonical source locations, expected file hashes, agent/revision identity
and prior running state, and fsyncs
the file (and parent directory on Linux). An existing or incomplete journal
blocks another activation; it is not overwritten. Backups are limited to 8 MiB
total, 128 files and a 24 MiB serialized journal. Successful application or verified
rollback removes only the identical journal after the activation result commits
in the database. Failed commit/rollback or process interruption preserves it.
Rollback now reads back every restored file, including originally absent files.
On Linux, application and rollback also fsync each managed file's parent after
rename/unlink. Newly created skill directories are synchronized leaf-to-root
under the guarded agents root. File readback alone cannot acknowledge those
directory changes. A failed barrier is dependency-unavailable: effective head
must not advance, and journal/drain remain held if persistence cannot be verified.
Interrupted writes can leave private `.fleet-next-*` files; do not publish them
or remove recovery material to bypass activation. They do not authorize adoption.
The journal contains resolved env secrets encoded as hex, not encrypted/redacted:
keep it private like runtime `.env` (Unix mode 0600); never attach it to a PR,
logs, screenshots or support reports. Do not delete it to bypass a blocked agent.
If activation fails and rollback is unconfirmed, keep the drain in place and
inspect the last error. Crash recovery/operator reconciliation is not yet a public
API; do not edit state rows to fabricate readiness.
This slice preserves restart recovery evidence; it does not automatically reclaim
an interrupted activation or prove OS process-tree quiescence. Windows private
storage ACLs are not certified: activation is held before file/runtime effects.
Linux is the private-storage and durability gate.

An `uncertain` dispatch with a consumed submission permit may have been accepted
by Hermes. Never re-send it or clear its capacity hold. Investigate runtime
session/run IDs and acceptance before recovery. EOF without a terminal status
remains waiting. A journal still prepared has not consumed that permit; only the
guarded worker described below may perform its one initial submission.

The private `hermes_dispatch_journal` distinguishes prepared intent from a
consumed submission permit and accepted ACK. Do not copy its original prompt,
request bytes/hash or credential fingerprint into support reports. Do not
change state/key/origin/deadline, delete rows, rotate credentials as a retry
workaround or downgrade a nonempty journal. Existing tokens need no DB backfill.
Known accepted pending and pinned active recovery uses GET only after original-context checks;
missing legacy journal or changed port/token retains history and capacity without
HTTP. Unknown acceptance keeps pending delivery plus an error, not confirmed
failure. Public operator reconciliation is not implemented yet. A retention
margin, durable=true, 401/404 or an empty/reset runtime store never authorizes
another POST under this or a new key.

Prepared restart recovery scans at most20 records per five-second keyset cycle.
It verifies original bytes/key/origin/credential, fresh health/protocol and the
original optional recovery epoch before the transactional submission claim.
The claim rechecks current identity, drain, capacity and DB deadline; one winner
may submit original bytes, and only that transaction may move a prepared
uncertain outbox to dispatching. A changed/unavailable prerequisite leaves the
permit untouched and records only a generic warning. Submitted/accepted/legacy,
failed, expired, archived, drained and task/PM records are not adopted. Do not
edit journal rows to manufacture a prepared state or renew a horizon. A crash
after permit commit but before HTTP remains unknown, not permission to retry.
See [ADR 0020](adr/0020-prepared-dispatch-restart-recovery.md).

After pin-to-worker crash, the recovery loop reads the original native run; it
does not open a replacement event stream. A valid terminal GET atomically settles
run/delivery/optional assistant. Running/waiting/stopping, including drained
agents, remain observed; invalid status or lost runtime retains capacity. Missing
native tool/approval history is not reconstructed. Runtime completion is not a
safe process-tree stop or a Tracker stage receipt.

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

The native process path captures stdout/stderr into `agent_logs` and applies
generic secret-marker redaction before persistence. This is not production
Docker log ingestion or proof of exact resolved per-launch secret redaction.
The container candidate currently provides private bounded Base log readback;
generation-bound collection, durable replay/cursors, rotation/gap handling and
redaction of the original resolved credentials remain required before rollout.

`/logs` has process logs, events and audit tabs. Use audit for role changes,
settings changes, skill/config edits, runtime actions, handoff and delegation.

## Recovery

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
marks an untracked responsive Hermes runtime degraded, not tracked/runnable.
An HTTP failure does not prove process death; a previously running/unconfirmed
runtime remains degraded and replacement/config activation must wait for
reconciliation. PID visibility and parent exit are not descendant quiescence.

GET-only accepted free-chat recovery can restore the current exact pending
approval after Fleet restart. It requires original journal/origin/credential,
native run/session pins and fresh capabilities. The owner decides through the
normal exact-request API; do not create a replacement run or rewrite native
SQLite. Missing capabilities/context and unknown decision ACKs retain their hold.
Historical approvals/tool events are not reconstructed from the status snapshot.

Migration000014 preserves logical journal timestamp order across wall-clock
regressions; it does not correct clocks or renew retention. Keep the original
deadline/key, investigate clock synchronization and do not remove the guard or
renew a submitted permit. Nonempty downgrade requires explicit reconciliation.
See [clock evidence](CHAT_CLARIFICATION_VERIFICATION.md#journal-clock-order-repair).

Use idempotency keys when retrying session/message create calls. If the previous
payload differs, the API returns `409` and the operator should create a new
intent instead of replaying the old key.

## OIDC authentication mode

`auth.mode=oidc` (см. docs/ENV.md): access-токены валидируются как RS256 против JWKS провайдера (кэш в памяти, refresh по интервалу и при неизвестном `kid`); `iss`/`aud` проверяются строго; HMAC-токены и локальный логин (`POST /api/v1/auth/login`) отклоняются — перевод на режим требует выданных провайдером токенов. Роль FC берётся из `oidc_role_claim` (admin→Admin, operator/maintainer→Operator, иначе User). Legacy-фоллбек компакт-токенов в oidc-режиме не действует.
