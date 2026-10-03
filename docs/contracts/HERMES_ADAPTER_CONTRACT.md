# Hermes Adapter Contract

Hermes is the first implemented runtime.

Environment:

- `HERMES_HOME=agentN/config`
- `API_SERVER_ENABLED=true`
- `API_SERVER_KEY=<derived per-agent token>`
- `HERMES_SERVE_HEADLESS=1`

Working directory:

- `agentN/workspace`

Managed files:

- `config/config.yaml`
- `config/SOUL.md`
- `config/.env`
- `config/skills`
- `runtime/source.json`

Lifecycle:

- start: configured Hermes command with `serve --host 127.0.0.1 --port <api_port>`
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
- Hermes serve/JSON-RPC is the intended programmatic chat surface.
- Dashboard remains a separate UI surface and is not the source of truth for
  Fleet message writes.
