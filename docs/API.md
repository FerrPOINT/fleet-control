# API

## PM Human Controls Integration Candidate

PM MCP `workflow_step` with a null/absent report returns the existing instruction
response and retains it under the original key/body in the existing tool journal.
New PM question/revision publication requires that proof for the same current
run/native binding/assignment. A held call is not a publication receipt; no new
route, public DTO, model-access authorization or Hermes hook is introduced.

This candidate reuses the existing run stop/steer and clarification-command
routes; it adds no second control API or scheduler. Workspace/PG/browser and
authentic updated OpenAPI qualification remain required before release.

`ClarificationAnswerCommand.continuation_state` separates `not_required`,
`pending` and `confirmed` from Tracker delivery `state`. The pending-command
GET also returns `delivered` answers whose PM continuation remains `pending`.
Explicit delivery/recovery uses the original command ID and saved answer; a
successful Tracker answer alone cannot confirm continuation. Confirmation
requires the same execution/checkpoint, verified Workflow rebind and native
acceptance. Neither delivery nor continuation publishes requirements.

Task-bound PM stop/steer require the owner, fresh project access, the current
assignment and supported native capabilities. Stop acknowledgement is not
terminal or safe-process-stop proof. Unknown effects keep their original key
and receipt, not a new submission. Idle PM free-form launch currently reports
`pm_idle_prompt_contract_unavailable`; no fabricated clarification or unbound
chat fallback is permitted. Operators cannot substitute for owner decisions.

## Hermes Recovery Candidate

The recovery slice changes no public Fleet route, DTO or OpenAPI schema. The default-off original-key
extension uses an authenticated non-dispatch lookup, not a repeated run POST.
Pinned free-chat terminal evidence commits delivery, optional redacted assistant
and run state atomically. This provides no task/PM model authority or business
completion. See [the scoped release](plans/2026-10-09-hermes-recovery-release.md)
and [wire contract](contracts/HERMES_RECOVERY_V1.md); native compatibility is pending.

## Approval Recovery Candidate (Unit14)

The existing POST `/api/v1/sessions/{session_id}/approvals/{approval_id}/decision`
keeps body `{choice:"once"|"deny",idempotency_key:string}`, verified-human/owner
and assignment guards. Native preflight now requires the original accepted
free-chat context and exact current pending action. The durable uncertain
reservation is replayed without another native POST; only an exact ACK delivers.
GET on that decision path and GET `/api/v1/sessions/{session_id}/approvals`
remain scoped DB-only reads. Missing decision returns404, not permission to POST.
Background current-snapshot recovery is not historical decision acceptance.
No public schema change; source-only, exact Linux/native validation pending.

## Acknowledged Steer Transcript Follow-Up

Successful free-chat steer now atomically appends one redacted human-authored
`control` message with `delivery_state=mirrored` to existing session history.
Its ID equals `command.id`; `runtime_message_id` is the Fleet-local link
`fleet-control:<session_run_id>:<command_id>:steer`, not a native message ID.
It acknowledges guidance only, never completion. Uncertain/rejected/terminal-
observed receipts do not produce a delivered control mirror. Exact acknowledged
POST replay can repair a pre-fix missing mirror without native I/O; scoped GETs
remain read-only. No endpoint, DTO, generated contract or client change is made.
See [the follow-up boundary](plans/2026-10-09-steer-transcript-release.md).

## Durable Runtime Controls Unit13

The [isolated control release](plans/2026-10-09-runtime-controls-release.md)
requires a verified human and exactly one bounded `Idempotency-Key` for
free-chat steer/stop. Responses add nullable `command`; scoped
`GET /sessions/{session_id}/runs/{run_id}/controls[/{command_id}]` reads
durable receipts without dispatch. Existing owner/project guards remain.
Task/PM-bound commands remain denied. Unknown ACK is not success or a retry
permit; stop ACK does not prove safe OS stop or SDLC completion.
Both stop and steer check current Tracker project access before returning the
task-bound assignment conflict. With a valid request key, revoked access returns
403 rather than 409; neither path reserves a control or dispatches to runtime.
Authentic Rust-generated OpenAPI and TypeScript types are now integrated;
see [generator and consumer evidence](plans/2026-10-09-runtime-controls-ui.md).
Explicit original-key UI settlement/reload recovery is fixture-verified;
combined native gates remain open.

