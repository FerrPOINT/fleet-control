# Hermes Adapter Contract

PM callbacks read only the reservation's sealed original origin and credential
fingerprint, with matching launch/controller and exact native run/session. The
supervisor retains lifecycle exclusion through observation commit and checks
physical custody after blocking database locks, without nested endpoint writes,
pool reads or waiting on busy global maps. Missing legacy binding stays held;
terminal replay preserves timestamps/events. See
[ADR0036](../adr/0036-original-pm-runtime-proof.md); this is not PM dispatch authority.

Pinned Base preparation seals six native skill discovery/disable settings;
effective validation denies missing or changed policy and every HOME/skills file
other than the pinned package's materialized `<name>/SKILL.md`, even support files under an
allowed skill directory. Unix canonical hard links fail closed without mutation.
This is not a full loaded plugin/tool inventory
or first-step receipt. [Policy and upgrade](../RUNTIME.md#sdlc-skill-discovery-policy).

Private controller recovery storage follows
[Controller Recovery V1](CONTROLLER_RECOVERY_V1.md). Any outstanding epoch
fences original-owner Hermes delivery and generation operations, even after a
stored ACK. Native takeover, original-key outcome readback and fresh DB/native
lease authority are required before the recovered owner may dispatch or control
a run. Storage/component tests are not actual Hermes recovery acceptance.
Candidate000021 connects native handover/original-key ACK through a trusted
supervisor entry, without enabling new-owner model/control effects. Missing ACK
after the one committed claim stays held; expiry never permits another dispatch.
Dual live-lease heartbeat, automatic startup and actual ongoing-Hermes acceptance
remain separate release gates. No Hermes SQLite write or public endpoint is added.

Automatic container launch input now includes a private exact dotenv snapshot
before create, frozen by the preparation intent hash and checked before start.
Native Hermes dotenv interpolation, credential sanitization, external sources,
managed overlays and reloads are not attested by that input snapshot. Resolved
secret redaction remains a blocker for the production Docker log collector.
No Hermes wire protocol, delegated PM credential persistence or public API
changes follow from this input-custody guard; Java behavior is unchanged.

The opt-in [container consumer](CONTAINER_CONTROL_V1.md) uses the original Base
registration/ACK/bridge endpoint for all Hermes requests, including health,
capabilities, dispatch, readback, streams and controls. The public adapter
protocol is unchanged. Gateway generation comes from the immutable Fleet
launch and observed original running namespace, not from Hermes-supplied fields
or a newly discovered PID/address. Unknown start remains held and cannot fall
back to native launch or loopback HTTP. Base endpoint validation alone proves
neither network reachability nor loaded configuration, model access or SDLC
readiness. Automatic Compose generation preparation now saves the original
private intent and consumes Base's never-started preparation receipt before DB
claim/start. It uses /config and /workspace inside the container and binds only
the isolated bridge, without published ports. With an explicit trusted Fleet
controller, the new private mapping resolves local AgentPaths to original
named-volume subpaths and binds proof/file/digest before start; it does not use
daemon-root binds or rslave fallback. Every endpoint/observe/control-lifecycle
lookup retains that original proof. Actual Rust Fleet UID/file/model access,
isolated chat, configuration drain/replacement and restart pass the controlled-model
[container acceptance](../CONTAINER_SUPERVISOR_ACCEPTANCE.md). Controller crash,
private-journal loss and readiness-failure rollback still require live acceptance.
Confirmed original namespace exit permits a new history-ordinal generation;
unknown preparation/start keeps its original intent/claim and cannot advance it.
Previous generation files are preserved, not rewritten or restarted.
The000018 pre-create DB fence commits the immutable ordinal/controller/generation/
operation/intent hash before file creation and Base prepare. Neither missing
intent nor whole private-directory loss authorizes another create or native
fallback. Resolved credentials remain private files, never database columns.
Original file restore/readback is distinct from unimplemented controller takeover.

Retained-child checks below apply to the legacy native path. Container config
activation uses original namespace stop before file effects and a new generation;
unknown custody retains drain and recovery evidence. Its rollback has focused
PostgreSQL/fake-Base tests, not actual Docker failure-injection acceptance.
Native evidence does not certify the container path.

Additive000019 seals the Base-verified bridge origin to the original started
launch and PID. Dispatch preparation and submission require that exact origin
and private Fleet launch ID; an arbitrary private address is not authority.
Endpoint history is immutable and blocks downgrade until reconciled. This adds
no task/PM admission, new public endpoint setter or fallback to localhost.

The automatic queue selector now respects the original acknowledged controller
in the [launch journal](RUNTIME_LAUNCH_JOURNAL_V1.md). Foreign controllers leave
pending delivery unchanged instead of claiming and failing another process's
prompt. Actual submission still requires retained child custody and the pinned
generation; no process adoption or unknown-command replay is introduced.

Retained-child verification observes `try_wait` before using its cached PID:
an unreaped exited native child cannot authorize generation or HTTP dispatch.
This is a current process observation, not host boot/config/descendant proof.
A foreign controller's managed health response is degraded/read-only and cannot
overwrite the original owner's runtime status, heartbeat or capabilities.
The Fleet health request audit remains; nonpersisted degradation does not
generate an `agent_down` alert.

Additive000016 implements original approval context/claim and atomic historical
ACK completion, separate from the legacy pending-request lifecycle. Context is
fixed before a possible effect; old uncertain decisions never acquire it later.
The opt-in adapter now calls these methods for exact-byte approval POST and
original-context GET recovery. Legacy behavior and installed flags are unchanged.
The native `approval-outcomes` case verifies once/deny and lost-ACK GET recovery
through the committed control plugin. Separate `approval-restart` verifies two
SIGKILLs/three Fleet PIDs with one surviving gateway and original GET-only decision
settlement. Combined extensions and installed release remain acceptance gates.

The [control outcome extension](HERMES_CONTROL_OUTCOME_V1.md) now has a Rust
wire-consumer with closed capability/context/ACK validation and an
internal additive000015 stop/steer context/ACK journal. The default-false
supervisor sends saved bytes/UUID/epoch headers after claim and runs bounded
GET-only recovery. Approval decisions remain separate; no automatic enablement
or task authority is added. Private context never appears
in public receipts, and positive ACK does not erase an observed terminal fact.

Current pending approval readback is GET-only and limited to the original
accepted pinned free chat; it alone cannot prove historical decision delivery.
That separate default-off consumer requires the original witness, not a current
pending request or terminal run. See [ADR0022](../adr/0022-current-native-approval-snapshot.md).
Additive000014 floors logical journal progress after the existing guard while
preserving original horizon/key/permit. It is not native retention or trusted
clock proof; see [ADR0023](../adr/0023-logical-journal-progress-time.md).

Fleet consumes SSE through the bounded
[event-stream profile](HERMES_EVENT_STREAM_V1.md): strict HTTP/MIME/unencoded JSON,
incremental UTF-8, completed frame delimiters, original identity and independent
frame/text/snapshot/connection budgets. Invalid/expired input keeps the accepted
run and capacity held; EOF requires original-run status proof, never frame flush.
Worker retirement is not run stop, upstream event replay or task admission.

The opt-in [managed native gate](../../scripts/native_supervisor_live/README.md)
tests the actual Base launcher, Fleet lifecycle/activation/dispatcher and two
native gateway processes. Its local-model happy path checks separate HOME/cwd,
ports/SOUL/token, prompt replay and native run status after restart. It changes
no public protocol. Tracked parent exit is not process-tree quiescence; managed
lost-ACK recovery, tools/approvals and task/PM admission remain separate gates.

Prepared journals may recover the one original POST through their still-unused
transactional permit, with exact saved bytes/key/context and fresh protocol
facts. Pending delivery remains pending when its saved prepared intent encounters
a pre-submission error; existing failed delivery remains terminal. For managed
launches, retained original child/generation verification precedes native probes.
Only after that verification is the private `fleet_launch` binding included in
the comparison with freshly normalized native protocol facts. Hermes metadata
cannot supply or change that binding. Claim and actual submission recheck it.
This is not POST replay for submitted/unknown acceptance. Readback paths
remain non-dispatch; task/PM records require their independent admission.
See [ADR 0020](../adr/0020-prepared-dispatch-restart-recovery.md).

Opt-in original-key recovery uses [extension v1](HERMES_RECOVERY_V1.md).
Verified facts are frozen before the first POST; epoch header guards original
native admission. Lookup only restores the witnessed original run ID and does
not supply terminal/session/admission evidence. DB-clock expiry is checked at
the atomic mapping update; missing/changed facts retain capacity with no POST
replay. Legacy records are not backfilled; default flag is false.

Configuration renderer version is part of the frozen snapshot. New Hermes
drafts use 2 and persist the native API listener plus protected dotenv values;
legacy/absent version 1 retains old bytes/hash interpretation. Unknown versions
are refused without fallback. Provisioning creates only missing files, never
rewrites effective history. Native loader equality is not loaded-runtime admission.

Supervisor stop retains process ownership until kill/wait confirms exit. Unknown
untracked/recorded-running state is not converted to stopped, and restart cannot
spawn a replacement through that error. A Hermes `run.cancelled` event remains
distinct from process wait and complete process-tree quiescence.

Activation reserves `<controller_root>/<agent-uuid>.activation.json` v3 before
stopping a tracked Hermes or changing files. The operator must provision this
Linux-owned `0700` directory outside the agents root and every agent mount;
Fleet does not create, chmod or adopt it. Previous bytes, expected hashes,
canonical locations, exact candidate/effective snapshot hashes and original
controller/launch are protected, size-bounded and HMAC-authenticated.
Missing/unsafe storage or a legacy `config/.fleet-activation-journal.json`
keeps drain without moving, rewriting or deleting recovery evidence.
Rollback verifies every old file;
unknown outcomes preserve journal/drain. Cleanup follows the committed DB result,
not an HTTP success or EOF. The default-off signed recovery worker can restore
stopped agents or acknowledge committed files after matching readback. Running
rollback needs original namespace exit and fresh rollback readiness; unknown
acceptance and unsigned v1/v2 remain held. No unknown command is resent and no
descendant stop or SDLC readiness is inferred from HTTP health. Actual running
Hermes crash acceptance remains a separate gate.
This storage contract alone cannot prevent access by an agent using the
controller's OS identity; runtime/container isolation is a separate gate.
Managed apply/rollback now require Linux parent-directory fsync after rename and
unlink, plus leaf-to-root synchronization of newly created directory entries.
Unconfirmed persistence retains journal/drain and does not promote effective head.
Protected temporary files may survive interruption; they are not authority.

Hermes is the first implemented runtime.

## Совместимый запуск

На pinned Hermes `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3` raw
`hermes serve` запускает dashboard/headless web server, не gateway API
`/v1/runs`. Текущий Fleet argv требует Base compatibility wrapper из
`services-base/deploy/fleet-hermes-launch.py`. В packaged image он установлен
как `/opt/fleet-hermes/bin/hermes`; именно его должен выбирать managed setting
`FLEET_CONTROL_FLEET__HERMES_COMMAND`. Raw CLI с тем же именем несовместим
с текущим launch contract.

Wrapper принимает Fleet `serve --host 127.0.0.1 --port <api_port>`, проверяет
loopback и порт `1024..65535`, экспортирует `API_SERVER_HOST` /
`API_SERVER_PORT` и выполняет `/opt/hermes/.venv/bin/hermes gateway run`.
Не-`serve` команды передаются исходному CLI без преобразования.

Environment:

- `HERMES_HOME=agentN/config`
- `API_SERVER_ENABLED=true`
- `API_SERVER_KEY=<derived per-agent token>`
- `HERMES_SERVE_HEADLESS=1`

Новый renderer v2 также закрепляет `API_SERVER_HOST` / `API_SERVER_PORT` в
защищённом dotenv и native YAML. Исторический v1 допускает эти ключи из
`env_json`; Hermes загружает `.env` с `override=True`. Поэтому старые env/argv
не доказывают native bind, а renderer v2 сам по себе не доказывает loaded config
работающего процесса. Исторические immutable snapshots,
markers и ожидаемые managed bytes сохраняются без молчаливой перезаписи.
`HERMES_SERVE_HEADLESS` в Fleet env не меняет назначение raw CLI команды.
Ни wrapper, ни эта документация не закрывают native acceptance или admission.

Working directory:

- `agentN/workspace`

Managed files:

- `config/config.yaml`
- `config/SOUL.md`
- `config/.env`
- `config/skills`
- `runtime/source.json`

Lifecycle:

- start: configured Base compatibility wrapper with
  `serve --host 127.0.0.1 --port <api_port>`
- stop: terminate tracked process
- restart: stop then start
- health: reconcile tracked process state through `/health`
- readiness: `/health` plus `/v1/capabilities` containing `run_status`,
  `run_events_sse` and `run_stop`

These lifecycle checks are not SDLC admission. Pinned Base effective readback
verifies the immutable Git snapshot and a bounded closed HOME skill-file tree;
it does not certify plugin/project/external discovery, loaded model/tool settings
or a frozen assignment. See [runtime boundaries](../RUNTIME.md). Missing native
proof keeps `runtime_ready=false`; no fallback or extra skill deletion is used.

For Base package activation, the supervisor compares the frozen
[Workflow mapping](SDLC_WORKFLOW_BINDING_V1.md) to fresh owner metadata before
stopping Hermes or writing files. Declared profile and numeric namespace IDs
are not native-loaded profile proof. A readback outage before mutation releases
drain after recording a failed revision; unverified rollback remains drained.

Native stdout/stderr writes use the shared repository's exact persisted-row
acknowledgement after redaction. See [Logging Standards](../LOGGING_STANDARDS.md).
This does not implement production Docker collection, replay or secret snapshots.

Session control:

- Fleet stores transcript/control mirrors in `session_messages`.
- Fleet dispatches through the runtime supervisor boundary.
- Fleet creates Hermes runs with `POST /v1/runs`, `input` and
  `session_id=fleet:<session_id>:<agent_id>`.
- Before POST, Fleet atomically reserves the concrete run and immutable exact
  request journal, then consumes a single-send permit under row locks. The
  fixed unprefixed endpoint uses native `default` profile and no session-key
  override header; the credential fingerprint includes that context. HTTP sends
  the journal bytes without reconstructing input/model/options. Free chats also
  require fresh durable 86400-second idempotency and matching wire endpoints.
- A verified ACK commits native ID, prompt/outbox and journal `accepted` together.
  Pending and pinned recovery compare original origin/fingerprint before GET; no
  journal, changed token or moved port requires reconciliation without HTTP.
  Legacy history stays readable, but credentials are not backfilled.
- Unknown POST acceptance holds capacity and pending delivery with an error.
  Neither an identical key nor a recovery deadline permits automatic replay:
  this native baseline exposes no non-dispatch HTTP key lookup or store epoch,
  and a reset SQLite can still advertise durable storage. An owner-supported
  positive lookup is required to recover the original native ID safely;
  negative lookup/404/expiry must never authorize resend.
- Fleet mirrors events from `GET /v1/runs/{run_id}/events`.
- After a pin-to-worker crash, recovery observes the existing run by GET only.
  It does not reattach an SSE queue or resend a prompt. Pinned session identity
  stays immutable; a nonterminal, missing or invalid status retains capacity.
  Terminal run/prompt/optional assistant and durable events commit in one
  transaction; exact replay performs no writes, contradictory evidence conflicts.
  Empty successful output creates no fabricated assistant. Old stream progress
  cannot append delta/tool/approval effects after terminal commit. Details:
  [ADR 0018](../adr/0018-atomic-terminal-pinned-recovery.md).
- Terminal names come from the explicit SSE header or root `event`, never a
  nested tool/subagent discriminator. Only exact run terminal events can end
  the accepted run; they must contain its `run_id`. Successful completion requires
  `completed=true`, `partial=false`, `interrupted=false`. After durable terminal
  persistence, Fleet stops consuming the stream instead of allowing late events
  or transport errors to regress the state.
- On EOF without terminal evidence, authenticated HTTP 200 status readback must
  identify `object=hermes.run` and the accepted `run_id` with native status/flags.
  JSON is limited to 1 MiB with identity encoding; unsupported aliases or invalid
  proof keep the run waiting and capacity held. PM also matches the acknowledged
  effective session. These observations do not prove process-tree quiescence or
  successful Workflow/Tracker completion; see [runtime boundaries](../RUNTIME.md).
- Fleet forwards verified controls to `/v1/runs/{run_id}/steer` and
  `/v1/runs/{run_id}/stop` under the [bounded consumer profile](HERMES_RUN_CONTROL_V1.md).
  Original accepted journal and pinned native status are mandatory; ACK is not
  terminal or safe-stop proof. Fleet requires an authenticated server-derived actor and bounded
  `Idempotency-Key`, persists immutable control context and consumes a durable
  single-send permit. Unknown POST/ACK remains `uncertain`; exact replay returns
  that receipt without native POST. A verified terminal mirror only resolves the
  hold as `terminal_observed`, without claiming command acceptance. Session/run
  controls GET exposes scoped readback without raw input, keys or credentials.
  The retired run-wide approval adapter method fails
  closed. Only exact targeted human decisions use `/v1/runs/{run_id}/approval`.
  They require the same original accepted free-chat context, fresh exact native
  capabilities and the pending request in its pinned native session. Unjournaled
  and task/PM context cannot authorize the POST. ACK requires exact200/JSON/
  identity encoding/64KiB bounds; preflight failure retains the reserved hold
  without retry. This is not native configuration-generation attestation.
- For executor sessions, the runtime dispatch target is the primary executor
  even when the mirrored message author is the selected leader.
- Fleet must not write directly into Hermes SessionDB.
- Programmatic chat/control uses the native gateway HTTP API reached through
  the compatibility wrapper, not raw Hermes serve/dashboard JSON-RPC.
- Dashboard remains a separate UI surface and is not the source of truth for
  Fleet message writes.

Current waiting approval recovery uses only the authenticated pinned run status
GET plus fresh capabilities. The nested event must identify the exact run and
nonempty request ID (maximum256 bytes), expose once/deny and a bounded action
description; an optional native session reference must match. The original
accepted free-chat journal and current primary agent are rechecked atomically.
Replays preserve resolved/stopping/terminal state and do not generate transcript
messages or another SSE. Only the currently visible native request is recoverable;
historical approvals/tool events remain open. Unknown decision ACK recovery is
separate: original-mode durable contexts settle through verified native witnesses,
including the native `approval-outcomes`/`approval-restart` evidence above. Current
pending snapshots do not prove those decisions or backfill legacy uncertainty.
