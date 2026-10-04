# Hermes Adapter Contract

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

`API_SERVER_HOST` / `API_SERVER_PORT` сейчас задаёт wrapper, не Fleet renderer.
Hermes загружает агентский `.env` с `override=True`; текущий renderer допускает
эти ключи из `env_json`. Следовательно, первоначальные env/argv не доказывают
фактический native bind. Нужен versioned renderer с защищёнными host/port
и проверкой native loaded configuration. Исторические immutable snapshots,
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
- Fleet mirrors events from `GET /v1/runs/{run_id}/events`.
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
