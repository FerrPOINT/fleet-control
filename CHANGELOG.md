# Changelog

## [Unreleased]

### Added
- Versioned managed Fleet settings API with validated preview, optimistic
  concurrency, explicit restart confirmation, immutable history, atomic audit
  records and rollback-as-a-new-version. The active snapshot is applied on
  process startup while deployment-owned secrets and port mappings are kept.
- Managed settings workspace for runtime, agent ports, integrations, auth and
  retention with a shared draft, field-level preview, restart confirmation,
  version history and rollback. Failed mutations keep the draft and
  confirmation context available for retry.
- Streamlined настройки Fleet (#13); локализованный/компактный dashboard (#12).
### Fixed
- `/settings` больше не предлагает фиктивное применение runtime-конфигурации:
  UI показывает эффективные startup-значения только для чтения, а legacy PUT
  допускает только совместимый no-op и отвечает `409` при попытке изменения.
- Frontend Docker build снова воспроизводим с `pnpm --frozen-lockfile`:
  security overrides синхронизированы между `package.json` и lockfile.
- Семантика сохранения настроек (#14); метки полей редактора (#11); аудит dev-зависимостей (#15).
### Security

- HMAC access-token validation now delegates to the shared `sdlc-auth-core::Validator` (`hmac_with_audience` with the configured issuer/audience); the duplicated local decode/validate path is retired. Legacy pre-fleet token fallback and OIDC mode are unchanged.
- Align `tower-http` with `sdlc-shared` (0.7): production container builds no longer link incompatible CORS layer types after the shared dependency update.

### Added

- Settings UI теперь явно различает сохранённую запись и активную конфигурацию,
  а браузерный QA покрывает pending, успех, ошибку и повтор сохранения без записи
  в действующий backend.
- Настройки Fleet Control: локализованные вкладки, поиск пользователей,
  сохранение черновика и явные состояния загрузки, сохранения и ошибки.
- Runtime reconciler Phase 1: `reconcile_action` desired-state переходы (failed/stopped при desired=running — авто-restart в reconciler-цикле), unit-тесты матрицы статусов.

- Fleet alerts: `agent_restart_loop` (≥3 рестарта за 15 мин, перекрывает шторм `agent_down`, авто-resolve при recovery) и `agent_heartbeat_stale` (running без health ≥10 мин, reconciler-скан с дедупом).

## [Unreleased]

### Added

- Fleet monitoring alerts: миграция 6 (`fleet_alerts`), авто-алерты переходов здоровья агентов (agent_down / agent_recovered авто-resolve), `GET /api/v1/fleet-alerts` + acknowledge (Operator+, аудит), нед блокирующая запись переходов из start/stop/health.

## [Unreleased]

### Added
- OIDC/JWKS validation mode: `auth.mode=oidc` — RS256 против JWKS провайдера (кэш + refresh + kid-miss retry), строгие `iss`/`aud`, role-клейм маппинг, HMAC/local login отключены fail-closed.

## [Unreleased]

### Added
- Bulk runtime updates/rollback: `POST /api/v1/deployments/jobs/bulk` — один deployment job на агента (≤100), пропуск archived, `rollback` для runtime_update (IMPLEMENTATION_PLAN Phase 3).

## [Unreleased]

### Added
- Operator retention policy thresholds + scheduled stale-folder review (IMPLEMENTATION_PLAN Phase 3): `fleet.retention.stale_archived_days` (default 30) / `fleet.retention.review_interval_secs` (default 3600); storage report помечает `stale` + `archived_days`; фоновый review-воркер логирует stale-агентов; `POST /api/v1/settings/retention/review` (operator, audited).

## 0.2.0 - 2026-09-01

Initial Fleet Control scaffold.

- Created new Rust/React repository from the shared SDLC application stack.
- Added fleet-control domain model for Hermes and Java Agent runtime kinds.
- Added fresh PostgreSQL schema for agents, runtime state, configs, skills,
  sessions, workflow bindings, events, logs and auth users.
- Implemented Hermes folder provisioning and local process supervisor.
- Added Java Agent runtime template and phase 2 adapter contract.
- Added React pages for dashboard, agents, runtime, skills, config, sessions,
  workflows, deployments, logs and settings.
- Added documentation, contracts, ADRs and screenshot manifest.
