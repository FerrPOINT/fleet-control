# Changelog

## [Unreleased]

- Process-log inserts return their own persisted redacted row atomically, even
  when another stdout/stderr writer has already inserted a newer log. PostgreSQL
  regressions cover deterministic interleaving, 64 writers and rejected inserts.
- Диалоги apply/rollback настроек возвращают фокус на кнопку preview; новое подтверждение очищает ошибку предыдущей операции. Pending и product-owned preview/retry сохранены. Base закреплён на проверенный SHA для воспроизводимой общей поставки.

- Active Central Auth users can operate Fleet without local role grants.
  Personal token scopes remain enforced, private sessions remain owner-only,
  shared sessions permit central users, and bootstrap/role mutation cannot
  promote historical profiles. Standalone legacy RBAC is preserved.
- Session SSE remains bound to the original validated user during replay and
  idle checks; a still-active token rebound to another subject closes the stream.
- Global Fleet SSE revalidates the original bearer identity, activity and read
  scope during idle and before every delivery. Revocation and Auth failure close
  held streams; central private session events remain owner-only, and standalone
  streams recheck current local roles.

- Central profiles use the confirmed current name from the same JWT/PAT
  activity check. Same-sub ID, local roles and historical same-email users are
  preserved; missing names fail closed and unchanged profiles avoid a write.
- Restore compatibility with the historical 13-step split migration ledger
  alongside the canonical 10-step registry without rewriting applied versions;
  unknown or mixed histories are rejected.

- Повторный явный вход разрешён после отменённого перехода Central Auth;
  автоматический guard не отменяет logout. Отмена перехода не показывает
  ложную ошибку доступности Auth, а ошибка сервера остаётся видимой.

- Fix standalone Docker builds with explicit shared Base contexts and locked
  installs; add Compose health/auth/privacy/SSE/restart acceptance in CI.
- Disable Nginx buffering for session streams and bind frontend to loopback
  by default; explicit network exposure requires deployment configuration.
- Add seven SDLC specializations, agent-grouped Chats, durable session events,
  transactional prompt dispatch and versioned config activation with drain/rollback.
- Enforce verified-subject permissions, redact legacy configuration secrets and
  preserve managed files during provisioning. Automatic SDLC remains blocked
  pending cross-service contracts and live acceptance.

- Выход направляет браузер в Central Auth до изменения локального auth-state.

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

- Fleet shell adopts the full-width shared PlatformHeader with a nav-only
  sidebar, one bounded account menu and unchanged central logout ordering.
  Mobile navigation is 44 px, closes after navigation/desktop resize and
  cleans up its media listener. Dashboard localization live smoke now uses
  the real drawer/account controls and canonical private QA session path.
- Fleet detail layouts now use the shared Base 320 px contextual rail from
  1024 px. Agent overview/workspace and session controls stack below primary
  content on smaller screens; leader sessions precede the team editor in DOM
  and visual order. Two-panel editors remain wide workspaces.
- Agent detail: localized six tabs and runtime/storage states, wrapping 40 px
  navigation, explicit load errors with retry, pending locks and confirmed save
  feedback. Invalid JSON objects cannot submit a previously valid config value;
  failed saves preserve drafts.
- Hermes с desired state running восстанавливается после перезапуска Fleet,
  если локальный процесс отсутствует. SSE completion сохраняет настоящий
  output модели вместо имени события или идентификатора run.
- Settings tabs now keep their Radix tab panels in the accessibility tree, so
  every `aria-controls` reference resolves across responsive layouts.
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
