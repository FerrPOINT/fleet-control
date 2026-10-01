# Чаты и сессии Fleet Control

## Актуализация 1 октября 2026

Новый основной маршрут `/chats`: реальные агенты и внутри их отдельные сессии.
Лиды отложены, controls выбора лида/handoff/delegation здесь не показываются.
`/sessions` сохраняет исторический интерфейс. Семь SDLC-специализаций отделены
от runtime kind и legacy product role. Обычный private chat не запускает SDLC.

Реализованы transactional message outbox, сериализация concurrent idempotency,
durable per-session stream cursor, owner checks, terminal readback после EOF,
config revisions/drain. Реализована immutable-привязка Tracker instance/task/agent и
production-вкладки диалога, уточнений и требований. Автономный SDLC и реальное
продолжение PM после ответа пока не реализованы.
Далее сохранена спецификация и аудит legacy-раздела на прежнем HEAD; её open gaps
нельзя автоматически считать закрытыми новым UI. Текущий статус и оставшаяся работа:
[SDLC implementation](SDLC_IMPLEMENTATION.md).

Дата сверки: 30 сентября 2026 года.

Статус: единая спецификация раздела, сверенная с исходниками. Наличие этого
документа не подтверждает прохождение сквозной приёмки. Реализованное поведение,
целевые требования и открытые расхождения указаны отдельно.

Сверка выполнена на рабочем checkout с HEAD
`e465c43f8ce4f8610dc5f603d4d298d509652b04`. Проверены session routes, repository,
runtime supervisor, типы и страницы списка/деталей сессий. Это source review,
а не проверка работающего окружения.

Проверенные источники:

| Область | Исходник |
| --- | --- |
| Routes, HTTP authorization и SSE | [sessions.rs](../backend/api/src/routes/sessions.rs) |
| Central/legacy identity | [middleware/mod.rs](../backend/api/src/middleware/mod.rs) |
| Session/message/delegation persistence | [infra/src/lib.rs](../backend/infra/src/lib.rs) |
| Hermes dispatch, events и controls | [runtime/mod.rs](../backend/infra/src/runtime/mod.rs) |
| Типы и request schemas | [domain/src/lib.rs](../backend/domain/src/lib.rs) |
| Session list/create UI | [sessions/index.tsx](../frontend/src/pages/sessions/index.tsx) |
| Transcript и controls UI | [session-detail/index.tsx](../frontend/src/pages/session-detail/index.tsx) |

Целевые требования ниже требуют реализации и evidence; предложенные уточнения
central access, direct-leader removal и workflow continuation должны получить
отдельное решение владельца продукта/соответствующего контракта.

## 1. Назначение и границы

Чат позволяет человеку работать с исполнителем или управлять командой через
лида. Fleet хранит владельца, участников, сообщения, связи и аудит; runtime
выполняет запрос и возвращает события. Переписка и выполнение имеют разные
состояния: принятый HTTP-запрос ещё не означает доставку или успешный результат.

Одна Fleet-сессия имеет одного владельца-человека, одного основного агента и
не более одного выбранного лида. Несколько запусков агента продолжают историю
этой сессии. Делегация создаёт отдельный дочерний чат исполнителя.

`AgentKind = hermes | java_agent` определяет протокол исполнения.
`AgentProductRole = leader | executor` определяет продуктовые обязанности.
Профиль, prompt/SOUL, skills и workflow binding определяют специализацию.

Fleet не записывает сообщения напрямую в Hermes SQLite/SessionDB. Запись в
runtime проходит только через adapter. `project-workflow` владеет определениями
workflow и namespaces; сообщение в Fleet не является командой завершения стадии.

Связанные документы: [архитектура](ARCHITECTURE.md), [модель данных](DATA_MODEL.md),
[API](API.md), [runtime](RUNTIME.md), [workflow](WORKFLOW.md),
[handoff](contracts/SESSION_HANDOFF_CONTRACT.md), [безопасность](SECURITY.md).

## 2. Термины и владение данными

