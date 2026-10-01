# SDLC: реализация и оставшаяся приёмка

Дата: 1 октября 2026. Статус: частичная реализация foundation; автоматический
SDLC не включён. Этот документ уточняет исторические отметки `done` в
IMPLEMENTATION_PLAN и CURRENT_STATE. Они не являются приёмкой нового SDLC.

## Границы

- Tracker: задачи, требования, редакции, назначения, очередь, leases, переходы.
- Fleet: реальные агенты, конфигурационные редакции, чаты и runtime-запуски.
- Project Workflow: версии workflow, шаги, checkpoint/rebind, terminal receipt.
- CI/CD: pipelines, exact SHA, артефакты, deployment и acceptance receipts.
- Base: общая identity и UI utilities, без доменной логики SDLC.

Лиды отложены. Их данные, старые URL и история сохраняются, но разделы лидов
и исполнителей убраны из основной навигации. `/chats` не предлагает управление
лидом или handoff. `/sessions` остаётся legacy-маршрутом с прежними controls.
Смена агента SDLC-задачи в будущем создаёт отдельный чат, не меняет существующий.

## Реализовано в этом изменении

| Область | Реализация | Доказательство / предел |
| --- | --- | --- |
| Специализации | Семь `SdlcRole`, поле `sdlc_role`, формы создания/редактирования | Миграция 000009; developer/tester backfill только для executors |
| Каталог | `/agents`: управление для operator/admin, read-only directory для user | Секреты и runtime paths не запрашиваются read-only страницей |
| Чаты | `/chats`: конкретный агент → его сессии, поиск, аватар владельца, private create | Отдельные агенты не смешивают transcript; task binding ещё не реализован |
| Доступ | Central subject сохраняет локальную роль, не получает effective-admin | Bootstrap admin только по явно настроенному проверенному subject |
| Messages | Авторизация owner/operator; человеческий API не позволяет выдавать себя за агента | PostgreSQL regression test; отдельная machine identity ещё не реализована |
| Idempotency | Session key сериализуется transactional advisory lock; message key — session row lock | Concurrent replay и payload conflict проверены на PostgreSQL |
| Dispatch | Prompt и outbox создаются одной транзакцией через trigger | Claim блокирует agent row; capacity 1; pending не отправляется при drain |
| Unknown acceptance | Нет автоматического redispatch после неопределённой отправки | `uncertain` удерживает слот; recovery operator API ещё отсутствует |
| Runs | Новые попытки сохраняют отдельные runtime run records | Completion mirror дедуплицируется по runtime ID под session row lock |
| EOF | EOF без terminal event требует status readback | Non-terminal/error оставляет waiting, а не ложный completed |
| Stream | PostgreSQL events и per-session cursor; `Last-Event-ID`; проверка доступа на каждом poll | Durable replay; retention/expired-cursor snapshot ещё не реализованы |
| Redaction | Известные env secrets и derived token; stream snapshots удерживают split-secret suffix | Unit regression; произвольный неизвестный секрет не гарантированно обнаруживается |
| Legacy config | Secret-поля старого DB-конфига маскируются при чтении, валидные `secret_ref` сохраняются | Regression проверяет nested/array credentials и обычный task key |
| Config | Immutable snapshot config+skills, draft/validated/activating/active/failed | Desired/effective разные; activation drain, readback, tracked-runtime restart, rollback |
| Failed rollback | Агент остаётся drained, effective revision не меняется | PostgreSQL regression; recovery после crash требует дальнейшей реализации |
| Filesystem | Не присваивать непустую чужую папку, проверять marker и symlink/junction components | Guard действует на managed writes; защита от внешнего TOCTOU требует OS isolation |
| Provisioning | Существующий effective `.env` сохраняется; активный runtime не переподготавливается | Exclusive create и Unix `0600`; три regression tests |
| Readiness | Runtime health отдельно от readiness SDLC | `workflow_assignment_protocol_not_verified` блокирует весь автоматический SDLC |
| Shared UI | Совместимое расширение bearer SSE utility в Base | Cursor сохраняется при reconnect, credentials только в header |

## Контракт конфигурации

1. `PUT /agents/{id}/config` сохраняет draft и desired snapshot, не применяет
   файлы работающему Hermes.
2. `GET /agents/{id}/config/revisions` показывает последние 100 редакций.
3. `POST /agents/{id}/config/revisions/{revision}/validate` проверяет структуру,
   SOUL, skills content и доступность secret refs.
4. `POST .../{revision}/activate` принимает только desired validated revision.
   Новые dispatch/provision/start/restart и конфигурационные изменения блокируются.
5. Worker ждёт отсутствия активных runs и неопределённых dispatch, проверяет marker,
   применяет файлы, readback и readiness ранее работавшего tracked runtime.
6. Успех переключает effective revision. Ошибка с подтверждённым откатом оставляет
   старую effective revision; непроверенный rollback сохраняет drain.

Секрет в новом env задаётся как `{"PROVIDER_API_KEY":{"secret_ref":"PROVIDER"}}`.
Оператор предоставляет `FLEET_CONTROL_SECRET__PROVIDER` процессу Fleet.
Fleet разрешает reference только в managed `.env`, не наследует Fleet credentials
в Hermes process environment. Ref не является значением секрета.
API_SERVER_KEY генерируется Fleet, изменять его через draft нельзя.

