# Implementation Plan

Phase 0: pre-development hardening.

- RBAC and permissions endpoint. — done: `SystemRole = admin|operator|user` c бэкенд-энфорсментом (middleware + `GET /users/{id}/permissions`, `PATCH /users/{id}/role`), legacy `is_system_admin` alias; см. docs/AUTHORIZATION.md.
- Idempotent sessions/messages. — done: миграция 0004 (`idempotency_key` + `idempotency_payload_hash`, unique `(user_id, idempotency_key)`), replay возвращает исходную сессию/сообщение.
- Session participants, leader selection, handoff and delegation. — done: `leader_agent_id`, роли `leader|executor`, `/sessions/{id}/participants`, handoff/delegation-роуты (docs/API.md §sessions).
- Deployment jobs and settings surfaces. — done: `deployment_jobs` + bulk `POST /deployments/jobs/bulk`, settings API с per-key аудитом (docs/API.md).
- Product pages for leaders and executors. — done: `frontend/src/pages/{leaders,executors}` + карточки агентов/сессий.
- Technical pages for agents, deployments, logs and settings. — done: `frontend/src/pages/{agents,deployments,logs,settings,alerts}`.
- Screenshot manifest and evidence capture. — done: 132 файла в `docs/assets/screens/` + manifest.md (desktop 1920×1080/2560×1440 + mobile 375×812).
- Documentation and ADR alignment. — done: полный док-паритет с task-tracker (55 файлов), ADR + ADR_INDEX синхронизированы.

Phase 1: Hermes MVP completion.

- Finish real Hermes API session open/send/stream integration. — done: Hermes `/v1/runs` адаптер (open/send/SSE mirroring/stop/steer/approval forwarding) — см. CURRENT_STATE.md.
- Expand fake Hermes lifecycle tests into real adapter contract tests. — done: контрактные тесты адаптера на фикстурах `backend/tests/fixtures` (Run-переходы, SSE, approvals).
- Add runtime reconciler tests for desired-state restart. — done: `reconcile_action(status, desired)` решает Restart/HealthCheck/None (unit-тесты переходов), reconciler-цикл перезапускает failed/stopped агентов с desired=running и health-checkает running.
- Add clean DB migration and seed workflows. — done: SeaORM-миграции 0001+ (idempotent IF NOT EXISTS), seed в тестах через фикстуры.
- Replace the local HMAC token validator with `sdlc-auth-core::Validator::hmac`
  after WSL/CI can fetch `services-base`. — done: `AuthService` валидирует через `sdlc_auth_core::Validator` (hmac-mode), локальный дубликат decode-логики удалён; см. docs/AUTHORIZATION.md.
- Add OIDC/JWKS validation mode and retire the compact-token legacy fallback
  after the transition window. — done: `auth.mode=oidc` — RS256/JWKS (кэш+refresh, kid-miss), строгие iss/aud, маппинг ролей, local login и HMAC-токены отклоняются fail-closed (см. docs/ENV.md, docs/OPERATIONS.md).

Phase 2: Java Agent runtime.

- Implement Spring Boot launch/provision adapter. — done: supervisor поднимает `java -jar <agents_root>/agentN/runtime/backend.jar --spring.profiles.active=noop`, readiness по `/actuator/health/readiness` (db-only).
- Wire health, capabilities, sessions and chat stream. — done: health/capabilities/sessions/chat-стрим через ja REST/SSE адаптер (runtime/mod.rs).
- Add Java Agent runtime tests and screenshots. — done: runtime-тесты provision/launch/readiness + скрин-свидетельства java-agent-страниц в evidence-сете.

Phase 3: fleet operations.

- Add operator retention policy thresholds and scheduled stale-folder review. — done: `fleet.retention.stale_archived_days` / `fleet.retention.review_interval_secs`, fleet-wide `GET /api/v1/agents/storage-review`, stale flag + archived days in storage report, scheduled review worker, `POST /api/v1/settings/retention/review`.
- Add richer monitoring and alerts. — done: `fleet_alerts` (миграция 6): авто-алерты переходов здоровья (agent_down/agent_recovered), `GET /fleet-alerts`, acknowledge (Operator+), авто-закрытие открытых и подтверждённых алертов после восстановления, аудит; событийная модель включает restart-loop/heartbeat-stale.
- Add bulk runtime updates and rollback. — done: `POST /api/v1/deployments/jobs/bulk` (см. docs/API.md); rollback помечает runtime_update jobs через `detail.rollback`.
- Add cross-project workflow health integration. — done: `refresh_workflow_bindings` сверяет биндинги с живым project-workflow каталогом (reconciler, runtime/mod.rs:344), `binding_status` в UI/API.
