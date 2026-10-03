# SDLC execution v1 — target contract

Статус: **Target approved**, 2026-10-02; не implemented API. Нормативный регламент
находится в services-base `docs/platform/SDLC.md`. Этот контракт принадлежит Fleet;
Tracker определяет назначения и переходы, Workflow — cursor/terminal receipt,
Forge — workspace и delivery receipts. Base не становится orchestrator.

## Assignment v1

Tracker выпускает immutable envelope; Fleet валидирует и dispatch-ит его.
Все refs opaque, project context устанавливается backend, не моделью.

| Поля | Тип / инвариант |
| --- | --- |
| contractVersion | `base-sdlc/assignment/v1` |
| tenantId, ownerSubject, projectId, taskId, rootTaskId | Непустые owner-resolved identity refs |
| requirementRevision, taskRevision, decompositionRevision | Positive revisions; decomposition nullable до принятия |
| stage, role, workflowKey, mode, scope | Только Tracker binding; scope business/delivery/aggregate |
| assignmentId, executionId, sessionId, runId | Разные identity, не display ticket; immutable после accept |
| agentId, namespace, profile | Конкретный агент и effective config, не только роль |
| cycleNumber, attemptNumber | Initial cycle 0; rework увеличивает cycle; retry увеличивает только attempt |
| fencingToken, leaseId, leaseExpiresAt | Monotonic owner fencing; TTL 30 s, heartbeat 10 s |
| operationKey, payloadHash | Stable key + canonical JSON SHA-256; другой payload того же key — conflict |
| pinnedInputs[] | owner/type/ref/revision/sha256; immutable требования, findings, predecessor receipts |
| configuration | Fleet config revision, workflow catalog SHA/hash, exact skills commit и manifest hash |
| allowedSkills[], allowedActions[], allowedPaths[] | Пересечение config, фазы и role policy; не права из prompt |
| workspaceLease | Nullable PM; Forge resource identity, pinned source/branch, generation и permissions |

Проверка schema, подписи/доверенного issuer, tenant, project access, binding,
configuration и freshness выполняется до модели. Missing/mismatch fail-closed.
Task display key, workflow cursor и Hermes session не взаимозаменяемы.

## Result v1 и receipts

`base-sdlc/result/v1`: assignment/execution/run/task refs, revisions, fencing,
operationKey/payloadHash, cycle/attempt, outcome, requirementCoverage[], checks[],
evidenceRefs[], findings[], workflowTerminalReceiptRef, forgeReceiptRefs[],
finalCommentRef. Outcome `passed` либо `needs_rework` (только Reviewer/Tester);
awaiting-input checkpoint — отдельный протокол, не successful result.

Fleet terminal run event подтверждает состояние исполнения, но не Task transition.
Workflow receipt подтверждает accepted report и `complete=true`, mode/scope/cycle,
cursor revision, config hash и pinned assignment. Forge receipt подтверждает
exact source/artifact/config/target и resource cleanup. Tracker terminal receipt
`base-sdlc/transition-receipt/v1` содержит прежнюю/новую revision и stage/status,
result hash, accepted receipt refs, ReworkRequest/cycle (если применимо), eventId,
operationKey и commit timestamp. Consumer не принимает self-authored receipt модели.

Version negotiation обязательна; неизвестная major version отклоняется. Minor
расширение только с capability agreement. Pinned history никогда не нормализуется
задним числом. Trusted receipts доступны по owner lookup и immutable hash.

## Dispatch, чат и recovery

1. Tracker атомарно пишет assignment и свой outbox. Fleet durable inbox дедуплицирует
   eventId/key/hash; transport ACK только после durable accept.
2. Fleet резервирует capacity 1 конкретного агента и создаёт/находит чат
   `(tenant, taskId, agentId)`, Hermes session binding и starting run.
3. Runtime capability lookup должен подтвердить durable idempotency и acceptance
   lookup. Unknown dispatch удерживает capacity до reconcile, не redispatch.
4. Step admission подтверждает assignment/run/fencing. Starting/running состояние
   после restart продолжается по прежним IDs и ключу, не создаётся новая сессия.
5. Active message — steer с messageId; между turn-ами новый run той же сессии.
   После terminal stage ввод до нового assignment блокируется.
6. Developer initial/rework одного агента сохраняют чат; replacement agent получает
   другой чат без автоматического копирования истории. `@mention` не назначает работу.
7. Final result доставляется владельцу. Resource release выполняет владелец после
   durable terminal acknowledgement, не после EOF/HTTP 200/истечения lease.

При clarification: request относится к exact revision/run; сохранить unfinished
phase и pinned inputs; writable diff checkpoint с remote SHA; durable awaiting-input
ACK освобождает ресурсы. Ответ принимается один раз, свежий lease/run получает CAS
rebind прежнего cursor/mode/scope/cycle. Старый ответ и stale result отклоняются.
Approval — отдельный scoped human gate; обычное сообщение не разрешает transition.

## Права и capabilities

Требования к семи concrete Hermes, effective model/tools/limits, source-only
example и validate/drain/activate/readback/recovery описаны в
[runtime configuration v1](../SDLC_RUNTIME_CONFIGURATION_V1.md).
Это дополнение B-SDLC-02, не установка и не изменение существующего API.

Канонические role-инструкции/skills и декларативный manifest принадлежат Base
(`services-base/agent-skills/`). Fleet materializes только pinned allowlist и
roleInstruction, проверяет hashes и хранит effective config с model/provider,
limits и concrete identity. Редактируемой копии пакета во Fleet нет; отсутствие
доступа к приватному Base pin является blocker, не fallback на старые файлы.
Будущий prompt assembly соединяет pinned roleInstruction, backend assignment,
текущую фазу Workflow и разрешённые skills после admission. Этот контракт
не реализует installation или сборку промпта и не меняет runtime.

| Capability | Разрешение | Текущий статус / работа |
| --- | --- | --- |
| Project context read | Все роли, assigned project | Нужен end-to-end machine-scoped contract; context token read-only |
| Draft guarded writes/publication | PM | Target; PR PM foundation не считать acceptance |
| Decomposition/materialized children | Architect | Target Tracker capability |
| Candidate branch write | Developer | Target Forge role credential; не общий PAT |
| Candidate review/test | Reviewer/Tester | Read + свои reports/findings; без feature write |
| Build/deploy/acceptance | DevOps | Existing Forge contour + target receipts/policy |
| Terminal stage request | Назначенная роль | Только после Workflow receipt; следующий stage выбирает Tracker |
| Config/skills/agent start | Fleet operator | Модели запрещено; existing config snapshots/drain переиспользуются |

Общие Business/admin token, Tech PAT, Docker socket и SSH keys не передаются.
Раздельный HOME не является OS isolation. Context failure не завершает Task failed.
`/live` — process; readiness SDLC дополнительно проверяет bindings, machine identities,
Workflow/Hermes capabilities, Forge pool и владельцев. Неподтверждённая capability
сохраняет существующий readiness blocker.

## Обязательные contract tests реализации

Duplicate/conflicting payload; crash before/after durable accept; uncertain dispatch;
lease expiry при живом run; delayed result; stale confirmation/answer; scope/token
substitution; cross-tenant lookup/title/count/stream; mode injection; config activation
во время run; restart checkpoint/rebind; resource ACK потерян и повторно прочитан.
Сквозная приёмка остаётся отдельной вехой, не результатом этого документа.