Skills capture выполняется при сохранении draft. Изменение skill в БД само по себе
не означает применение; после редактирования нужно сохранить новую config revision.
Java lifecycle сохраняется, но chat/control и config activation остаются phase 2.

## Сверка с текущим Hermes

Read-only source review: Hermes HEAD `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3`.
`gateway/platforms/api_server_run_idempotency.py` уже содержит durable reservation
по scope/key, fingerprint conflict, retention и persisted status. Capability
`features.runs_idempotency` объявляет supported/durable/retention_seconds; при
недоступной SQLite durable становится false. Fleet не пишет в эту SQLite.

Это подтверждает наличие механизма в исходниках, но не доказывает capabilities
установленного агента или прохождение Hermes tests в текущем окружении. Перед
автоматическим recovery нужны pinned/runtime version, проверка durable capability
и acceptance/replay integration. До этого unknown dispatch не повторяется.
Terminal `interrupted` трактуется как failed, без fabricated assistant reply;
это проверено отдельным fake Hermes HTTP regression.

## Оставшиеся блокеры по этапам

| Этап | Обязательная работа | Владелец |
| --- | --- | --- |
| 1 | Исполняемые versioned assignment/result/receipt контракты и capability matrix | Все четыре сервиса |
| 2 | Machine scopes, project access, проверенный central subject между сервисами | Tracker/Fleet/Workflow/CI |
| 2 | Tracker requirement revisions, confirmation exact revision, structured clarification | Tracker |
| 2 | Immutable task/root IDs, real parent/root decomposition, cycle validation | Tracker |
| 2 | Project role-agent snapshot, workflow/config revisions, deployment policy snapshot | Tracker |
| 2 | Assignment leases, heartbeat/fencing, outbox/inbox и monotonic side-effect checks | Tracker + consumers |
| 3 | Unique task-ID/agent-ID chat binding, замена агента без handoff старой истории | Fleet/Tracker |
| 3 | Recovery dispatching/uncertain после crash без повторной неизвестной команды | Fleet/Hermes |
| 3 | Доказательство server-side Hermes idempotency/acceptance lookup | Hermes/Fleet |
| 3 | Activation journal на диске, crash recovery, operator reconciliation и audited force-cancel | Fleet |
| 3 | Config snapshot workflow/model prerequisites, skill auto-revision UX | Fleet |
| 3 | Stream retention/reset snapshot, bounded payloads, transcript pagination, targeted approval | Fleet |
| 4 | PM Draft creation saga, checklist, publication, escalation/checkpoint/resume | Tracker/Fleet |
| 4 | 11 modes, first-step gate, bind/rebind/history/terminal evidence | Workflow + Fleet |
| 5 | Pool 2, capacity 1, root lease 1 incl children, PM outside technical pool | Tracker |
| 5 | DAG delivery/integration и Rework в том же agent/task chat, предел 3 циклов | Tracker/Fleet |
| 5 | Existing CI pipeline mandatory, resolved exact SHA, candidate root branch | CI/CD |
| 5 | Task workspace, manifest deploy, receipts, health/acceptance, manifest rollback | CI/CD |
| 5 | Retry 1/5/20 только с доказанной безопасностью; manual resume audit | Все consumers |
| 6 | Cross-service contract, crash, stale fencing, cross-user stream, fake-runtime HTTP tests | Все сервисы |
| 6 | Chromium/Firefox/WebKit, real seven-agent scenario и deployment evidence | Fleet + integration environment |
| 7 | Opt-in project rollout, disable новых assignments без удаления/kill истории | Tracker |

## Протокол целевого SDLC

Стадии: Draft, Backlog, Analysis, Architecture, Development, Review, Testing,
Rework, Deployment. Completion основной задачи требует Deployment + acceptance;
успешный run не завершает стадию самостоятельно.

11 режимов: `pm_draft`, `analyst_analysis`, `architect_decomposition`,
`developer_initial`, `developer_rework`, `reviewer_delivery`, `reviewer_integration`,
`tester_delivery`, `tester_integration`, `devops_delivery`, `devops_integration`.
Это target contract, не опубликованные/работающие workflow definitions.

Execution `SDLC-<ordinal>` не равен chat UUID и display task key. Envelope содержит
contract version, central owner subject, project/task/root IDs, requirement revision,
assignment ID, execution ID, attempt, fencing token, payload hash и idempotency key.
Старый fencing token/requirement revision не принимаются. Lease TTL 30s, heartbeat
10s; после expiry нельзя повторно назначать, пока прекращение старого run не доказано.

PM уточняет бизнес-неопределённости до публикации. После публикации критический
business escalation возвращается в PM root chat, infra failure — оператору.
Clarification и approval разные сущности; поздний ответ старому request/run не
может возобновить новое execution. Обычное сообщение не является stage transition.

## Приёмка

Сквозной сценарий остаётся невыполненным: семь изолированных Hermes; PM clarification
и публикация; root и две child tasks; дефект/Rework; integration; настоящий deploy
и acceptance receipt. Fixture screenshots не заменяют этот сценарий.

Проверки foundation: PostgreSQL integration в `backend/infra/tests/sdlc_foundation.rs`,
unit redaction/path guards, frontend Chats/nav, shared SSE reconnect tests.
Актуальные результаты команд записываются в CURRENT_STATE после выполнения.
Ни текущий интерфейс, ни отсутствие ошибки HTTP не означают «100% SDLC готов».