`GET /api/v1/sessions/{session_id}/runs/{run_id}/controls/lookup` recovers the
original command receipt when the POST response (including its command ID) was
lost. It requires a verified human session and exactly one bounded, validated
`Idempotency-Key` header. Existing session read, project and run guards apply.
The actor is always the authenticated user, never a supplied actor ID; even an
operator cannot look up another actor's key. The existing unique actor/key pair
is constrained by the exact session/run. A match returns one
`RuntimeControlReceipt` object (200); no match returns 404, never a list or a
fallback to the most recent command. Keys and payload hashes are not returned.
The original collection GET remains the unchanged latest-100 bounded list.
The literal lookup route is distinct from the UUID receipt route: an old server
rejects it rather than silently ignoring a header on the collection endpoint.
Missing/404, transport/auth errors and old-server rejection never prove that
the unknown POST did not execute, clear a hold, or authorize a fresh key or
redispatch. Clients must retain the original actor/session/run/key context.
This is Fleet-ledger readback only: no native I/O, ledger mutation, outbox action
or migration. Rust registration and authentic OpenAPI/TS are integrated from
run37953053154/artifact11626597579 for exact sourcecfa30f39. The sole generated
wire delta adds this GET; the receipt DTO and legacy collection remain unchanged.
This generator result is not the new API/PG test gate or live control acceptance.

Requiring the header is an intentional security-breaking migration on these
two existing POST operations: old clients must provide one stable key for the
original command and retain it on replay. Missing keys are rejected before
native dispatch; clients must not generate a fresh key after an unknown result.
The compatibility gate explicitly models only the exact required string header
`Idempotency-Key` on stop/steer. It rejects missing/optional/duplicate/changed
headers and still checks every other parameter, body, response and route through
the unchanged Base compatibility checker. This is not general v1 compatibility
and must be coordinated with consumers before enabling unit13.

## Docker Activation Restart Recovery

The opt-in [Base4 consumer](RECOVERED_ACTIVATION_CONSUMER.md) adds no public API or
DTO. Existing desired/effective activation endpoints retain their drain gates.
Sequential revisions over an effective recovered child use those same endpoints,
the original anchor/latest lease and sealed predecessor lineage; there is no
new public override or child takeover API. Failed-next rollback targets current
effective, not the obsolete root revision. Native acceptance remains pending.
Its private recovered authority is not a human override, takeover API or permission
to replay unknown native commands. Older unit18 recovery limitations below refer
to the legacy Base169 path; unsupported Base4 phases still retain typed holds.

The [config18 restart successor](CONTAINER_ACTIVATION_RECOVERY_RELEASE.md) exposes
typed recovery reason/action JSON through the existing revision `last_error`
string and `agent_config.recovery_required` audit event. This is a recovery
request, not a new command endpoint or successful activation. Drain remains set.

The [config18 P2 successor](CONTAINER_ACTIVATION_PREFLIGHT_FIX.md) adds Docker-only
rendered-byte preflight and live-custodian read-only retry. No public routes or
schemas change; invalid targets/unknown effects retain drain.

Docker configuration activation uses the existing config revision endpoints;
there is no new public endpoint or generated schema. Desired remains the
requested revision; effective changes only after original generation, file and
API proof. Unknown effects retain drain. See
[unit18 private activation contract](CONTAINER_ACTIVATION_RELEASE.md).
The [normal integration](CONTAINER_ACTIVATION_INTEGRATION.md) keeps public routes
unchanged and isolates sibling agents' lifecycle/activation work without relaxing
drain, ownership, unknown-effect or admission checks.

