# Runtime

## Native Run Controls

The new opt-in Base [control outcome producer](contracts/HERMES_CONTROL_OUTCOME_V1.md)
records the original command before one native handler and exact ACK before
transport. GET-only outcomes survive producer restart; missing/uncertain never
authorize another effect. Production Fleet consumer integration is still
required. Do not enable the plugin on installed agents or retrofit old receipts.

The Rust GET wire-consumer prepares an opaque original context and validates
bounded exact ACK readback; the wire module contains no POST or state mutation.
The internal repository now binds original stop/steer context to the single-use
claim and commits positive ACK/receipt/audit/event atomically (migration000015).
The supervisor does not yet call that path or run GET outcome recovery, and
approval decision integration remains pending. A deserialized context is revalidated before HTTP and
is not a human/machine authorization proof. See the outcome contract above.

The [control profile](contracts/HERMES_RUN_CONTROL_V1.md) separates acknowledgement
from terminal proof. Stop/steer require fresh Fleet/native identity, original
accepted journal/origin/credential context and advertised native capabilities.
HTTP200 JSON acknowledgements are bounded to64KiB/ten seconds; invalid/unknown
outcomes never retry or release capacity. Steer does not write running over a
concurrent state; stop ACK only sets stopping. A terminal race leaves final
mirror/commit to readback. Run-wide approval is retired inside the adapter too;
exact human decisions remain a separate flow. The additive control ledger now
reserves/claims commands before effects, replays identical keys without a second
POST and preserves submitted/uncertain holds across supervisor restart. ACK and
stopping commit with audit and durable events. Reconciliation observes only an
independent terminal mirror, never inferring unknown guidance acceptance.
Targeted approval now shares the original accepted free-chat context guard;
fresh capabilities and GET verify the waiting exact request in the pinned native
session before POST. Legacy/task context cannot substitute for admission.
ACK requires exact200, JSON MIME, identity encoding and64KiB/ten-second bounds.
A reserved preflight hold stays uncertain without automatic resend. This does
not attest loaded configuration generation. Current waiting-request GET recovery
is implemented separately; historical approval/unknown-decision recovery,
installed approval acceptance and safe process-tree stop remain gates.

## Bounded Hermes Stream

The [event-stream consumer profile](contracts/HERMES_EVENT_STREAM_V1.md) defines
strict authenticated HTTP/MIME/identity framing and resource/deadline budgets.
UTF-8 is decoded at complete lines, not individual network chunks. Unfinished
frames are not flushed at EOF, empty frames cannot leak an event name and invalid
JSON never becomes a tool/approval mirror. Frame, total input, event count,
transcript and cumulative delta snapshots are bounded. A limit retires only the
consumer; it does not stop the native run or release capacity. Independent
authenticated GET terminal proof remains necessary. Native event replay/control
reconciliation and task admission are separate gates, not supplied by the codec.

## Managed Native Acceptance Scope

The opt-in [native supervisor gate](../scripts/native_supervisor_live/README.md)
exercises real Fleet provisioning, renderer-2 activation, prompt dispatch and
transcript persistence with the Base launcher and two pinned native gateways.
Only model inference is a deterministic local fixture. It observes each loaded
SOUL, process HOME/cwd, assigned port, token boundary and persisted run identity
after restart. Private dotenv ownership is checked under a non-root QA identity.

