# API

## Exact Runtime Approval Decisions

- `GET /api/v1/sessions/{session_id}/approvals` lists redacted requests visible to the owner or an operator/admin.
- `GET /api/v1/sessions/{session_id}/approvals/{approval_id}/decision` reads the durable decision; `404` means no command has been reserved.
- `POST /api/v1/sessions/{session_id}/approvals/{approval_id}/decision` accepts only `choice: once | deny` and a required `idempotency_key`. A verified human session is required; machine PATs cannot self-approve. Operator/admin approval is a runtime permission, never requirements confirmation.
- Reservation and audit commit before HTTP. An identical replay returns the same decision without another runtime call; a changed actor, key or choice returns `409`. Hermes must acknowledge the exact run/request/choice and exactly one resolution before `delivered` is stored.
- `uncertain` is not success or rejection. It survives process restart and blocks a new command for that request. Do not retry dispatch when the runtime outcome is unknown. Readback is safe; operator reconciliation still needs independent evidence.
- The legacy run-wide `/runs/{run_id}/approval` route returns `409`: broad `always`, session grants and `resolve_all` are not available in Fleet. Existing transcript routes remain supported.

These runtime requests are separate from Tracker clarification answers and exact-revision requirements confirmation.

The change from legacy run-wide `200` to unconditional `409` is an intentional
security-breaking migration, not backwards-compatible approval behavior. Clients
must list the exact requests and submit an idempotent decision for one request;
they must not fall back to the legacy route. The product compatibility wrapper
exempts only that former `200` response, checks the exact replacement refusal and
rejects any restored success response. Base still checks the route, request,
other responses, component schemas and every other API operation. Regression
tests prove neighboring removals and changes still fail. This exception disappears
once the baseline includes the retired route; the security guard still rejects
restored numeric/wildcard success responses and a missing or changed refusal.
It is not a general drift bypass.

## October SDLC Foundation

### Pinned Base Package Draft

`POST /api/v1/agents/{agent_id}/config/base-package` is an operator/admin-only
operation with no client-controlled source path or revision. Fleet reads regular
Git blobs at Base commit `4b9b4c9297a13fb28a6ba2039af2f7cb719f2f58` from the
configured operator-owned cache. It verifies schema, all seven roles, fourteen
skills, hashes and exact inventory before preparing the concrete agent's allowlist
and SOUL in a new configuration draft. Other enabled skills are disabled in that
draft; installed files, active runs and the effective revision are unchanged.

Before preparing the draft, Fleet also reads the persisted Workflow v3 binding
with a dedicated Base service-read PAT. Numeric namespace/workflow IDs stay
separate from namespace names, workflow keys and declared profiles; the mapping
is frozen in the configuration. An absent/denied/unavailable owner readback is
`503`; mismatched persisted IDs or a changed mapping is `409`. A declared profile
that differs from the Base package is `422`. No legacy catalog-token fallback
or automatic candidate installation is permitted. See the
[binding contract](contracts/SDLC_WORKFLOW_BINDING_V1.md).

The response is `AgentConfigRevision`, not installation/readiness evidence.
Missing cache, invalid package, non-Hermes runtime or mismatched role/namespace
fails with `422`. A changed desired revision, concurrent identity change or drain
returns `409`; unauthorized users receive `403`. No private source text or Git
stderr is included in errors or audit. Audit contains the revision number only.
The protected draft response contains its SOUL/skills just like existing revisions.

Validation and activation of a package-marked revision recheck the immutable
snapshot against the pinned Git source: a client-editable proof field alone is
not trusted. Fresh Workflow readback is required for validation, activation,
supervisor apply and package configuration observation; drift blocks the action.
Activation also checks role/namespace/workflow IDs under the agent row lock.
Existing legacy revisions without package metadata retain their behavior.
This endpoint does not attest physical/bundled skill inventory or enable dispatch.