Public routes are unchanged by [automatic Docker preparation unit17](AUTOMATIC_CONTAINER_PREPARATION_RELEASE.md).
The configured initial-generation path no longer requires an operator-prepared
document; it still requires original physical and admission gates.

Unit16 adds no public route or DTO. Its private mapped-volume/controller recovery
contract and pending native gates are in [the source handoff](MAPPED_CONTROLLER_RECOVERY_RELEASE.md).
Its correctness follow-up keeps foreign unknown heartbeats held without native
mutation and isolates agents' renewal work; no public readiness/admission changes.

## Original Docker Lifecycle

Existing start/stop/restart/health operations support the trusted opt-in Docker
supervisor. No new public routes, fields or OpenAPI schemas are introduced.
Unknown physical effects/ownership return unavailable and retain holds. A stopped
generation needs new operator preparation before restart. See
[bounded Docker source release](DOCKER_LIFECYCLE_RELEASE.md).

## Hermes Journal Release Unit12

The isolated Hermes journal unit changes internal dispatch persistence only; no
public endpoint, DTO or OpenAPI schema is added. An accepted run whose effective
session is unresolved remains pending and cannot receive run-control mutations.
See [the unit12 release plan](plans/2026-10-09-hermes-journal-release.md).

## Task-Chat Wire And Privacy Boundary

The seven clarification DTOs use closed nested schemas and JavaScript-safe
positive version values. Confirmation may include
`expected_routing_policy_version`; omitting it preserves the original wire.
`Analysis` is a valid Tracker stage. The contract checker compares validation
constraints, not just property names, against the generated Rust OpenAPI and an
explicit producer source or the accepted snapshot. This is source compatibility,
not a runtime admission receipt.

For central identities, `/chats` applies the private-owner predicate to the
shared relation used for counts, pages and cursor validation, even with an all
or multi-user selection. Task history/context/control/gateway reads use the
same session guard as main transcript routes, plus fresh project authorization
where required. Legacy standalone role semantics are retained separately;
read-all never permits answering or confirming for another task owner.

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

`sdlc_role` is independent from `kind` and `product_role`. Configuration `PUT`
now saves a draft, not effective runtime files. New operator/admin routes:
`GET /agents/{id}/config/revisions`, `POST /agents/{id}/config/revisions/{revision}/validate`,
`POST .../activate`, and `GET /agents/{id}/readiness` (all under `/api/v1`).
Human message requests cannot set agent authors/runtime IDs or non-prompt kinds.
Pending prompts use transactional outbox; acceptance-unknown dispatch is not retried.
Session SSE supports `Last-Event-ID` or `cursor`, and rechecks ownership/role/expiry.
Delta events contain a redacted `text` snapshot, not a fragment that could expose
a credential split across frames. See [current scope](SDLC_IMPLEMENTATION.md).

Base path: `/api/v1`.

## PM Chat Gateway

- `POST /projects/{project_id}/pm-drafts`: opt-in verified human owner creation;
  accepts `agent_id`, `title`, `description`, `idempotency_key`. The operation is
  persisted before Tracker HTTP, recovers Draft/reservation through authoritative
  readback, and atomically creates a private task-bound PM chat. With dispatch
  disabled, `202` returns `awaiting_admission` and `dispatch_allowed=false`.
  With opt-in PM dispatch enabled, a submitted intent returns
  `awaiting_runtime_acceptance` (`dispatch_allowed=false`) until native acceptance
  is known, then `runtime_accepted` (`dispatch_allowed=true`). Both runtime states
  require task/session IDs and `next_step=runtime`. Acceptance is not terminal
  execution, workflow completion, requirements confirmation or task completion.
  Reuse the exact request/key after an interrupted response; changed payload is
  `409`. Human credentials are request-local and never saved for unattended retries.
  Each POST, including replay, first verifies the concrete agent's namespace via
  fresh Workflow ownership readback with a separate server-only machine PAT.
  Foreign project mapping is `409`; unavailable/denied/invalid ownership is `503`.
  Namespace ownership alone is not full admission; enabled dispatch independently
  checks current assignment, configuration and workflow before the native request.
  See [ENV](ENV.md).
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
  Returns `202` with the current creation/runtime-acceptance projection. It reuses
  the saved operation and dispatch custody; an unknown native acceptance is not
  permission to submit a new native run.
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