| Объект | Значение и источник истины |
| --- | --- |
| Fleet session | Пользовательский чат/задача; `agent_sessions` в Fleet |
| Owner | `agent_sessions.user_id`, человек, которому принадлежит чат |
| Primary agent | Адресат новых запросов; публично `primary_agent_id`, физически `agent_id` |
| Selected leader | Опциональный координатор; `leader_agent_id` |
| Parent/child | Связь координационного и делегированного чатов; `parent_session_id` |
| Participant | Участник и роль в конкретном чате; `session_participants` |
| Message | Сохранённый prompt, ответ, tool/control/system event; `session_messages` |
| Runtime session | История исполнения в runtime; для Hermes `fleet:<session_id>:<agent_id>` |
| Run | Одна попытка исполнения; `session_agent_runs` и соответствующий runtime run |
| Approval | Запрос разрешения конкретного run; `runtime_approval_requests` |
| Audit | Кто инициировал изменение и что изменилось; `audit_log` |

`task_key` связывает чат с внешней задачей, но не превращает Fleet в task tracker.
Существующий `external_session_id` не заменяет per-agent runtime links.
Созданная запись pending run не доказывает, что runtime уже выполняет работу.

## 3. Создание чата и приватность

| Сценарий | Primary | Leader | Visibility | Parent и owner |
| --- | --- | --- | --- | --- |
| Человек пишет исполнителю | Executor | `null` | `private` | Нет parent; текущий человек |
| Человек пишет лиду | Leader | Тот же Leader | `leader_scoped` | Нет parent; текущий человек |
| Делегация из чата с выбранным лидом | Managed Executor | Лид родителя | `leader_scoped` | Parent обязателен; owner родителя |
| Человек выбирает лида в чате исполнителя | Текущий Executor | Managed Leader | `leader_scoped` | Owner сохраняется |
| Человек снимает лида с чата исполнителя | Текущий Executor | `null` | `private` | Owner и история сохраняются |

Создание самого чата не обязано запускать runtime. Непустое `initial_message`
при делегации является отдельной сохраняемой и доставляемой командой.
Текущая delegation route допускает любой leader-scoped parent, включая чат
исполнителя с выбранным лидом; UI обязан показывать фактического координатора.

При создании backend проверяет существование и доступность primary, роль лида,
связь `leader_executors`, доступ к parent и совместимость owner/leader.
Переданный клиентом `parent_session_id` не является доказательством доступа.
Для legacy `agent_id` действует тот же контракт, что для `primary_agent_id`.

Приватность означает отсутствие доступа у неприкреплённых runtime-агентов.
Она не исключает явно разрешённый операторский доступ. Такой доступ должен
быть понятен пользователю и отражён в политике прав.

## 4. Права человека и identity агента

Целевая матрица ранее согласованного Fleet MVP:

| Действие | Owner | Другой обычный пользователь | Operator/admin | Выбранный лид |
| --- | --- | --- | --- | --- |
| Читать сообщения, runs, участников | Да | Нет | Да, при разрешённом scope | Только привязанные чаты через machine identity |
| Отправлять сообщение | Да | Нет | По единой серверной политике с аудитом | Только в привязанный чат |
| Stop/steer/approval | Да, если capability позволяет | Нет | Да, с аудитом | Только явно выданные machine permissions |
| Выбрать/снять лида, handoff | Да, с проверками | Нет | Да, по единой политике | Не самостоятельно |
| Делегировать | Да, если есть managed executor | Нет | По единой политике с сохранением owner | Только с ограниченным delegation scope |
| Читать приватный чат другого исполнителя | Владелец своего чата | Нет | По операторскому разрешению | Нет |

Фильтр пользователей по умолчанию содержит текущего человека с аватаром/иконкой.
Удаление последнего chip означает всех только при `sessions:read_all`.
Без `user_id` backend возвращает текущего пользователя; `user_id=all`, пустое
значение и несколько чужих UUID требуют права расширенного чтения.