`sdlc_role` is independent from `kind` and `product_role`. Configuration `PUT`
now saves a draft, not effective runtime files. New operator/admin routes:
`GET /agents/{id}/config/revisions`, `POST /agents/{id}/config/revisions/{revision}/validate`,
`POST .../activate`, and `GET /agents/{id}/readiness` (all under `/api/v1`).
History remains limited to the latest 100 revisions, but readiness reads the
effective head directly and validate/activate load the exact agent/revision.
An older non-desired revision may validate; activation still returns 409 unless
it is the current desired validated revision. A revision on another agent is 404.
Workflow rebind and actual role/namespace/workflow identity changes return 409
while configuration is draining, a runtime run is unresolved or accepted prompt
dispatch is pending/dispatching/uncertain. Unchanged identity fields do not turn
a metadata-only update into an identity mutation.
Human message requests cannot set agent authors/runtime IDs or non-prompt kinds.
Pending prompts use transactional outbox; acceptance-unknown dispatch is not retried.
Session SSE supports `Last-Event-ID` or `cursor`, and rechecks ownership/role/expiry.
Delta events contain a redacted `text` snapshot, not a fragment that could expose
a credential split across frames. See [current scope](SDLC_IMPLEMENTATION.md).
Runtime mirror completion now requires the exact accepted Hermes run and native
success flags; nested terminal names, partial results and invalid EOF status
responses leave the run unresolved. No public route or DTO shape changes for
this guard. A completed runtime mirror does not authorize a Tracker transition;
see the [Hermes adapter contract](contracts/HERMES_ADAPTER_CONTRACT.md).

Base path: `/api/v1`.

## PM Chat Gateway

- `POST /projects/{project_id}/pm-drafts`: opt-in verified human owner creation;
  accepts `agent_id`, `title`, `description`, `idempotency_key`. The operation is
  persisted before Tracker HTTP, recovers Draft/reservation through authoritative
  readback, and atomically creates a private task-bound PM chat. `202` means
  `awaiting_admission`, with `dispatch_allowed=false`, not an active PM run.
  Reuse the exact request/key after an interrupted response; changed payload is
  `409`. Human credentials are request-local and never saved for unattended retries.
  Each POST, including replay, first verifies the concrete agent's namespace via
  fresh Workflow ownership readback with a separate server-only machine PAT.
  Foreign project mapping is `409`; unavailable/denied/invalid ownership is `503`.
  This is not full admission and does not enable dispatch. See [ENV](ENV.md).
- `GET /pm-drafts/operations/{operation_id}`: owner-only, fresh human/project
  access; returns historical creation state, IDs and no original input or machine
  credentials. It is not current workflow or admission authority and never
  starts a run. Reading completed history remains possible with creation disabled.
- `GET /projects/{project_id}/pm-drafts/operation?idempotency_key=...`: recover
  the operation after an unknown creation response, when its UUID never reached
  the browser. The exact owner/key lookup uses fresh project authorization;
  another owner, project or Tracker instance cannot expose its metadata. Missing
  operation is `404`, invalid key is `422`, invalid query shape is `400`.
- `POST /pm-drafts/operations/{operation_id}/continue`: strict empty JSON object
  `{}` (arrays, null, fields and nonobjects are `422`); continue the persisted
  original operation without accepting replacement input, agent or command key.
  Rechecks human identity, owner, current project access, rollout and namespace.
  Returns `202` with existing incomplete/awaiting-admission state, never a run.
- `GET /pm-drafts/projects?after={canonicalUuid}`: verified human project choices
  from Tracker's strict `/api/v1/sdlc/project-directory`, with no legacy directory
  fallback. Returns `enabled`, `tracker_instance_id`, `projects` (ID/key/name)
  and required nullable `next_cursor`. Tracker page size is 50; Fleet filters
  the page by rollout allowlist but preserves the original cursor, including an
  empty filtered page. There is no total, readiness claim or default project.
  Disabled creation returns `enabled=false` and an empty directory without an
  upstream read. Creation and recovery still perform their own access checks.