`display_name` локального профиля обновляется из актуальной центральной
проверки JWT или личного токена. `sub`, локальный ID и исторические авторы
сохраняются; старое имя в JWT и email prefix не являются источником имени.
При отсутствии проверенного имени запрос отклоняется без создания/изменения
профиля. Повторная проверка неизменённого имени не обновляет `updated_at`.

RBAC:

В центральном режиме все активные люди получают одинаковые пользовательские
права Fleet Control; экраны назначения локальных ролей скрыты. Роли ниже
применяются только к legacy-режиму. Runtime/service credentials остаются
отдельной машинной границей.

`/users/me/permissions` возвращает центральные права независимо от сохранённых
`system_role` и `is_system_admin`. Эти исторические поля не повышаются при входе.
Для личного токена `fleet-control:read` write-controls не объявляются, а backend
отклоняет мутации до выполнения handler. Назначение локальной роли всегда
возвращает `403` в central mode; bootstrap subject больше не выдаёт роль.

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
- Central users may select other users or `user_id=all` without local roles.
  Private sessions remain owner-only in list, detail, messages and SSE; shared
  leader-scoped sessions are accessible to all active central users. The private
  filter is applied before the list limit. Legacy expansion requires admin/operator.
- `POST /sessions` creates a session owned by the authenticated user. Use
  `primary_agent_id`; legacy `agent_id` is still accepted.
- `POST /sessions` is idempotent by `idempotency_key`; replay returns the
  original session, while the same key with a different payload returns `409`.
- `GET /sessions/{session_id}`
- Session detail includes optional `task_bound`, computed from the immutable
  Fleet task binding. It does not depend on legacy `task_key` or a display name.
  Private and task controllers require an explicit Boolean projection; missing
  projection keeps the controller closed. List DTOs may omit it.
- Session detail includes optional `pending_delivery`, computed from every
  session message in `pending` or `dispatched` delivery state, independently of
  the bounded history page. List DTOs may omit this projection. A consumer that
  gates prompt submission must hold sending when the detail projection is absent.
- An initial `pending` run with both runtime identity fields explicitly `null`
  is a preparation slot. A bound pending run or any `running`, `waiting` or
  `stopping` primary run still holds sending. Pending delivery holds it separately.
- `GET/POST /sessions/{session_id}/messages`
- `POST /sessions/{session_id}/messages` is idempotent by request key and avoids
  duplicate runtime dispatch on replay.
- A successful message POST acknowledges its specific persisted row, including
  when it is beyond the first 500 messages returned by history. The authorized
  POST receipt includes optional `request_payload_hash`, taken from the stored
  idempotency payload hash. History, runtime dispatch and assistant mirrors do
  not expose that digest. Requests without an idempotency key may omit it.
- The digest is SHA-256 of the parsed request serialized as compact JSON with
  sorted keys: `author_agent_id`, `body`, `idempotency_key`, `message_kind` and
  `runtime_message_id`; missing optional fields serialize as `null`. Public body
  redaction does not change this digest. A client requiring positive command
  confirmation must retain the original key if the digest is missing or differs.
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
  returns persisted redacted process records. Internal writes acknowledge the
  exact inserted row, even if another stream has already written a newer row;
  public response fields and ordering are unchanged.
- `GET /events` as SSE. The bearer token is revalidated before delivery and once
  per second while idle. Revocation, expiry, disabled users or Auth unavailability
  terminate the existing connection; the client must authenticate again.
  Central private session events are owner-only. `fleet` event names and payload
  shapes are unchanged; no token is accepted through URL query parameters.
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

