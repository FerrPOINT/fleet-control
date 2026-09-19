# API

Base path: `/api/v1`.

Auth:

При настроенном `FLEET_CONTROL_AUTH__CENTRAL_JWKS_URI` UI использует Central
Auth Authorization Code + PKCE, а backend проверяет центральный Bearer token и
активность сессии. Локальные register/login/refresh и изменение роли закрыты;
ошибка Central Auth не переключает приложение на локальный пароль.

- `POST /auth/register`
- `POST /auth/login`
- `POST /auth/refresh`
- `POST /auth/logout`
- `GET /users/me`
- `GET /users/me/permissions`
- `GET /users`
- `PATCH /users/{user_id}/role`

В платформенном режиме access tokens выпускает Central Auth. Локальный профиль
создаётся строго по `sub`, каталог `/users` синхронизируется с Central Auth, а
личные API-токены управляются в Admin Panel. Local HMAC JWTs остаются только для
явного legacy-режима без центральной конфигурации.

RBAC:

В центральном режиме все активные люди получают одинаковые пользовательские
права Fleet Control; экраны назначения локальных ролей скрыты. Роли ниже
применяются только к legacy-режиму. Runtime/service credentials остаются
отдельной машинной границей.

- `admin`: all users, settings, RBAC, sessions and runtime actions.
- `operator`: agents, leaders, executors, runtime, config, skills, deployments,
  logs and all sessions.
- `user`: own sessions/messages and read-only agent directory.

Fleet:

- `GET /dashboard`
- `GET /agent-directory`
- `GET /agents`
- `POST /agents`
- `GET /agents/storage-review` returns a fleet-wide managed-folder retention
  review with total bytes, archived bytes, purge-ready agents and marker/path
  issues.
- `GET/PATCH/DELETE /agents/{agent_id}`
- `GET /agents/{agent_id}/storage` returns the managed folder storage report,
  marker status and purge eligibility.
- `POST /agents/{agent_id}/purge-files` physically removes the managed
  `agents_root/agentN` folder after archive, exact name confirmation and marker
  validation.
- `POST /agents/{agent_id}/provision`
- `POST /agents/{agent_id}/start`
- `POST /agents/{agent_id}/stop`
- `POST /agents/{agent_id}/restart`
- `POST /agents/{agent_id}/health`
- `GET/PUT /agents/{agent_id}/config`
- `GET /agents/{agent_id}/skills`
- `PUT /agents/{agent_id}/skills/{skill_name}`
- `GET /leaders`
- `GET/PUT /leaders/{leader_agent_id}/executors`
- `GET /executors`

Sessions and workflow:

- `GET /sessions?agent_id={agent_id}&leader_agent_id={leader_id}&user_id={id1,id2}`
  lists sessions by primary agent, selected leader and user filter.
- Omitting `user_id` returns only the current user's sessions.
- `user_id=all` returns all users only for admin/operator; normal users are
  forbidden from expanding beyond themselves.
- `POST /sessions` creates a session owned by the authenticated user. Use
  `primary_agent_id`; legacy `agent_id` is still accepted.
- `POST /sessions` is idempotent by `idempotency_key`; replay returns the
  original session, while the same key with a different payload returns `409`.
- `GET /sessions/{session_id}`
- `GET/POST /sessions/{session_id}/messages`
- `POST /sessions/{session_id}/messages` is idempotent by request key and avoids
  duplicate runtime dispatch on replay.
- `GET /sessions/{session_id}/stream`
- `GET /sessions/{session_id}/participants`
- `PUT /sessions/{session_id}/leader`
- `POST /sessions/{session_id}/handoff`
- `POST /sessions/{session_id}/delegations`
- `GET /sessions/{session_id}/runs`
- `POST /sessions/{session_id}/runs/{run_id}/steer`
- `POST /sessions/{session_id}/runs/{run_id}/stop`
- `POST /sessions/{session_id}/runs/{run_id}/approval` forwards the decision to
  Hermes and resolves pending Fleet approval mirror records for that run.
