<p align="center">
  <img src="docs/assets/fleet-control-readme-banner.svg" alt="Base Fleet Control - isolated agent runtime operations" />
</p>

SDLC implementation is in progress, not production accepted. Current scope and
remaining blockers: [SDLC implementation](docs/SDLC_IMPLEMENTATION.md).

Docker recovered configuration activation, including sequential revisions over
the proved effective child, has a default-off
[Base4 source consumer](docs/RECOVERED_ACTIVATION_CONSUMER.md), not native acceptance
or production readiness. SDK19a is unchanged; utility9b is sealed separately.

Chat/PM work is in progress: [implementation plan](docs/CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md),
[contract](docs/contracts/CHAT_CLARIFICATION_CONTRACT.md). Production chat controllers
use Fleet/Tracker APIs; workflow dispatch/resume and real PM acceptance remain blocked.
Task-bound reads enforce current Tracker project access, including directory counts
and stream replay. Central private-owner filtering applies to directory rows,
counts and cursors even with expanded user filters. The seven clarification DTOs
are validated against nested wire constraints; source parity is not admission.
See the [verification ledger](docs/CHAT_CLARIFICATION_VERIFICATION.md)
for PostgreSQL evidence and the remaining live-integration gates.
Creation recovery now includes owner/key lookup, persisted-operation continuation
and strict Tracker project choices. The new [creation form proposal](docs/design/PM_DRAFT_CREATION_PREVIEW.md)
is isolated and awaiting approval, not production UI or runtime admission.
The server-only delegated PM client is limited to enumerated SDLC operations for
its assigned task. This client restriction does not replace Tracker authorization
and does not enable credential handoff or runtime dispatch.