| `job_kind`         | Обязательные поля                                                                            | Результат                                         |
| ------------------ | -------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| `product_deploy`   | `environment: "demo"`, точный 40-символьный `commit_sha`, UUID `idempotency_key`, `title`    | Forge deployment для commit из защищённого `main` |
| `product_rollback` | `environment: "demo"`, UUID успешного `previous_release_id`, UUID `idempotency_key`, `title` | Отдельный Forge rollback deployment               |

Для этих видов `agent_id`, `runtime_kind` и произвольный `detail` не допускаются. Повтор с тем же ключом и тем же содержимым возвращает исходный job, изменение параметров даёт `409`. `detail` ответа содержит связанный Forge deployment/pipeline ID и `health_verified`; `completed` возможен только после успеха pipeline и самостоятельной HTTP-проверки Pulse API/UI. Ошибка Forge, отмена, 30-минутный таймаут или провал health завершают job как `failed` с `last_error`. Переходы и ключ идемпотентности хранятся в PostgreSQL и восстанавливаются после рестарта. Для локального стенда задаются `FLEET_CONTROL_FLEET__PULSE_HEALTH_URL` и `FLEET_CONTROL_FLEET__PULSE_UI_URL`.

- `POST /settings/retention/review` — запустить проход stale-folder review сейчас (operator, audited): возвращает `stale_agent_ids` archived-агентов старше `fleet.retention.stale_archived_days`, порог и время прохода

Управляемая версия накладывается на deployment/env baseline при следующем
старте процесса. Секреты и сетевое подключение контейнера из baseline всегда
сохраняются; пользователи и платформенный вход управляются Central Auth.

The frontend build regenerates TypeScript types from `openapi/openapi.json`.
The OpenAPI JSON is regenerated from Rust source before release. Native Windows
regeneration requires MSVC `link.exe`; WSL/Linux generation is supported.

## PM Runtime Readback

`GET /api/v1/agents/{agent_id}/readiness` requires central service access or
standalone operator/admin access.
An active/effective database revision does not imply that its runtime files
are intact. Fresh read-only filesystem verification adds
`effective_configuration_readback_failed` when the snapshot, marker, isolated
workspace or skills cannot be verified. Underlying paths, resolved credentials
and file hashes are not exposed. `effective_revision` still reports the database
head, not a successful runtime observation. Workflow admission remains a separate
blocker; this endpoint cannot authorize PM dispatch.
Hermes-owned skill categories and `.bundled_manifest` are not Fleet snapshot
files. They are preserved, not certified by managed-file readback. The separate
`runtime_skill_inventory_not_verified` blocker prevents treating intact managed
files as proof of complete skill inventory/native provenance.

`GET /internal/runtime/v1/pm/runs/{session_run_id}` is a machine-only callback
for Project Workflow, outside browser authentication. It requires the dedicated
`FLEET_CONTROL_PM__READBACK_TOKEN`; an unset/short/reused credential fails closed.
With a configured separate credential, exactly one `Authorization` header is
required. Missing or duplicate headers return `401` before bearer parsing,
repository access or runtime probing, including two equal valid bearer values.
The response is the flat Workflow `RuntimeObservation`, without a Fleet envelope.
It contains immutable Tracker/assignment/execution identity, Fleet run UUID,
binding, dispatch key, fence, checkpoint and a fresh observation UUID/status.

Fleet probes the authenticated Hermes `/v1/runs/{runtime_run_id}` on every call.
Runtime mapping mismatch, unknown status, unreachable runtime and malformed or
oversized replies return `503`; contradiction of stored terminal proof returns
`409`. Cached Fleet run state, EOF and a human-provided status are not proof.
The endpoint does not create assignments, dispatch a prompt or resume Workflow.

## Fleet alerts (monitoring, Phase 3)