У identity человека и identity агента разные полномочия. UI-поле
`author_agent_id` не подтверждает, что действие выполнил автономный лид.
Нужно сохранять как отображаемого автора, так и реального инициатора команды.
Runtime credential не должен автоматически давать доступ к Fleet API.

Текущее поведение имеет две существенные особенности:

- В Central Auth middleware любой допущенный человек получает эффективную
  роль `admin`; default own filter сохраняется, но расширение на всех доступно.
  Legacy `user` ограничен своими сессиями. Нельзя обещать изоляцию людей,
  ссылаясь только на legacy RBAC. Политика central scopes требует решения.
- API разрешает запись owner и operator/admin, но repository сообщений и
  делегаций проверяет owner. Поэтому чтение чужого чата не гарантирует право
  отправки или делегации; это расхождение требуется устранить.

Права проверяются для detail/messages/participants/runs/stream и каждой команды,
а не только для списка или навигации. Заголовки, previews, counts и дочерние
ссылки также проходят проверку до раскрытия данных.

## 5. Лидер, команда и делегация

Лид управляет только явно назначенными исполнителями. Наличие исполнителя в
команде не открывает все его пользовательские чаты. Доступ лида определяется
выбранным `leader_agent_id` конкретной сессии.

Делегация должна атомарно закреплять parent, child, owner, leader, executor и
начальную команду. Повтор с тем же ключом и полным payload возвращает тот же
child и ту же команду; изменение даже одного поля возвращает `409`.

Исполнитель получает контекст задачи, предназначенный для него. Добавлять всю
команду в participants parent или копировать все transcripts запрещено.
Возврат результата в parent требует отдельной ссылки/summary с проверками прав
и защитой от повторной публикации. Автоматическая агрегация пока не подтверждена.

Сообщение лида в чате исполнителя доставляется primary executor, а не runtime
лида. В direct leader chat адресат и выбранный лид совпадают.
Текущий UI позволяет человеку выбрать автора «лид», но не передаёт
`message_kind=user_prompt`: repository по умолчанию создаёт для agent author
`assistant_message`, а supervisor такой message только зеркалит. Поэтому
эта кнопка ещё не доказывает реальную отправку команды исполнителю.

Автономному лиду необходимы ограниченные tools/API: создать child, прочитать
разрешённую историю, отправить команду и получить результат. Backend выводит
identity лида из credentials, проверяет session scope и текущую team binding.
Передача произвольного `author_agent_id` человеком не заменяет этот канал.

Удаление исполнителя из команды блокирует новые команды лида в соответствующие
чаты. История сохраняется. Уже принятые runs должны быть остановлены или
доведены до terminal по явно согласованной политике; права не расширяются.

## 6. Участники и смена координатора

Owner, primary и selected leader представлены типизированными participants.
`observer` в модели не является готовым механизмом приглашения людей:
отдельных invitation/ACL API сейчас не подтверждено.

Смена лида после появления сообщений требует подтверждения: новый лид получает
доступ к разрешённой истории чата. Для активного run сначала нужен stop/reconcile;
backend должен сериализовать изменение или вернуть `409` с причиной.
При снятии лида новые чтения/команды старого лида и открытые stream прекращаются.
Уже раскрытый контекст невозможно отозвать из памяти runtime.

После смены пересчитываются participants, доступные действия и badges.
История старых runs и авторство сообщений сохраняются. Текущий repository
удаляет leader-role runs при смене лида; это расходится с сохранением истории.

Для direct leader chat снятие координатора требует отдельного решения:
первоначально leader = primary. Пока правило не закреплено, UI не должен
представлять такую операцию как обычное превращение executor chat в приватный.
Текущий backend допускает `leader_agent_id=null` и для primary leader.

## 7. Handoff и runtime-контекст

Handoff переносит primary в существующей Fleet-сессии. Делегация создаёт новую
сессию. Обе операции сохраняют владельца и не передают владение другому человеку.