- `GET /workflow-bindings`
- `GET /workflow-catalog` reads the live Project Workflow catalog via its
  read-only `/internal/runtime/catalog` bridge. Configure
  `FLEET_CONTROL_FLEET__PROJECT_WORKFLOW_URL` and
  `FLEET_CONTROL_FLEET__PROJECT_WORKFLOW_CATALOG_TOKEN` on Fleet; the latter
  must match Workflow's `PROJECT_WORKFLOW_FLEET_CATALOG_TOKEN` and is never
  returned to the browser. Background binding refresh uses the same bridge.
- `PUT /workflow-bindings/{agent_id}` requires an operator and an exact
  `{namespace_id, workflow_id}` pair from that live catalog. Fleet rejects a
  workflow outside the selected namespace, atomically updates the agent and its
  binding, then records an audit entry. Stale bindings are never retargeted by
  the background reconciler.

Session defaults:

- Direct executor chat: `leader_agent_id = null`, `visibility = private`.
- Direct leader chat: primary agent and selected leader are the same leader.
- Child executor session from a leader chat inherits `leader_agent_id` and gets
  `parent_session_id`.
- Backend validates that selected leaders manage the target executor through
  `leader_executors`.

Runtime:

- `GET /runtime-templates`
- Hermes chat dispatch uses the runtime adapter and `/v1/runs`; Fleet sends
  `session_id=fleet:{session_id}:{agent_id}` and stores the mirror transcript.
- `GET/POST /deployments/jobs`
- `GET /deployments/jobs/{job_id}`
- `POST /deployments/jobs/{job_id}/cancel`
- `GET /logs`
- `GET /events` as SSE
- `GET /events/recent`
- `GET /audit-log`

Settings:

- `GET/PUT /settings/runtime`
- `GET/PUT /settings/ports`
- `GET/PUT /settings/integrations`
- `GET/PUT /settings/auth`
- `POST /deployments/jobs/bulk` — bulk runtime updates/rollback (Phase 3): один job на агента из `agent_ids` (≤100), archived/unknown пропускаются и считаются в `skipped`; `rollback: true` допустим только для `runtime_update` (помечает jobs и добавляет `detail.rollback`).
- `POST /settings/retention/review` — запустить проход stale-folder review сейчас (operator, audited): возвращает `stale_agent_ids` archived-агентов старше `fleet.retention.stale_archived_days`, порог и время прохода

Auth settings expose legacy `mode`, `jwt_issuer`, `jwt_audience`, token TTLs and
refresh-cookie policy. В платформенном стенде фактический human auth задаётся
Central Auth env-конфигурацией и общим `sdlc-auth-core`.

The frontend build regenerates TypeScript types from `openapi/openapi.json`.
The OpenAPI JSON is regenerated from Rust source before release. Native Windows
regeneration requires MSVC `link.exe`; WSL/Linux generation is supported.


## Fleet alerts (monitoring, Phase 3)

- `GET /api/v1/fleet-alerts?state=open|acknowledged|resolved` — алерты переходов здоровья агентов (Operator+). Kinds: `agent_down` (critical, running/ready → failed/stopped/degraded), `agent_recovered` (авто-resolve открытых `agent_down`/`agent_restart_loop`/`agent_heartbeat_stale` при возврате в running/ready), `agent_restart_loop` (warning: ≥3 restart-событий за 15 минут — перекрывает одиночный `agent_down`, чтобы оператор видел цикл, а не шторм), `agent_heartbeat_stale` (warning: running-агент без свежего health ≥10 минут; сканируется reconciler-циклом, дедуп по одному open-алерту на агента).
- `POST /api/v1/fleet-alerts/{alert_id}/acknowledge` — Operator+; ack только для `open`-алертов; аудит `fleet_alert.acknowledge`.
- Переходы пишутся в `fleet_alerts` (миграция 6) из start/stop/health операций без блокировки ответа.