- `GET /api/v1/fleet-alerts?state=open|acknowledged|resolved`: Operator+;
  canonical persisted kinds are `agent_down`, `agent_recovered`,
  `agent_restart_loop`, and `heartbeat_stale`. Heartbeat warnings apply only to
  `running` agents with a valid, nonfuture health timestamp older than ten
  minutes. Acknowledgement does not resolve an incident. Concurrent creation
  uses an agent-row transaction lock and returns the existing open/acknowledged
  incident. A fresh heartbeat resolves that incident even without a status
  transition, while leaving unrelated down/loop alerts unchanged. Missing or
  future health timestamps and nonrunning statuses do not prove recovery.
  Explicit failed/stopped-to-running/ready health recovery resolves active
  down/loop/heartbeat incidents. Resolution and its redacted audit commit
  together. Legacy `agent_heartbeat_stale` is a display alias only, not a
  supported database kind.
- `POST /api/v1/fleet-alerts/{alert_id}/acknowledge` — Operator+; ack только для `open`-алертов; аудит `fleet_alert.acknowledge`.
- Переходы пишутся в `fleet_alerts` (миграция 6) из start/stop/health операций без блокировки ответа.

## Durable Clarification Answer Commands

All paths below include `/api/v1`. These are Fleet journal endpoints, not new
Tracker capabilities. Each request requires a verified human, the local session
owner and fresh Tracker project/immutable-binding authorization before journal
access. Operator history access alone is insufficient. Store/delivery also
require the bound current PM agent; history readback does not require the old
assignment to remain current. No human bearer token is persisted.

The trusted `VerifiedHumanSession` middleware extension is mandatory before
local session, Tracker context or journal access, including legacy answer
ingress. A sessionless owner-subject PAT or a caller-supplied human header is
not a human session and receives `401` without journal reads or delivery.

- `POST /sessions/{session_id}/clarifications/{question_id}/answer-commands`:
  existing `ClarificationAnswerRequest` body, including original `idempotency_key`.
  Returns `200 ClarificationAnswerCommand` after committing custody, without
  delivering to Tracker. Exact key/body replay returns the same command;
  changed target/body or another unresolved command for the task/question/owner
  conflicts. This also fences attempts through another chat or assigned agent.
- `GET /sessions/{session_id}/clarification-answer-commands`: returns unresolved
  `stored`, `delivering`, `uncertain` receipts, oldest first. More than 100 is
  fail-closed, never silently truncated. An empty list is not an exact historical
  command acceptance receipt.
- `GET /sessions/{session_id}/clarification-answer-commands/{command_id}`:
  reads that owner's exact Fleet receipt, including terminal state; never sends.
- `POST /sessions/{session_id}/clarification-answer-commands/{command_id}/delivery`:
  no replacement body/key. Explicitly claims one fenced delivery attempt, using
  the saved body/key against Tracker's existing clarification answer POST.
  Returns `200` with the current receipt; HTTP success alone is not delivery.

The receipt contains `id`, `session_id`, `question_id`, original `request`,
`payload_sha256`, `state`, nullable `answer`/`rejection_status`, `created_at` and
`updated_at`. These owner-private responses use `Cache-Control: no-store`.
They contain the private original answer for recovery; do not put them in logs,
URLs, browser persistent storage or shared caches.

Only an exact typed Tracker `200` answer with matching question/version/revision,
owner and payload proves `delivered`. Unknown transport/status/body stays
`uncertain`. A known first-attempt rejection may be `rejected`, but a later
rejection cannot erase earlier uncertainty. Expired attempts recover only the
same command with a new internal attempt token, never a new business key.
Concurrent live attempts return the existing receipt without another POST.

