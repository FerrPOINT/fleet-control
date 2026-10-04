# Changelog

## [Unreleased]

- Require bounded tracked-process kill/wait before acknowledging runtime stop;
  preserve ownership on failure and refuse untracked active stop/restart rather
  than clearing PID/state without proof. Process-tree release remains separate.
- Serialize configuration application/rollback with runtime lifecycle actions;
  recheck drain after waiting, preserve unconfirmed health ownership and enforce
  bounded Java readiness plus wall-clock startup deadlines for both runtimes.
- Refuse physical folder purge when runtime stop is unconfirmed; do not emit a
  purge success event or audit entry after a failed stop.
- Validate delegated credentials against acknowledgement-time TTL after Base
  lock waits; retain exact scope/expiry bounds and disable environment proxies.

- Persist protected, exclusive configuration activation backups before runtime
  or file changes; verify restored bytes on rollback and retain the journal until
  the database result commits. Interrupted/unknown journals block another apply;
  automatic crash reconciliation remains a separate gate.

- Revalidate pinned Base effective snapshots against Git and a bounded closed
  HOME skill-file inventory. Add opt-in agent-scoped, freshly introspected Base
  PAT configuration observation with metadata only and no runtime admission.
- Keep chat directory owner filtering on the existing user/session index by
  converting its JSON owner list to a UUID array before membership checks.
  Preserve all-users counts, project ACLs, search and pagination contracts.

- Bind server-only delegated PM credentials to enumerated operations for their
  canonical assigned task. Reject legacy/foreign task paths and owner/verifier
  actions before attaching the bearer. Keep the Base delegation wire unchanged;
  receiving-service enforcement and real runtime handoff are separate gates.

- Add owner/key recovery for an unknown PM Draft creation response and strict
  empty-object continuation using persisted original input. Add bounded strict
  Tracker project directory choices with preserved rollout-filtered cursors.
  Include an isolated creation/recovery design proposal and generated screenshots;
  production form approval/integration and PM runtime admission remain pending.

- Add disabled-by-default owner-driven PM Draft creation with a persisted
  operation ledger, authoritative Tracker readback/input/reservation checks and
  atomic task-bound chat. Recover lost responses with the same command keys;
  reject stale/changed receipts. Creation stops at awaiting_admission and never
  dispatches a PM runtime prompt. Generate the two public routes from Rust.

- Add an internal atomic PM Draft chat/binding operation with durable replay,
  exact human owner and real Hermes PM checks. It creates no prompt or runtime
  run; runtime admission and initial delivery remain separate work.

- Preserve new transcript allocation order across host clock rollback without
  changing public message DTOs or UUID cursors. Merge overlapping history pages
  in server order and keep older-page loading alive during SSE reconnect, with
  a catch-up read for messages arriving while the page was pending.
  Historical backfill retains the previous timestamp/UUID ordering.

- Connect an opt-in authenticated Tracker metadata poller with fresh Base subject
  and exact read-only scope checks, project-scoped keyset scans, bounded HTTP and
  durable cursor replay. No PM dispatch or task transition is performed.

- Add strict `metadata_v1` Tracker event decoding and immutable per-binding
  projection/version pins, including empty pages. Validate source digests and
  lossless decimal cursors; reject implicit legacy conversion. Polling remains
  separate from this transactional storage contract.

- Reconcile verified PM terminal proof and visible runtime state atomically;
  serialize late stream updates so they cannot reopen the old run or overwrite
  its accepted mapping. Unknown acceptance continues to hold agent capacity.

- Add a server-only Base credential delegation client with exact Tracker PM
  assignment/execution/agent/version scopes, bounded no-retry/no-redirect HTTP,
  redacted secrets and fixed-origin child authorization. Coordinator and runtime
  handoff remain blocked pending live integration.

- Add transactional Tracker event inbox foundation with immutable replay receipts,
  per-binding cursors and safe transcript/stream projections; PM answer delivery
  remains separate integration work.

- Recheck Tracker project access for task-bound transcript, lists/counts, runs,
  controls and every session-stream event; retained Fleet ownership cannot bypass revocation.
- Revalidate PM assignment after approval reservation waits; definitely undispatched
  failures are terminal and mirror foreign keys no longer deadlock PM capacity locks.
- Reject malformed approval lists as recoverable UI errors and synchronize the
  complete three-browser chat/catalog/approval fixtures.
- Add server-authorized chat directory search, aggregate counts and scoped cursors;
  keep own-user default and preserve return context without loading all transcripts.
- Add immutable PM run reservations and authenticated fresh Hermes readback for
  Workflow; unknown acceptance holds capacity and terminal proof cannot regress.
- Integrate exact-request human tool approvals with immutable command replay,
  stale-assignment protection and no automatic redispatch after an unknown outcome.
  PM structured dispatch/resume and live clarification acceptance remain incomplete.

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