При handoff проверяются целевой runtime, актуальная team binding, права и
отсутствие незавершённого исполнения. Обновляются primary participant и namespace;
создаётся новый runtime link. Старые runs остаются историей.

Изменение `agent_id` в БД не переносит Hermes-контекст автоматически:
`fleet:<session_id>:<old_agent>` и `fleet:<session_id>:<new_agent>` различаются.
Требуется versioned context package с разрешённым summary, task refs и нужными
результатами. Отправка этого пакета должна иметь receipt и idempotency key.
Текущий handoff contract оставляет синхронизацию adapter; автоматический перенос
контекста не следует считать реализованным.

Смена primary на другого лида должна одновременно согласовать selected leader
и visibility; нельзя сохранять комбинацию «primary leader A, selected leader B».
После операции UI перечитывает session, participants, runs и доступные лиды.

## 8. Состояния и разрешённые переходы

Существующие enums:

| Уровень | Значения |
| --- | --- |
| Session | `draft`, `active`, `handoff_requested`, `blocked`, `done`, `archived` |
| Run | `pending`, `running`, `waiting`, `stopping`, `completed`, `failed`, `cancelled` |
| Message delivery | `pending`, `dispatched`, `completed`, `failed`, `mirrored` |
| Approval | `pending`, `approved`, `denied`, `cancelled` |

Целевое поведение run:

```text
pending -> running -> completed
              |    -> failed
              |    -> waiting -> running
              |    -> stopping -> cancelled
```

Это схема обычного пути, а не доказательство внедрённой state machine.
При гонке stop/completion принимается подтверждённый terminal runtime state;
успешный stop HTTP response означает `stopping`, а не немедленный `cancelled`.
`waiting` должно иметь причину: approval, human input или external dependency.
`failed` доставки после транспортной ошибки не доказывает, что run не выполняется.

Session state не равен run state. Завершение одного ответа не завершает задачу;
`done` не должно выставляться по закрытию SSE. `archived` исключает новые команды,
но сохраняет историю. Полный lifecycle API для всех session states ещё не
подтверждён, и наличие enum не означает доступную пользовательскую операцию.

Для одной пары session/primary допускается не более одного изменяющего контекст
активного run. Guard должен действовать в БД/dispatcher, включая несколько
вкладок и экземпляров Fleet. Параллельные child sessions могут исполняться в
пределах capacity runtime и lease рабочего workspace.

Пока этот guard не доказан, обычный submit во время active run может создать
новый run. Целевой UI предлагает steer для поддерживаемого runtime или явно
сообщает, что нужно дождаться terminal; скрытая очередь без контракта недопустима.

## 9. Доставка, идемпотентность и восстановление

Текущий Hermes path:

1. Backend проверяет человеческие права и сохраняет сообщение.
2. Repository применяет idempotency key/hash.
3. Supervisor готовит Fleet run и вызывает `POST /v1/runs` синхронно в HTTP request.
4. Hermes получает Bearer runtime token, `Idempotency-Key=<Fleet message UUID>`,
   input и стабильный runtime session id.
5. Принятый runtime run id сохраняется; worker читает Hermes events в фоне.
6. Fleet сохраняет итог и mirror events, публикует изменения для UI.

В текущем path только чтение events фоновое; durable outbox/dispatcher не
подтверждён. Ответ message API может описывать сохранённое сообщение до
последующего обновления delivery state. UI должен перечитать delivery/runs,
а не трактовать любой HTTP `200` как успешное исполнение.

Целевые гарантии:

- Session key scoped по owner; message key по session и реальному инициатору.
  Клиент сохраняет один key и неизменный payload при сетевом retry.
- Delegation key покрывает весь payload, включая `initial_message`. Сейчас
  создание child и initial message отдельны; конфликт может оставить child.
- Concurrent replay сериализуется; unique violation переводится в readback
  или payload conflict, а не в непонятный `500`.
- Сообщение и dispatch intent фиксируются атомарно. Worker выполняет доставку
  с устойчивым ключом и lease; restart продолжает pending/unknown операции.