The legacy `POST .../clarifications/{question_id}/answers` now uses the same
journal and retains its successful `TrackerAnswer` response; unresolved delivery
returns `503`. Saved-answer GET is not original-key lookup. No Tracker exact-
command GET, unattended background delivery, PM continuation, runtime run or
requirements publication is introduced. Actual Rust codegen37999711562 produces
the four routes/two new DTOs without changing existing API. The authenticated
schema is integrated atb249ee5 and TypeScript regeneration/openapi:check pass;
strict final integrated Rust schema comparison and behavioral compatibility checks
remain a coordinated pre-release gate, not completed evidence for this unit.
See [the source boundary](plans/2026-10-10-clarification-command-custody.md).

## Общая база

Подключение версий, границы контрактов и проверки описаны в [BASE_INTEGRATION](BASE_INTEGRATION.md).

## Configuration Foundation Release Candidate

This packet extends foundation47 `8befcb6`; its own Linux tests and generated
OpenAPI parity are still pending. It adds no migration and does not enable SDLC
assignment, native admission or automatic dispatch.

### Pinned Package Draft

`POST /api/v1/agents/{agent_id}/config/base-package` requires an operator/admin.
There is no request-controlled checkout or revision. The server reads regular Git
blobs at Base `4b9b4c9297a13fb28a6ba2039af2f7cb719f2f58` from
`fleet.base_package_checkout`, independently of SDK `.base-revision`.
All seven roles, fourteen skills, hashes and inventory are verified before a new
draft freezes the concrete role instruction, allowlisted skills and fresh
Workflow v3 mapping. Other skills are disabled in the draft; active files,
effective head and runs are untouched. The protected revision response includes
SOUL/skill content, as existing operator revision reads do; errors/audit do not.

Missing checkout/package, wrong role/runtime/profile: `422`; changed desired
revision/identity/drain: `409`; Workflow unavailable/denied: `503`.
Validate, activation request, supervisor preflight and readiness re-read the
[Workflow binding](contracts/SDLC_WORKFLOW_BINDING_V1.md). Package snapshots are
reverified against Git, not trusted because they contain a proof JSON field.
Preflight failure before mutation preserves the old head/files and releases drain;
unverified rollback retains the existing fail-closed drain behavior.

History remains latest-100; validation/activation use exact agent/revision lookup,
and readiness loads the effective head directly. An older draft may validate but
cannot activate unless it is the current desired validated revision.
Role/namespace/workflow/product-role changes and rebind are fenced under the
agent row lock against drain, unresolved runs and pending/dispatching/uncertain
outbox work. Unchanged identity fields permit metadata-only updates when not draining.

### Machine Configuration Observation

`GET /internal/runtime/v1/agents/{agent_id}/configuration` is opt-in and outside
human/browser authentication. Exactly one Authorization header is required before
token parsing or HTTP. A fresh Base PAT introspection must identify the registered
canonical subject, exactly one `fleet-control:read` scope and an allowed concrete
agent UUID. Wildcards, write/duplicate scopes, revoked tokens and browser/local
credentials fail closed; no user is created. Transport has no retry/redirect/proxy,
identity encoding only, five-second timeout and 16 KiB response bound.

The response contains public package proof, frozen Workflow mapping, agent/role,
effective revision, observation UUID/time and blockers, with `Cache-Control: no-store`.
It excludes SOUL/skill content, env values, paths and credentials. Git provenance,
managed files and a fresh owner mapping must verify; agent/effective head are
re-read to detect concurrent change/drain. Missing effective package/drain/change:
`409`; invalid configuration or unavailable provenance/files/owner: `503`;
missing/duplicate/invalid/revoked credential: `401`; foreign subject/scopes/agent:
`403`. Disabled/invalid server authority returns `503` before authentication IO.

For package snapshots, HOME skills must contain exactly the expected flat
`<skill>/SKILL.md` files. Unattested support/scripts, flat Markdown, nested/case
aliases, links/special entries and Unix hardlinks are rejected without mutation
(maximum 4096 entries, 16 directory levels). Legacy non-package managed-file
verification is unchanged. Project/external/plugin discovery and loaded native
settings remain unverified. `runtime_ready=false` is unconditional: this is not an
admission receipt, lease, assignment ACK or authority to dispatch.
