# Hermes Adapter Contract

Prepared journals may recover the one original POST through their still-unused
transactional permit, with exact saved bytes/key/context and fresh protocol
facts. This is not POST replay for submitted/unknown acceptance. Readback paths
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

Activation reserves `config/.fleet-activation-journal.json` before stopping a
tracked Hermes or changing files. Previous bytes, expected hashes and exact
agent/revision are protected and size-bounded. Rollback verifies every old file;
unknown outcomes preserve journal/drain. Cleanup follows the committed DB result,
not an HTTP success or EOF. This is not automatic restart reconciliation or proof
that all descendant OS processes stopped.

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
- Fleet forwards run controls to `/v1/runs/{run_id}/steer`,
  `/v1/runs/{run_id}/stop` and `/v1/runs/{run_id}/approval`.
- For executor sessions, the runtime dispatch target is the primary executor
  even when the mirrored message author is the selected leader.
- Fleet must not write directly into Hermes SessionDB.
- Programmatic chat/control uses the native gateway HTTP API reached through
  the compatibility wrapper, not raw Hermes serve/dashboard JSON-RPC.
- Dashboard remains a separate UI surface and is not the source of truth for
  Fleet message writes.