- После timeout между runtime acceptance и Fleet commit сначала выполняется
  readback/reconcile. Blind resend или новый ключ могут создать двойной run.
- Повтор terminal event не создаёт второй assistant message; нужен уникальный
  source event/result identity. Runtime run id сам по себе не уникален для всех
  сообщений: у одного run несколько tool events и один итог.
- Частичный ответ сохраняется или явно помечается неполным. EOF stream без
  terminal проверяется через `GET /v1/runs/{run_id}`, а не считается успехом.
- Restart Fleet восстанавливает наблюдение за живыми runs отдельно от process
  lifecycle reconciliation. Pending approval также восстанавливается.

Повтор прежнего message key возвращает прежний результат и не является кнопкой
«выполнить заново». Re-execute после подтверждённого terminal failure требует
новой явно связанной попытки; неизвестный результат сначала reconcile.

## 10. Streaming и протокол событий

Текущий Fleet endpoint: `GET /api/v1/sessions/{session_id}/stream`.
Каждый SSE frame имеет `event: session`; JSON discriminator `type` использует
snake_case. Keep-alive отправляется с интервалом 20 секунд.

| JSON type | Назначение |
| --- | --- |
| `session_changed` | Перечитать метаданные сессии |
| `session_message_changed` | Перечитать message; вложенное `event` может быть `message.dispatched` или `message.completed` |
| `session_run_changed` | Изменение run state и runtime link |
| `session_run_delta` | Временная текстовая delta конкретного run |
| `runtime_approval_requested` | Запрос разрешения, связанный с run |

Названия `message.created`, `message.delta`, `tool.event` из ранних планов не
являются текущими самостоятельными SSE event names. Tool/approval text
сохраняется в mirror; отдельная доставка каждого tool event не гарантирована.

Текущий stream использует in-memory broadcast: нет SSE `id`, replay journal,
`Last-Event-ID` или межпроцессного fanout; lagged события пропускаются. Проверка
доступа выполняется при подключении. Session detail пока использует polling
messages/runs раз в две секунды, а не этот stream.

Целевой клиент использует authenticated stream без токенов в query string,
bounded reconnect/backoff и deduplication. Delta отображается отдельно от
сохранённого итога; итог заменяет draft. На reconnect сначала/после подключения
сверяются persisted snapshots и cursor, чтобы закрыть окно потерянных событий.

Для гарантированного replay нужны durable event id/sequence, retention window
и обработка expired cursor через полный readback. События упорядочиваются внутри
session/run; нельзя сортировать только по клиентскому времени. История сообщений
должна иметь устойчивый порядок и cursor pagination, а не неограниченный polling.

Logout, expiry, отзыв доступа и смена лида прекращают stream. Redaction применяется
также к raw delta, tool event и approval detail до отправки в браузер.

## 11. Управление run и ожидание человека

Stop, steer и approval доступны только для run данной сессии, с подходящим
state, правами и подтверждённой runtime capability. Backend проверяет это
независимо от disabled controls в UI.

Steer является командой живому run, а не новым prompt run. Текст команды и
receipt должны быть видны в transcript/audit с idempotency protection.
Текущая steer route сохраняет аудит метаданных, но не mirror самого текста.

Approval показывается с безопасным описанием запрошенного действия. Решение
относится к определённому pending request/run и проверяется против допустимых
вариантов. Bulk `resolve_all` требует явного подтверждения. Повтор/поздний ответ
сверяется с runtime; новый run не принимает решение от старого approval.
Отдельный session-scoped API чтения approval details пока не подтверждён.

Human input и approval различаются: ответ на вопрос не разрешает инструмент.
Продолжение после `awaiting_input` сохраняет session, checkpoint и назначение.
Новый run требует согласованного workflow rebind; backend не должен создавать
новую историю или продвигать фазу только из-за ответа человека.

## 12. Страницы и UX