- `POST /sessions/{id}/task-binding`: explicit owner binding to assigned concrete PM.
- `GET /sessions/{id}/task-context`: verified binding and Tracker context; unbound chat
  returns null context, dependency failure is not an empty successful SDLC response.
- `GET /sessions/{id}/chat-controls`: authoritative ownership/capability/dispatch gates.
- `GET /sessions/{id}/history?before={messageUuid}&limit=50`: latest-first pages, each
  page returned in server allocation order, independent of host timestamps; maximum
  100, UUID cursor scoped to session. Internal ordering is not an SSE replay cursor.
- `GET /sessions/{id}/clarifications`, `POST .../{questionId}/answers`.
- `GET /sessions/{id}/requirements`, `POST .../{revision}/confirm`.

Tracker context/confirmation may return `Analysis` as well as Draft,
Clarification and Backlog. Confirmation optionally forwards
`expected_routing_policy_version` (1..9007199254740991) for explicit routing
snapshot opt-in; omission/null preserves legacy wire and never enrolls a task.
The owner/project/exact-revision checks remain authoritative in Tracker.
Analysis intent/reservation metadata is read-only context, not permission to
send a prompt or bypass assignment admission. See
[chat contract](contracts/CHAT_CLARIFICATION_CONTRACT.md#bounded-event-metadata).

Answer and confirmation forward the verified original bearer to configured Tracker,
which revalidates human session, project membership and exact owner. Local legacy tokens
cannot authorize these commands. Operator read-all is not proxy consent. Payload conflict
and upstream status are retained; unknown network outcome requires same-key readback/replay.
Questions/revisions remain Tracker-owned JSON envelopes documented by the cross-service
contract, not a second Fleet database. OpenAPI generates the Fleet routes and response DTOs;
the gateway rejects malformed successful responses and versions unsafe for JavaScript.
`pnpm chat:contract` verifies seven wire shapes against the accepted Tracker v1 snapshot.
The separate PM Draft boundary checks exact captured Tracker reservation/readback
bytes, required nulls, canonical UUIDs and the canonical command envelope hash.
Use `node scripts/verify-chat-contract.mjs --tracker <tracker-openapi.json>` to check the
actual sibling build before rollout. Wire checks cover field names, required fields,
types/nullability, UUID/date formats and enums; semantic gates have separate backend tests.

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
  validation. The agent remains archived after file removal.
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

Единые правила чатов и source-review расхождения описаны в [CHAT.md](CHAT.md).
Там отдельно отмечены текущий SSE wire format, synchronous dispatch, ограничения
agent authorship и различия central/legacy permissions; наличие route не
подтверждает выполнение всех целевых гарантий.

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
- `POST /sessions/{session_id}/runs/{run_id}/approval` is retired and returns
  `409` without dispatch; use the exact-request decision endpoints above.
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

- `GET /settings/runtime` — эффективные startup-пути, источники и команды.
- `GET /settings/ports` — эффективные startup-порты backend и агентов;
  frontend port остаётся справочным внутренним значением deployment.
- `GET /settings/integrations` — эффективное подключение Project Workflow из
  startup-конфигурации.
- `GET /settings/auth` — эффективная legacy auth/cookie policy; в платформенном
  режиме human auth принадлежит Central Auth.
- Legacy `PUT` для этих четырёх endpoint сохранён для совместимости как
  idempotent no-op: точное совпадение с effective snapshot возвращает `200`,
  а попытка изменить значение — `409 Conflict`. Runtime-конфигурация через
  эти legacy endpoint не изменяется.
- `GET /settings/managed` — применённый процессом управляемый snapshot и номер
  активной версии. До первого применения `active_version` равен `null`, а
  snapshot собирается из deployment/env-конфигурации.
- `POST /settings/managed/preview` — нормализовать и проверить полный snapshot,
  вернуть field-level diff и признак обязательного restart без записи в БД.
- `POST /settings/managed/apply` — атомарно создать новую активную версию.
  Требует `expected_active_version` и `confirm_restart=true`; после успешного
  ответа процесс завершается с кодом `75`, чтобы supervisor выполнил restart.
- `GET /settings/managed/versions?limit=20` — неизменяемая история версий.
- `POST /settings/managed/versions/{version}/rollback` — создать новую активную
  версию из выбранного snapshot; история не переписывается.

Managed snapshot включает runtime roots/commands, диапазон портов агентов,
несекретные CI/CD и Project Workflow integration settings, auth/cookie policy и
retention thresholds. Database URL, JWT/runtime/API tokens, backend bind port,
frontend port и host/container port mappings остаются deployment-owned. Apply и
rollback записываются в `audit_log` в одной PostgreSQL-транзакции с переключением
активной версии; stale `expected_active_version` получает `409 Conflict`.

- `POST /deployments/jobs/bulk` — bulk runtime updates/rollback (Phase 3): один job на агента из `agent_ids` (≤100), archived/unknown пропускаются и считаются в `skipped`; `rollback: true` допустим только для `runtime_update` (помечает jobs и добавляет `detail.rollback`).

`POST /deployments/jobs` также принимает отдельные продуктовые операции Service Pulse:

| `job_kind` | Обязательные поля | Результат |
|---|---|---|
| `product_deploy` | `environment: "demo"`, точный 40-символьный `commit_sha`, UUID `idempotency_key`, `title` | Forge deployment для commit из защищённого `main` |
| `product_rollback` | `environment: "demo"`, UUID успешного `previous_release_id`, UUID `idempotency_key`, `title` | Отдельный Forge rollback deployment |

Для этих видов `agent_id`, `runtime_kind` и произвольный `detail` не допускаются. Повтор с тем же ключом и тем же содержимым возвращает исходный job, изменение параметров даёт `409`. `detail` ответа содержит связанный Forge deployment/pipeline ID и `health_verified`; `completed` возможен только после успеха pipeline и самостоятельной HTTP-проверки Pulse API/UI. Ошибка Forge, отмена, 30-минутный таймаут или провал health завершают job как `failed` с `last_error`. Переходы и ключ идемпотентности хранятся в PostgreSQL и восстанавливаются после рестарта. Для локального стенда задаются `FLEET_CONTROL_FLEET__PULSE_HEALTH_URL` и `FLEET_CONTROL_FLEET__PULSE_UI_URL`.
- `POST /settings/retention/review` — запустить проход stale-folder review сейчас (operator, audited): возвращает `stale_agent_ids` archived-агентов старше `fleet.retention.stale_archived_days`, порог и время прохода

Управляемая версия накладывается на deployment/env baseline при следующем
старте процесса. Секреты и сетевое подключение контейнера из baseline всегда
сохраняются; пользователи и платформенный вход управляются Central Auth.

The frontend build regenerates TypeScript types from `openapi/openapi.json`.
The OpenAPI JSON is regenerated from Rust source before release. Native Windows
regeneration requires MSVC `link.exe`; WSL/Linux generation is supported.

## PM Runtime Readback

`GET /api/v1/agents/{agent_id}/readiness` remains operator/admin-only.
An active/effective database revision does not imply that its runtime files
are intact. Fresh read-only filesystem verification adds
`effective_configuration_readback_failed` when the snapshot, marker, isolated
workspace or skills cannot be verified. Underlying paths, resolved credentials
and file hashes are not exposed. `effective_revision` still reports the database
head, not a successful runtime observation. Workflow admission remains a separate
blocker; this endpoint cannot authorize PM dispatch.
For a pinned Base snapshot, readback also revalidates Git provenance and the
role allowlist, then scans HOME skills (maximum 4096 entries / 16 levels).
Unlisted/nested/case-aliased skill files, links, special files and missing allowed
skills fail closed without deleting them. Legacy non-package snapshots preserve
the previous managed-only verification. `.bundled_manifest` never supplies proof.
Project/external/plugin discovery and runtime-loaded settings are not attested. The separate
`runtime_skill_inventory_not_verified` blocker prevents treating intact managed
files as proof of complete skill inventory/native provenance.

### SDLC Configuration Observation

`GET /internal/runtime/v1/agents/{agent_id}/configuration` is an opt-in machine
read, outside browser/local admin authentication. Base freshly introspects a
dedicated PAT at the configured fixed origin. Its canonical subject must equal
the registered configuration reader, its scopes must be exactly
`fleet-control:read` (no wildcard, duplicates or write scope), and the concrete
agent UUID must be in Fleet's deployment-owned reader allowlist. This uses Base's
existing service read/write PAT issuance, not an unissuable compound grant.
Email is never identity authority.
No PAT is stored by this read, and no shadow user is created.

The version-1 observation exposes only agent/role, effective revision, pinned
public package proof and frozen Workflow mapping metadata, observation UUID/time, managed-file verification
and blockers. It never returns prompt/skill content, env values, filesystem paths
or credentials. The effective database head is re-read after verification;
concurrent change/drain returns `409`. Effective head and exact pinned revision
are read directly, independently of the latest-100 history window.
Missing package/effective revision also
returns `409`; invalid/disabled dependency configuration or failed file/provenance
checks return `503`; missing/invalid/revoked PAT `401`, wrong subject/scopes `403`.

`runtime_ready` is deliberately `false`: this is a read-only observation, not a
lease, config activation, assignment acceptance/ACK, admission token or run
receipt. Tracker must not dispatch using its observation UUID. Native effective
settings/inventory and the frozen assignment/workflow protocol remain gates.

`GET /internal/runtime/v1/pm/runs/{session_run_id}` is a machine-only callback
for Project Workflow, outside browser authentication. It requires the dedicated
`FLEET_CONTROL_PM__READBACK_TOKEN`; an unset/short/reused credential fails closed.
The response is the flat Workflow `RuntimeObservation`, without a Fleet envelope.
It contains immutable Tracker/assignment/execution identity, Fleet run UUID,
binding, dispatch key, fence, checkpoint and a fresh observation UUID/status.

Fleet probes the authenticated Hermes `/v1/runs/{runtime_run_id}` on every call.
Runtime mapping mismatch, unknown status, unreachable runtime and malformed or
oversized replies return `503`; contradiction of stored terminal proof returns
`409`. Cached Fleet run state, EOF and a human-provided status are not proof.
The endpoint does not create assignments, dispatch a prompt or resume Workflow.


## Fleet alerts (monitoring, Phase 3)

- `GET /api/v1/fleet-alerts?state=open|acknowledged|resolved` — алерты переходов здоровья агентов (Operator+). Kinds: `agent_down` (critical, running/ready → failed/stopped/degraded), `agent_recovered` (авто-resolve открытых или подтверждённых `agent_down`/`agent_restart_loop`/`agent_heartbeat_stale` при возврате в running/ready), `agent_restart_loop` (warning: ≥3 restart-событий за 15 минут — перекрывает одиночный `agent_down`, чтобы оператор видел цикл, а не шторм), `agent_heartbeat_stale` (warning: running-агент без свежего health ≥10 минут; сканируется reconciler-циклом, дедуп по одному активному алерту на агента).
- `POST /api/v1/fleet-alerts/{alert_id}/acknowledge` — Operator+; ack только для `open`-алертов; аудит `fleet_alert.acknowledge`.
- Переходы пишутся в `fleet_alerts` (миграция 6) из start/stop/health операций без блокировки ответа.

## Общая база

Подключение версий, границы контрактов и проверки описаны в [BASE_INTEGRATION](BASE_INTEGRATION.md).
