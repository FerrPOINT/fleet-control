# Implementation Plan

## Current SDLC Scope

Docker consumer is now connected in the integration candidate: immutable
container registration/paths/config/context/source pins precede Base start;
start/health/stop and every Hermes endpoint use original container receipts.
Unknown start stays claimed and cannot resend or fall back to native Hermes.
This requires a private operator-prepared generation. Next implement guarded
Compose rendering/create, trusted controller bridge attachment, daemon source
mapping, logs, new-generation restart and namespace-based config rollback.
Do not enable installed Docker mode or label automatic provisioning complete.
Task admission, producer first step, PM tools/resume and real Forge/seven-agent
acceptance remain separate required gates.

The full fresh Rust1.88/PG candidate gate passes490 tests with29 explicitly
ignored profiles, all-target check/strict Clippy and fmt. See
[verification and retained failure](CHAT_CLARIFICATION_VERIFICATION.md).
This proves neither automatic provisioning nor actual container/Hermes or
full PM acceptance; do not relabel the ignored/live gates as complete.

Read-only remote recheck: Base PR150 at4cfdfa9 is ready/CLEAN with nine green
CI37420102732 jobs; PR144 is merged at main63fff28. This certifies the endpoint
utility, not the Fleet consumer. Its validation/publication remains separate.

## Historical Native Packet

The earlier Base release was d2c8ef6, preserving accepted maina3d6a79 with unchanged
boundary/bootstrap bytes and nine green jobs in CI37409952184. The a0f7044
observation below is historical. Fleet2293862 is published integration source;
its229 component/38 harness cases and fresh native lifecycle are scoped local
evidence, not readiness for PM, host-boundary consumption or automatic SDLC.

The reconciled foundational release PR47 now has five green exact-head CI jobs
at `5f20540`, but stays Draft for missing live PM acceptance. Base PR144 has nine
green jobs at `a0f7044`; its host primitive still is not a Fleet consumer.
The separate integration follow-up rejects dead retained-child dispatch and
foreign-controller runtime overwrites/false HTTP health-transition alerts.
Its fresh compilation, component and native evidence must be accepted separately;
neither foundational CI nor fixture screenshots certify the later runtime tail.

The pre-spawn native launch journal `000017` binds agent/config/controller before
execution and holds unknown outcomes across Fleet replicas. Component validation
is recorded separately; this does not complete the host-boundary integration,
loaded config, safe descendants or task admission. Release after `000016` as its
own one-migration packet, never by rewriting applied migration history.

The packet also pins native launch identity in private dispatch intents and
checks it before preparation, permit consumption and actual HTTP submission.
An old or legacy intent cannot follow a replacement managed gateway. This is
not loaded-generation attestation or full task admission.

Source reconciliation with accepted main `3c6b8ef` now preserves canonical and
legacy split foundations, verified central names and the accepted Base SDK.
The actual PostgreSQL lineage suite verifies both populated histories through
the shared eight-step tail (18 canonical/21 split entries), not just a clean
installation. Historical migration bytes are unchanged. Ordered single-migration
release packets, exact-head CI and review are still prerequisites. Keep PR47
and dependency packets unchanged until their owners perform that process.

The historical phases below are not completion evidence for the October SDLC plan.
Current implementation, boundaries, blockers and acceptance are maintained in
[SDLC_IMPLEMENTATION.md](SDLC_IMPLEMENTATION.md). Leaders are deferred. Java lifecycle
exists, but runtime chat/control still returns phase-2 errors and cannot run SDLC.

Original approval journal000016 and its opt-in consumer are implemented.
Original-mode HTTP reservation, exact-byte approval POST and bounded GET-only
recovery require their recorded component gates. Native positive/unknown decision
outcomes, separate Fleet SIGKILL recovery and combined run/command plugins are
verified in disposable native QA. Installed rollout and complete SDLC remain.
Journal component
checks do not close those gates or enable installed runtime flags. Ordered
one-migration release packets and exact-head CI/reviews still precede rollout.

Phase 0: pre-development hardening.

- RBAC and permissions endpoint. — done: `SystemRole = admin|operator|user` c бэкенд-энфорсментом (middleware + `GET /users/{id}/permissions`, `PATCH /users/{id}/role`), legacy `is_system_admin` alias; см. docs/AUTHORIZATION.md.
- Idempotent sessions/messages. — done: миграция 0004 (`idempotency_key` + `idempotency_payload_hash`, unique `(user_id, idempotency_key)`), replay возвращает исходную сессию/сообщение.
- Session participants, leader selection, handoff and delegation. — done: `leader_agent_id`, роли `leader|executor`, `/sessions/{id}/participants`, handoff/delegation-роуты (docs/API.md §sessions).
- Deployment jobs and settings surfaces. — done: `deployment_jobs` + bulk `POST /deployments/jobs/bulk`, settings API с per-key аудитом (docs/API.md).
- Product pages for leaders and executors. — done: `frontend/src/pages/{leaders,executors}` + карточки агентов/сессий.
- Technical pages for agents, deployments, logs and settings. — done: `frontend/src/pages/{agents,deployments,logs,settings,alerts}`.
- Screenshot manifest and evidence capture. — done: 82 desktop-файла в `docs/assets/screens/` + manifest.md (`1920x1080` и `2560x1440`).
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
- Wire health, capabilities, sessions and chat stream. — partial: health/readiness and lifecycle exist; chat/control remain phase 2 in runtime/mod.rs.
- Add Java Agent runtime tests and screenshots. — done: runtime-тесты provision/launch/readiness + скрин-свидетельства java-agent-страниц в evidence-сете.

Phase 3: fleet operations.

- Add operator retention policy thresholds and scheduled stale-folder review. — done: `fleet.retention.stale_archived_days` / `fleet.retention.review_interval_secs`, fleet-wide `GET /api/v1/agents/storage-review`, stale flag + archived days in storage report, scheduled review worker, `POST /api/v1/settings/retention/review`.
- Add richer monitoring and alerts. — done: `fleet_alerts` (миграция 6): авто-алерты переходов здоровья (agent_down/agent_recovered), `GET /fleet-alerts`, acknowledge (Operator+), авто-закрытие открытых и подтверждённых алертов после восстановления, аудит; событийная модель включает restart-loop/heartbeat-stale.
- Add bulk runtime updates and rollback. — done: `POST /api/v1/deployments/jobs/bulk` (см. docs/API.md); rollback помечает runtime_update jobs через `detail.rollback`.
- Add cross-project workflow health integration. — done: `refresh_workflow_bindings` сверяет биндинги с живым project-workflow каталогом (reconciler, runtime/mod.rs:344), `binding_status` в UI/API.