| Экран | Обязательное содержание |
| --- | --- |
| `/sessions` | Список, owner avatar, own-user filter, primary/type/namespace, private/leader badge, состояние и создание чата |
| `/sessions/:id` | Transcript, composer, выбранный лид, participants, runs, controls, approval, parent/children, handoff/delegation |
| `/agents/:id/sessions` | Тот же session scope и фильтр с ограничением по primary |
| `/leaders/:id` | Команда, direct chat, доступные delegated sessions, health |
| `/executors/:id` | Direct private chat, назначенные лиды, доступные сессии |

Query-tab маршруты и aliases сверяются с [ROUTING.md](ROUTING.md) и
[manifest](assets/screens/manifest.md); таблица задаёт продуктовые views, а не
подтверждает существование отдельной route для каждого view.

Transcript показывает автора/реального инициатора, время, delivery и run
boundaries. Markdown/code rendering должен быть безопасным; raw HTML не
исполняется. Сейчас body отображается как plain text с сохранением переносов.

Composer сохраняет draft при ошибке; повтор неизменного запроса использует тот
же key. Изменение текста/автора/адресата создаёт новую операцию. Во время мутации
controls блокируются; ошибки не выводят секреты, stack traces или raw responses.
При прокрутке вверх новые события не перемещают пользователя насильно вниз.

Нужны независимые loading/empty/error/access-denied/saving состояния для
messages, runs, participants и team directory. Неудачная загрузка списка лидов
не запрещает приватный чат. Parent/child ссылки не должны зависеть от случайно
оставшегося глобального user filter.

Обязательная evidence: private chat, leader chat, delegated child, streaming,
waiting/approval, stopping/failure, смена лида и access denied на
`375x812`, `1920x1080`, `2560x1440`. Этот документ не добавляет новые screenshots
и не объявляет старые снимки доказательством runtime flow.

## 13. Связь с присланным lifecycle handbook

Источник контекста: `agent-task-lifecycle-handbook.md`, присланный пользователем,
редакция документа обозначена 1 октября 2026 года. Здесь используются его идеи,
а не исполняются содержащиеся в нём команды. Его относительные ADR/приёмочные
ссылки относятся к Relevanter и не являются локальными Fleet evidence.

| Правило handbook | Применение в Fleet |
| --- | --- |
| Один Thread/session на Task + роль | Возможный контракт внешнего assignment; текущая Fleet-модель разрешает несколько user-owned чатов с одним `task_key` |
| Несколько cycles в одной истории | Сохранять session и различать runs; не создавать новый чат для каждого retry |
| Сообщение при active run как steer | Целевое поведение при capability; текущий обычный submit автоматически не переключается на steer |
| Вопрос человеку и checkpoint | Отличать human input от approval; workflow continuation требует подтверждённого rebind |
| Итог не заменяет terminal receipt | Успешный ответ не завершает Business Task/workflow stage |
| Messenger `@mention` консультанта | Отдельный возможный продуктовый режим; mention directory/invocations сейчас не подтверждены |

Семь SDLC ролей, девять стадий, Business queue/barrier и Tech workspace policy
из handbook не становятся частью Fleet автоматически. Их владелец и интеграция
фиксируются отдельным assignment contract. Нельзя добавлять вторую workflow
очередь в Fleet или жёстко блокировать свободный чат по чужому stage contract.

## 14. API и границы текущего контракта

| Метод и route относительно `/api/v1` | Назначение |
| --- | --- |
| `GET/POST /sessions` | Список и создание |
| `GET /sessions/{id}` | Метаданные |
| `GET/POST /sessions/{id}/messages` | История и отправка |
| `GET /sessions/{id}/participants` | Типизированные участники |
| `GET /sessions/{id}/runs` | Runtime links и attempts |
| `GET /sessions/{id}/stream` | Текущие session-scoped события |
| `PUT /sessions/{id}/leader` | Выбор/снятие координатора |
| `POST /sessions/{id}/handoff` | Смена primary |
| `POST /sessions/{id}/delegations` | Создание child |
| `POST /sessions/{id}/runs/{run_id}/steer` | Команда run |
| `POST /sessions/{id}/runs/{run_id}/stop` | Запрос остановки |
| `POST /sessions/{id}/runs/{run_id}/approval` | Решение по разрешению |