The task-chat release preserves both accepted migration lineages. Its pending
down migration refuses populated transcript, task-binding, creation or approval
history before dropping anything; empty-schema rollback remains supported.
See [migration rules](docs/MIGRATIONS.md#historical-lineages-and-task-chats).

<p align="center">
  <a href="#overview"><img src="https://img.shields.io/badge/Overview-3730a3?style=for-the-badge" alt="Overview" /></a>
  <a href="#capabilities"><img src="https://img.shields.io/badge/Capabilities-4338ca?style=for-the-badge" alt="Capabilities" /></a>
  <a href="#routes"><img src="https://img.shields.io/badge/Routes-0e7490?style=for-the-badge" alt="Routes" /></a>
  <a href="#quick-start"><img src="https://img.shields.io/badge/Quick_Start-155e75?style=for-the-badge" alt="Quick start" /></a>
  <a href="#visual-proof"><img src="https://img.shields.io/badge/Visual_Proof-0f766e?style=for-the-badge" alt="Visual proof" /></a>
  <a href="#safety"><img src="https://img.shields.io/badge/Safety-334155?style=for-the-badge" alt="Safety" /></a>
  <a href="#quality"><img src="https://img.shields.io/badge/Quality-52525b?style=for-the-badge" alt="Quality" /></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-2024-000000?style=flat-square&logo=rust&logoColor=white" alt="Rust 2024" />
  <img src="https://img.shields.io/badge/Axum-Rest_API-3730a3?style=flat-square" alt="Axum REST API" />
  <img src="https://img.shields.io/badge/SeaORM-2563EB?style=flat-square" alt="SeaORM" />
  <img src="https://img.shields.io/badge/PostgreSQL-17-4169e1?style=flat-square&logo=postgresql&logoColor=white" alt="PostgreSQL 17" />
  <img src="https://img.shields.io/badge/Redis-8-dc2626?style=flat-square&logo=redis&logoColor=white" alt="Redis 8" />
  <img src="https://img.shields.io/badge/React-19-38bdf8?style=flat-square&logo=react&logoColor=0f172a" alt="React 19" />
  <img src="https://img.shields.io/badge/OpenAPI-6BA539?style=flat-square&logo=openapiinitiative&logoColor=white" alt="OpenAPI" />
  <img src="https://img.shields.io/badge/CI-.github%2Fworkflows%2Fci.yml-15803d?style=flat-square" alt="Repository CI" />
</p>

---

> **Base Fleet Control** — self-hosted control plane для изолированных agent runtime: лидеры, исполнители, технические агенты, их сессии, skills, конфигурация, storage, деплой и lifecycle. Управляет runtime-границей; не заменяет workflow, документацию или CI/CD-системы за этой границей. Env-префикс: `FLEET_CONTROL_`.

<a name="overview"></a>

## Обзор и Snapshot

| Поле | Значение |
|---|---|
| Backend | Rust 2024 workspace: api, app, domain, infra, shared, server, cli, migration |
| Data | PostgreSQL 17 (agents, configs, skills, sessions, logs), Redis 8 + SSE |
| Frontend | React 19, Vite, Tailwind CSS |
| Runtime adapters | Hermes (изолированный home/workspace), Java Agent (operator-provided JAR) |
| API | [openapi/openapi.json](openapi/openapi.json) — canonical contract |
| Порты | repository-local: frontend `23802`, backend `23801`; Base umbrella: frontend `7742`, API `7741` |
| License | FerrPOINT Proprietary Source-Available Evaluation License v1.0 |

В standalone legacy mode первый зарегистрированный пользователь получает
`system_role = admin`. При Central Auth локальная регистрация выключена; вход
не повышает сохранённую роль, а legacy bootstrap-admin subject не выдаёт прав.
Все активные central users имеют одинаковый control-plane доступ; PAT service
scopes остаются обязательными. Приватный чат остаётся доступным только владельцу,
business answer/confirmation нельзя выполнить от имени другого владельца.

В central mode локальный профиль связывается с проверенным `sub`, а актуальное
имя берётся из той же проверки активности JWT/PAT. Переименование не меняет
ID/роль и не объединяет исторических пользователей по email; после обновления
страницы новое имя видно в меню. Статус узкой интеграционной приёмки и отдельные
незакрытые release gates: [Current state](docs/CURRENT_STATE.md).

<a name="capabilities"></a>

## Возможности

| Feature | Описание |
|---|---|
| Leader/executor модель | `agents.product_role` (leader/executor) отделён от `agents.kind` (runtime type). |
| Технические агенты | Создание, архивирование, profiles, skills, конфигурация, workspace/storage view и сессии. |
| Runtime lifecycle | Provision, start, stop, restart, health и logs через runtime adapters. |
| Сессии | Private-by-default task sessions, привязка к лидеру, control-message mirrors и runtime run links. |
| PM Draft | Opt-in owner creation: Tracker Draft/reservation и private task-bound чат с восстановлением. Ожидание admission, без запуска PM. |
| Workflows | Namespace/workflow bindings (source of truth — `project-workflow`). |
| Deployments | Runtime templates и deployment jobs. |
| Алерты | Fleet alerts page с bulk runtime update panel. |
| Storage review | Fleet-wide и per-agent size reports, marker state и purge eligibility. |
| OIDC / JWKS | Central auth mode: RS256 access tokens, cached JWKS, строгие `iss`/`aud`. |
| Наблюдаемость | Request id, audit/events, rate controls, health и Prometheus metrics. |
## Стек

| Zone | Tech | Роль |
|---|---|---|
| API | Rust + Axum | HTTP routes, auth, DTO boundary |
| Domain/App | Rust workspace crates | services, policies, repository contracts |
| Persistence | SeaORM + PostgreSQL | runtime data и migrations |
| Cache/Push | Redis + SSE | runtime support и event stream |
| Shared Base | services-base-aligned | fleet-standard request id и tracing bridge |
| Frontend | React + Vite + Tailwind | operational fleet UI |
| Contract | OpenAPI | generated frontend API types |
| Evidence | Playwright screenshots | UI coverage desktop and mobile viewports |

Модель runtime (из storage review):

| Area              | Details                                                                                                               |
| ----------------- | --------------------------------------------------------------------------------------------------------------------- |
| Hermes layout     | `HERMES_HOME=data/agents/agentN/config`, cwd `data/agents/agentN/workspace`.                                          |
| Java Agent layout | `AGENT_SERVER_PORT`, `SPRING_CONFIG_ADDITIONAL_LOCATION`, `/actuator/health`, `/api/v2/sessions`, `/v1/capabilities`. |
| Session ownership | `agent_sessions.user_id` references the authenticated user that created the session.                                  |
| Leader binding    | `leader_executors` defines which executors a leader may manage.                                                       |
| Session leader    | `agent_sessions.leader_agent_id` is nullable; `NULL` means private chat.                                              |
| Auth bridge       | Local HMAC JWTs carry fleet-compatible `aud`, `iss`, `role`, `scopes` and `sid` claims.                              |
| Agent naming      | Agent ordinals come from database allocation and materialize `agentN` folders.                                        |
| Deletion model    | Agent delete means archive/stop by default; physical purge is a separate explicit operation.                          |
| Purge preview     | Storage reports are read-only and recomputed from `agents_root/agentN` before deletion.                               |
<a name="quick-start"></a>

## Быстрый старт

Сначала разместите совместимый `services-base` рядом с `fleet-control`.
Docker использует именованный BuildKit context; требуется Compose 2.17+.
Ревизия Base и ограничения runtime описаны в [DEPLOYMENT](docs/DEPLOYMENT.md).

```bash
cp .env.example .env
# Заменить POSTGRES_PASSWORD, FLEET_CONTROL_JWT_SECRET и
# FLEET_CONTROL_FLEET__RUNTIME_TOKEN_SECRET в .env
docker compose up --build -d --wait
curl -fsS http://127.0.0.1:23801/api/v1/health
```

Frontend dev:

```bash
cd frontend
pnpm install
pnpm generate:api
pnpm dev
```

Backend dev:

```bash
cd backend
cargo run -p server
```

- Frontend dev: `http://127.0.0.1:5173`
- Frontend Docker: `http://127.0.0.1:23802`
- Backend: `http://127.0.0.1:23801/api/v1/health`
- API docs: `http://127.0.0.1:23801/swagger-ui/`

<a name="routes"></a>

## Фронтенд-роуты

| Route | Назначение |
| --- | --- |
| `/login`, `/register` | Auth |
| `/`, `/dashboard` | Fleet dashboard |
| `/leaders` | Leader agents, managed executors и leader-scoped sessions |
| `/leaders/new` | Создание leader wizard |
| `/leaders/:leaderId` | Leader team editor и sessions |
| `/leaders/:leaderId/edit` | Leader identity, profile, workflow и team edit |
| `/executors` | Executor agents и task sessions |
| `/executors/new` | Создание executor wizard |
| `/executors/:agentId` | Executor overview |
| `/executors/:agentId/edit` | Executor identity, profile и workflow edit |
| `/agents` | Технический инвентарь агентов с фильтром session ownership |
| `/agents/new` | Создание generic agent wizard |
| `/agents/:agentId` | Agent overview |
| `/agents/:agentId/edit` | Generic agent identity edit |
| `/agents/:agentId/runtime` | Runtime provision/start/stop/restart/health |
| `/agents/:agentId/skills` | Per-agent skills |
| `/agents/:agentId/config` | Config, SOUL и env editor |
| `/agents/:agentId/workspace` | Guarded workspace overview |
| `/agents/:agentId/sessions` | Agent-local sessions |
| `/sessions` | Cross-agent task sessions с user и leader фильтрами |
| `/sessions/:sessionId` | Transcript mirror, leader selector, runtime runs и handoff |
| `/workflows` | Namespace/workflow bindings |
| `/deployments` | Runtime templates и deployment surface |
| `/logs` | Global logs и event stream |
| `/settings` | Root paths, runtime sources, integrations и users |

<a name="visual-proof"></a>

## Визуальные доказательства

### Актуальный Header (1 октября 2026)

Общий Header принят на production QA image без API mocks. Sidebar содержит
только навигацию, профиль и выход находятся в одном account menu. Три темы,
320–2560 px, keyboard/touch и fingerprints описаны в
[evidence](docs/assets/screens/2026-10-01-platform-header/README.md).

![Fleet Header, 1920 px](docs/assets/screens/2026-10-01-platform-header/agents-dark-1920.png)

Следующая трёхрежимная галерея является историческим снимком до нового Header.
Представительные страницы были сняты на детерминированной fixture при
`1920x1080` в default theme. Полный маршрутный набор, включая responsive QA и
параметры пересъёмки, хранится в
[manifest](docs/assets/screens/manifest.md).

### Дашборд (`wide`)

![Дашборд](docs/assets/screens/1920x1080/03-dashboard.png)

### Карточка лида (`detail-with-aside`)

![Карточка лида](docs/assets/screens/1920x1080/06-leader-detail.png)

### Настройки (`reading/form`)

![Настройки](docs/assets/screens/1920x1080/36-settings.png)

## Архитектура

```mermaid
flowchart TD
    UI[React Fleet Control SPA] --> API[Axum API]
    API --> App[Application services]
    App --> Domain[Domain contracts]
    App --> Repo[SeaORM repositories]
    Repo --> DB[(PostgreSQL)]
    API --> Redis[(Redis)]
    App --> Runtime[Runtime supervisor/adapters]
    Runtime --> Hermes[Hermes serve process]
    Runtime --> Java[Java jar lifecycle and phase 2 chat]
    API --> OpenAPI[OpenAPI contract]
    OpenAPI --> Gen[Generated frontend types]
```

<a name="safety"></a>

## Границы

- `task-tracker` использовался только как stack/UI/docs donor; sibling fleet repos не мутируются.
- `services-base` предоставляет shared building blocks; Fleet Control использует telemetry-совместимый локальный bridge (WSL/CI не может fetch private shared repo). Auth tokens уже используют fleet-compatible HMAC claims; замена локальной валидации на `sdlc-auth-core` — отдельный шаг совместимости.
- `project-workflow` остаётся source of truth для workflow и namespace definitions.
- Java Agent provisioning намеренно заблокирован до реализации runtime adapter.
- Fleet Control зеркалит transcript/control сообщения и диспатчит через runtime boundary; он не пишет напрямую в Hermes SessionDB.
- Filesystem-операции остаются под настроенным agents root; secrets подлежат redaction.
- Physical purge — явная guarded-операция под настроенным agents root.

<a name="quality"></a>

## Качество и проверки

| Проверка | Команда |
| --- | --- |
| Frontend typecheck | `cd frontend && pnpm typecheck` |
| Frontend lint | `cd frontend && pnpm lint` |
| Frontend unit tests | `cd frontend && pnpm test` |
| Frontend build | `cd frontend && pnpm build` |
| Playwright e2e | `cd frontend && pnpm test:e2e` |
| Screenshots | `cd frontend && pnpm screenshots:local && pnpm screenshots:verify` |
| Backend format | `cd backend && cargo fmt --all -- --check` |
| Backend compile | `cd backend && cargo check --workspace --all-targets` |
| Backend clippy | `cd backend && cargo clippy --workspace --all-targets -- -D warnings` |
| Backend tests | `cd backend && cargo test --workspace` |
| README contract | `python3 scripts/verify_readme.py` |
| CI | GitHub Actions: docs, backend, OpenAPI drift и frontend gates |

[Production-проверка подтверждения настроек](docs/assets/screens/settings-confirmation-2026-10-04/README.md): ошибки, pending, keyboard/focus, темы и размеры экрана.

## Карта проекта

```text
fleet-control/
├── backend/     # Rust workspace: api, app, domain, infra, shared, server, cli, migration
├── frontend/    # React SPA: pages, widgets, generated API client и Playwright tests
├── openapi/     # canonical generated API contract
├── docs/        # requirements, architecture, contracts, operations, security и screenshots
├── .github/     # CI workflow
└── docker-compose.yml
```

## Документы

- [docs/README.md](docs/README.md) — обзор документации.
- [docs/CHAT.md](docs/CHAT.md) — полный контракт чатов и сессий, текущие ограничения и критерии приёмки.
- [docs/TZ.md](docs/TZ.md), [docs/PRODUCT_REQUIREMENTS.md](docs/PRODUCT_REQUIREMENTS.md) — scope и требования.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/FRONTEND_ARCHITECTURE.md](docs/FRONTEND_ARCHITECTURE.md), [docs/contracts](docs/contracts) — архитектура и контракты.
- [docs/DATA_MODEL.md](docs/DATA_MODEL.md), [docs/API.md](docs/API.md), [docs/ENV.md](docs/ENV.md) — технические справочники.
- [docs/LOCAL_SETUP.md](docs/LOCAL_SETUP.md), [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md), [docs/OPERATIONS.md](docs/OPERATIONS.md) — runbooks.
- [docs/SECURITY.md](docs/SECURITY.md), [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md) — security model.
- [docs/TESTING.md](docs/TESTING.md), [docs/RISK_REGISTER.md](docs/RISK_REGISTER.md), [docs/TRACEABILITY.md](docs/TRACEABILITY.md) — качество и traceability.
- [docs/PRE_DEVELOPMENT_GATE.md](docs/PRE_DEVELOPMENT_GATE.md), [docs/GAP_REGISTER.md](docs/GAP_REGISTER.md), [docs/QUALITY_GATE.md](docs/QUALITY_GATE.md), [docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md) — pre-development hardening gate.
- [docs/assets/screens/manifest.md](docs/assets/screens/manifest.md) — screenshot manifest.

<a name="license"></a>

## Лицензия

Proprietary source-available. Not open source. Viewing/evaluation only.

Commercial, production, resale, redistribution, SaaS/hosting use require written license from FerrPOINT. См. [LICENSE](LICENSE), [NOTICE](NOTICE) и [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