This is not the installed runtime, complete native config/plugin attestation,
OS isolation or descendant safe-stop certification. Successful HTTP readiness,
tracked parent stop and terminal free-chat output do not grant task/PM admission.
The separate `recovery` scenario now verifies a real lost `202` followed by exit
of the dispatching Fleet process and recovery in a distinct Fleet process. The
actual Base witness lookup restores the original already-terminal run; request
context stays immutable and native observations require one POST/inference and
no SSE subscription. See the [verification ledger](CHAT_CLARIFICATION_VERIFICATION.md#managed-native-lost-ack-recovery-4-october-2026).
Running/tool/approval recovery and safe orphan ownership transfer remain separate
gates. These ignored tests are not run by ordinary cargo tests.

## Original-Key Recovery Candidate

The opt-in [Base native extension](contracts/HERMES_RECOVERY_V1.md) records the
original reservation witness in the same SQLite transaction, not SessionDB.
Fleet freezes verified scope/store epoch facts in its existing private dispatch
journal and requires the epoch header on the original POST. A submitted unknown
acceptance can only use non-dispatch lookup, then atomically commit that original
ID within the DB-clock horizon. Native GET pins the actual session and supplies
terminal evidence. No recovery path submits another run or uses negative lookup,
expiry, reset or EOF as permission to release capacity. Legacy intents remain
held; installed enablement and task admission remain independent gates.
[ADR 0019](adr/0019-native-original-key-recovery.md) records
the boundary. This protocol change adds no public Fleet route or migration.

New Hermes snapshots use [renderer 2](adr/0017-versioned-native-hermes-renderer.md):
native `platforms.api_server.enabled`, loopback host/assigned port in config and
protected host/port/credential/HOME/CORS in dotenv. Launcher env alone is not
authoritative after Hermes dotenv loading. Version 1 preserves historical
config/env/marker bytes; existing files are never upgraded during provisioning.
Use normal drain/activation/rollback for a new revision. Native loader evidence
does not substitute for tracked-process/effective-revision/assignment attestation.

Runtime health is not SDLC readiness. Fleet returns
`workflow_assignment_protocol_not_verified` until the assignment, workflow
step/rebind and receipt protocols are verified. See
[SDLC implementation](SDLC_IMPLEMENTATION.md) for remaining runtime gates.

Runtime contract:

- `provision`
- `start`
- `stop`
- `restart`
- `health`
- `open_session`
- `send_message`
- `stream_events`
- `list_capabilities`

Hermes:

- Folders: `runtime`, `config`, `workspace`, `logs`.
- Env: `HERMES_HOME=<agents_root>/agentN/config`,
  `API_SERVER_ENABLED=true`, derived per-agent `API_SERVER_KEY`.
- Cwd: `<agents_root>/agentN/workspace`.
- Managed launch: Base compatibility wrapper with
  `serve --host 127.0.0.1 --port <api_port>`; not the raw upstream CLI.
- Readiness requires `/health` and `/v1/capabilities` with `run_status`,
  `run_events_sse` and `run_stop`.
- Message dispatch uses `POST /v1/runs`, Fleet session ids formatted as
  `fleet:<session_id>:<agent_id>`, and `GET /v1/runs/{run_id}/events` for SSE
  mirror updates.
- Runtime controls use `/steer`, `/stop` and `/approval` endpoints when the
  capability matrix allows them.
- Dashboard is an operator link, not the write channel for messages.
- An unavailable API does not prove an untracked runtime exited. Tracked or
  unconfirmed active/PID/desired-running state remains degraded with ownership
  metadata preserved. Stop/restart cannot replace it without proof. Only an
  already inactive observation can remain stopped; HTTP health is not quiescence.
- Prompt outbox is transactional. Unknown POST acceptance is not automatically
  retried; the agent remains occupied pending reconciliation.
- Free-chat submission now requires fresh durable wire capabilities and the
  private immutable journal described in the
  [data model](DATA_MODEL.md#hermes-dispatch-journal). Concrete run reservation
  and exact request/key/origin/default-profile credential fingerprint commit
  together, then a one-winner submission permit commits before HTTP. The client
  sends saved bytes, not reconstructed prompt/model/options. An unknown response
  keeps pending delivery with an error; neither the original key nor the fixed
  recovery horizon permits another POST. An independent prepared-intent worker
  recovers only an unconsumed original permit, never a submitted/legacy record.
  It verifies frozen request/context and fresh protocol facts, then claims the
  existing run under drain/capacity/deadline locks. A prepared uncertain outbox
  becomes dispatching only with that permit. Concurrent workers have one winner;
  a crash after permit consumption remains unknown, not permission to resend.
  [ADR 0020](adr/0020-prepared-dispatch-restart-recovery.md) defines the boundary.
  Compatible positive original-key lookup is separate; operator reconciliation
  and managed runtime acceptance still remain release requirements.
- A verified HTTP 202 in a free chat now commits the run ID, prompt delivery and
  outbox acceptance together before any status GET. The run remains `pending`
  until an authenticated, bounded status read identifies its effective Hermes
  session; the requested `fleet:<session>:<agent>` alias is not that proof.
  Readback failure retains this ACK and agent capacity. A bounded keyset worker
  retries only GET for journal-backed pending and already pinned active runs after
  Fleet restart; original origin/fingerprint must match before HTTP. Legacy ACKs
  without that proof retain history/capacity but are not automatically probed.
  It never submits a prompt. The first transactional session pin starts the
  stream; identical concurrent pins do not start another worker. Effective
  session and runtime run IDs are immutable, and late generic updates cannot
  regress a terminal state. EOF status must also match that pinned session.
  Delivery updates serialize under a message row lock: an error with no native
  ID cannot erase a committed ACK, conflicting IDs fail, and terminal deliveries
  do not reopen. Controls reload the current run identity before HTTP; a pending
  session pin rejects stop/steer/approval even when a caller holds an old running
  snapshot. This is an explicit temporary limitation during readback outages,
  not confirmed cancellation. Independent acceptance/pin/control journaling is
  still needed to safely stop a known accepted run during such an outage.
  Task-bound/PM runs retain their separate authority and are not admitted by
  this free-chat recovery path. Source tests and live acceptance are recorded
  separately in the verification ledger. A pin-to-worker crash now recovers by
  status GET without another SSE attachment. Validated terminal run, prompt
  delivery, optional assistant and durable events commit atomically. Exact replay
  changes no timestamps/cursor; empty output creates no synthetic reply. Old
  SSE workers observe persisted terminal state; late delta/tool/approval writes
  are guarded under row locks. A crash before ACK commit or an unknown run ID
  still needs further durable recovery. The journal preserves evidence without implementing unknown-key
  recovery, retention-safe redispatch or process-tree quiescence. Native caps
  have no store epoch and even a new empty SQLite can advertise durable=true;
  a missing record/404 is not permission to recreate a run.
- The managed HTTP client disables implicit retries, redirects and environment
  proxies. Run submission accepts only HTTP 202 with a safe opaque `run_id`, a
  boolean `replayed` and a known `status`; a new run must say `started`. A replay
  preserves the original run identity, not a successful stage result. Rejection
  bodies and transport diagnostics are not exposed as dispatch errors.
  Capability JSON is bounded to 256 KiB, acceptance JSON to 16 KiB and PM status
  readback to 1 MiB, including streamed bodies; encoded responses are rejected.
- Before preparing a task-bound run, Fleet freshly verifies the concrete agent
  binding and the authenticated server-agent capability contract: exact run
  endpoints and durable idempotency with the pinned 86400-second retention.
  Memory-only fallback is refused before run preparation or prompt submission.
  This necessary wire prerequisite does not attest native skills/configuration
  or admit an assignment. The finite retention is not permission to redispatch
  unknown acceptance; lookup by idempotency key and full admission remain gaps.
- Stream EOF is not completion. Fleet requires a terminal event or terminal
  status readback, and deduplicates the final mirror response.
  Only exact `run.completed`, `run.failed`, `run.interrupted`, `run.cancelled`
  or `run.stopped` events are terminal; nested/subagent events, generic `done`
  and cancellation requests are not. Terminal payloads must identify the accepted
  run. Success additionally requires the native `completed=true`, `partial=false`,
  `interrupted=false` flags. After EOF the authenticated status read must be
  HTTP 200, bounded to 1 MiB, unencoded and identify `object=hermes.run` and the
  exact accepted `run_id`; `succeeded` is not an alias. Invalid or non-terminal
  evidence keeps the run waiting and its capacity held, without a fabricated
  assistant reply. This is run-state evidence, not OS/process-tree quiescence,
  Workflow completion or authorization for a Tracker stage transition.
- Configuration is draft/validated/activating/active/failed with desired and
  effective revisions. Activation drains runs and checks files/runtime before
  releasing the agent. Failed rollback keeps the agent drained.
  Workflow rebind and actual role/namespace/workflow identity changes share the
  agent row lock with runtime reservations. Active/pending/stopping runs, queued
  or unknown prompt dispatch and configuration drain reject changes with 409.
  Metadata edits with unchanged identity remain possible outside drain.
- Effective configuration readiness now reads the actual managed files on each
  request using the persisted effective head, independently of the last-100
  configuration history page. Validate/activate also use exact agent/revision
  lookup; only the current desired validated revision can activate. It compares
  `config.yaml`, `SOUL.md`, `.env`, enabled/disabled skills
  and the revision marker against the persisted effective snapshot, not against
  hashes or paths provided by a marker. Missing/changed managed files,
  re-enabled disabled skills, foreign markers and symlink/junction paths fail closed.
  Pinned Base revisions additionally verify the actual Git package/snapshot with
  bounded async Git IO (5 s/process, 10 s/package, fixed blob/batch size ceilings);
  failures remain sanitized and never fall back to HEAD or network. Closed HOME
  skill inventory includes nested/unlisted skill files and special
  entries (4096 entries / 16 levels maximum). No unexpected files are removed.
  Legacy revisions keep managed-only behavior; `.bundled_manifest` is never
  authority. Plugin/project/external discovery and loaded model/tool settings
  remain outside this observation; the separate
  `runtime_skill_inventory_not_verified` blocker remains until real inventory
  and native provenance are integrated.
  Verification does not create or repair directories/files. Secret values and
  hashes are not returned. This observation is not a fenced admission, proof of
  runtime-loaded configuration, or task-specific deployment workspace receipt.

## Контракт запуска Hermes

Для pinned Hermes `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3` исходная команда
`hermes serve` запускает dashboard/headless web server, а не gateway API
`/v1/runs`. Текущий Fleet argv совместим только с Base wrapper
`services-base/deploy/fleet-hermes-launch.py`, установленным в packaged image
как `/opt/fleet-hermes/bin/hermes`. Managed setting
`FLEET_CONTROL_FLEET__HERMES_COMMAND` должен указывать на этот wrapper;
имя `hermes` в PATH само по себе не доказывает совместимость.

Wrapper проверяет `--host=127.0.0.1` и порт `1024..65535`, задаёт
`API_SERVER_HOST` / `API_SERVER_PORT` и запускает
`/opt/hermes/.venv/bin/hermes gateway run`. Остальные команды передаются
исходному CLI без преобразования. `HERMES_SERVE_HEADLESS` остаётся частью
текущего Fleet env, но не превращает raw `serve` в gateway API.

Открытый gap: Hermes `hermes_cli/env_loader.py` читает агентский `.env` с
`override=True`, поэтому значения host/port могут заменить env wrapper.
Текущий Fleet renderer не защищает `API_SERVER_HOST` / `API_SERVER_PORT`
от значений `env_json` и не фиксирует их как managed defaults. Проверка
wrapper до запуска не является доказательством фактически загруженной native
конфигурации. Для прямого native gateway запуска нужен versioned renderer
с защищёнными host/port и отдельной проверкой загрузки. Старые immutable
snapshots, revision markers и ожидаемые managed bytes сохраняются: их нельзя
молча переписать или пересчитать по новым defaults. Этот документ не закрывает
native acceptance, `runtime_ready` или SDLC admission.
См. [контракт адаптера](contracts/HERMES_ADAPTER_CONTRACT.md).

Отдельный [native protocol gate](../scripts/hermes_protocol_live/README.md)
проверяет настоящий API adapter/AIAgent/SQLite с локальной моделью, dropped ACK
и process crash. Он не запускает этот wrapper или полный gateway runner и не
закрывает dotenv precedence, managed lifecycle, native config/tool attestation
или production Fleet journal/unknown-acceptance recovery.

## Java Agent

- Existing externally provisioned Java jar lifecycle is retained. Chat/control
  and configuration activation are phase 2 and cannot enter automatic SDLC.
- Reserved fields: `AGENT_SERVER_PORT`,
  `SPRING_CONFIG_ADDITIONAL_LOCATION`, `/actuator/health`,
  `/api/v1/agent/chat/stream`, `/api/v2/sessions`, `/v1/capabilities`.
- Start requires the managed `runtime/backend.jar`, JDK command and db-only
  `/actuator/health/readiness` health. Missing jar fails validation; no fake
  successful chat/control is returned.

PM continuation proof is separate from runtime health and business completion.
The internal Workflow callback performs a fresh bounded/no-redirect HTTP probe
against the managed agent port, with Fleet's derived per-agent credential. It
verifies the acknowledged Hermes run and effective session identity. Queued,
running, approval-wait and stopping are non-terminal; interrupted is failed, not
successful or safely stopped. Terminal proof is immutable. Missing acceptance
mapping or inaccessible runtime blocks continuation; neither an SSE EOF nor a
database status can substitute for the probe.
PM `completed` readback also requires the native success flags above; a partial
or interrupted payload cannot record successful proof or release PM capacity.

Fresh terminal proof reconciles the matching Fleet run in the same transaction,
releasing runtime capacity even when the event stream was lost. A delayed EOF or
cached running/waiting event cannot reopen the old run; unknown acceptance still
holds the agent slot. A generic terminal cache update without verified PM proof
is rejected. Transcript finalization and Tracker stage completion remain separate.

## Current Approval Snapshot Recovery

For an originally accepted, pinned free-chat run, authenticated status GET can
restore the currently visible `approval.request` after Fleet loses its stream.
Fresh targeted-approval capabilities, exact native run/session/request identities
and the original origin/derived credential are required. Unknown, malformed,
oversized or foreign snapshots keep capacity held without a new prompt or SSE.

One transaction rechecks the current agent, primary session and accepted journal,
inserts the redacted request once and promotes running to waiting. Replays do not
insert transcript messages/events or reopen resolved decisions, stopping or
terminal runs. Task/PM bindings remain denied without fenced admission. This
recovers only the native status document's current request, not missed historical
questions, tool events or an unknown approval decision's outcome. Approval ACK
loss still remains uncertain. Process-tree quiescence is a separate open gate.