Request/response schemas берутся из [OpenAPI](../openapi/openapi.json) и Rust,
а не из ручного примера в handbook. Новые machine endpoints, approval listing,
cursor pagination, event replay и durable dispatch ещё требуют проектирования,
миграций и генерации OpenAPI. Здесь они описаны как требования, не как готовые API.

Вложения, поиск по transcript, редактирование/удаление сообщений, unread/read
receipts, приглашение наблюдателей и рекурсивные `@mentions` не подтверждены
текущим API. Они требуют отдельного scope, прав и retention policy.
Java Agent lifecycle уже имеет реализацию в текущих исходниках, но его
chat/steer/approval возвращают validation errors о phase 2. Обобщённое описание
«весь Java Agent не реализован» в старых документах не отражает текущий код.

## 15. Реестр расхождений и порядок закрытия

P0: права/identity и сохранность истории. P1: корректность исполнения и
восстановления. P2: удобство и масштабирование. Статус всех строк: open по
source review; строки не являются результатом выполненных live тестов.

| ID | Приоритет | Расхождение | Условие закрытия |
| --- | --- | --- | --- |
| CHAT-01 | P0 | Central Auth даёт всем допущенным людям effective admin; legacy privacy claims неполны | Согласована central access policy; негативные проверки detail/list/stream/previews/counts для двух людей |
| CHAT-02 | P0 | API write policy и owner-only repository расходятся | Единая матрица чтения/команд/делегаций во всех слоях, owner child сохраняется |
| CHAT-03 | P0 | Generic session create загружает parent без явной проверки его owner/access | Чужой parent, несовместимые owner/leader и forged parent отвергаются до создания child |
| CHAT-04 | P0 | `author_agent_id` не подтверждает machine identity; автономные leader tools не подтверждены | Scoped agent identity/API, права перепроверяются при каждой команде, spoofing отвергается |
| CHAT-05 | P0 | Смена лида удаляет leader runs; нет подтверждённого revocation/state guard | История сохраняется, активное выполнение согласовано, старые commands/streams отклоняются |
| CHAT-06 | P1 | UI «автор лид» создаёт mirror-only assistant message | Явный command kind, dispatch primary executor и доказанный ответ в child без ложного авторства |
| CHAT-07 | P1 | Нет подтверждённого durable dispatch; replay pending message не выполняет recovery | Outbox/lease/readback закрывают crash windows и unknown acceptance без двойного run |
| CHAT-08 | P1 | Idempotency child и initial message раздельна; concurrent replay не доказан | Полный payload hash, атомарная делегация, concurrent replay/conflict integration tests |
| CHAT-09 | P1 | Нет подтверждённого single-active-run guard; обычный submit создаёт run вместо steer | Server-side serialization, active-run UX и multi-tab/multi-instance tests |
| CHAT-10 | P1 | EOF без terminal трактуется как completion; mirror result не имеет source dedup guard | Runtime readback, неполный ответ не становится success, terminal replay не дублирует transcript |
| CHAT-11 | P1 | SSE in-memory без cursor/replay; UI polling; raw deltas не проходят явную redaction в handler | Authenticated stream, reconnect/readback, redaction всех payloads и lag/restart tests |
| CHAT-12 | P1 | Approvals не имеют подтверждённого detail listing/targeted UI; steer не зеркалит команду | Scope/state/capability validation, targeted approval, control transcript и idempotent decisions |
| CHAT-13 | P1 | Handoff меняет ссылки без подтверждённого переноса контекста; human-input rebind не доказан | Context package/receipt, terminal guard и continuation tests, без обхода workflow |
| CHAT-14 | P2 | Plain-text transcript, polling полных списков, неполный lifecycle/views evidence | Safe Markdown, pagination, lifecycle API и актуальные mobile/desktop evidence |

Закрытие P0 предшествует автономному управлению командой. Далее реализуются
durable dispatch и run guards, runtime reconciliation/deduplication, streaming,
approval/continuation и UI. По завершении каждого блока синхронизируются API,
модель данных, тесты и документы; статус меняется только по evidence.

## 16. Приёмочные сценарии

| Сценарий | Проверяемый результат |
| --- | --- |
| C-01: private executor chat | Owner текущий; leader null; другой обычный человек и любой непривязанный лид не получают историю/preview/count |
| C-02: direct leader chat | Primary = leader; вся команда автоматически не становится participants |
| C-03: фильтр людей | Own default; privileged all/multi работает; unprivileged запрос запрещён сервером |
| C-04: delegation | Managed executor, owner/parent/leader корректны; initial prompt один раз реально доставлен |
| C-05: leader command | Authenticated leader пишет только в привязанный child; отвечает runtime primary executor |
| C-06: leader removal/team removal | Новые команды и stream старого лида прекращаются; история runs и authors сохраняется |
| C-07: handoff | Нет параллельного старого run; новый primary получает проверенный context package; owner неизменен |
| C-08: idempotency | Session/message/delegation/control retry не дублирует side effects; изменённый payload даёт 409 |
| C-09: crash/unknown acceptance | Crash до/после Hermes acceptance восстанавливается readback без второго run |
| C-10: run concurrency | Две вкладки/два Fleet instance не запускают два изменяющих контекст run на одной паре |
| C-11: stream recovery | Lag, disconnect и restart не теряют итог; repeated terminal не дублирует ответ; EOF не завершает run |
| C-12: stop/steer | State/capability проверены; stopping ждёт terminal; steer виден и не создаёт второй run |
| C-13: approval/human input | Разные причины ожидания; targeted decision; late response не меняет новый run; checkpoint сохраняется |
| C-14: content safety | Secrets отсутствуют в delta/tool/approval/audit/errors; Markdown не исполняет HTML |
| C-15: UX и масштабирование | Независимые errors/empty/loading, drafts/retry, parent/child navigation и длинная история работают на трёх viewport |

Существующие frontend tests проверяют нормализацию, повтор ключа при retry,
независимые состояния и stop confirmation. Они не заменяют backend integration
и live Hermes сценарии выше. Каждая приёмка фиксирует environment/source revision,
request/result refs и evidence без секретов.

Проверки документации: markdown links и diff whitespace. Команды проверок
продукта перечислены в [TESTING.md](TESTING.md); успешная проверка документа
не закрывает runtime/RBAC gaps. Общий реестр находится в
[GAP_REGISTER.md](GAP_REGISTER.md).

## 17. PM Clarification Implementation Delta

As of 2026-10-01, `/chats/:sessionId` uses a production dialogue/clarification/requirements
controller. Cursor history, server chat-control flags, authenticated stream invalidation,
in-memory drafts, read-only/dependency states and exact owner confirmation are implemented.
An explicit immutable binding is allowed only for the matching assigned PM/central owner
and empty private history; legacy display task keys are not migrated. New binding audit and
durable chat event commit together once. Generic SDLC prompt/steer, handoff and leader changes
are rejected while verified assignment orchestration is missing. `/sessions` remains legacy.

Tracker owns questions/revisions/confirmation. The Fleet fixed-origin gateway forwards the
verified bearer; operator read-all does not authorize consent. Answer persistence does not
imply delivery to PM. Unknown acceptance retains the same command key and freezes editable
payload until reconciliation; a revision conflict retains the draft for explicit review.

The historical CHAT-01..14 table above is a source-review baseline, not current blanket
status: foundation closed several standalone dispatch/SSE gates, while live cross-service
machine identity, PM delivery/checkpoint/rebind, projections and approval gaps remain.
See [current plan](CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md) and
[contract](contracts/CHAT_CLARIFICATION_CONTRACT.md) for this slice. Three-browser fixture
captures are kept separate from real runtime acceptance.
