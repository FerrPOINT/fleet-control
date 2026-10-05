# SDLC: реализация и оставшаяся приёмка

Дата: 4 октября 2026. Статус: частичная реализация foundation; автоматический
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
| Чаты | `/chats`: конкретный агент → его сессии, поиск, аватар владельца, private create; immutable task/agent binding | Свободные legacy sessions не превращаются в SDLC автоматически; task chat creation ожидает admission |
| Доступ | Central subject сохраняет локальную роль, не получает effective-admin | Bootstrap admin только по явно настроенному проверенному subject |
| Messages | Авторизация owner/operator; человеческий API не позволяет выдавать себя за агента | PostgreSQL regression test; отдельная machine identity ещё не реализована |
| Idempotency | Session key сериализуется transactional advisory lock; message key — session row lock | Concurrent replay и payload conflict проверены на PostgreSQL |
| Dispatch | Prompt и outbox создаются одной транзакцией через trigger | Claim блокирует agent row; capacity 1; pending не отправляется при drain |
| Unknown acceptance | Нет автоматического redispatch после неопределённой отправки | `uncertain` удерживает слот; recovery operator API ещё отсутствует |
| Dispatch journal | Миграция 000012: atomic concrete run + immutable request bytes/hash/key/origin/default-profile fingerprint/protocol facts, одноразовый permit до POST; ACK/run/message/outbox/journal атомарны | Known-ID GET требует original context. Opt-in recovery фиксирует native store epoch до POST; legacy без journal не probe-ится. Operator API ещё отсутствует |
| Prepared recovery | Bounded keyset worker продолжает только исходные unconsumed prepared free-chat intents после fresh health/protocol proof; transactional permit проверяет identity/drain/capacity/deadline | 331 distinct Linux/PG tests PASS; concurrent supervisors отправляют один original POST. Submitted/expired/legacy/task/PM исключены, unknown не повторяется. Managed native/Fleet acceptance остаётся отдельной |
| Original-key recovery | Base native plugin сохраняет immutable witness в reservation transaction; Fleet использует non-dispatch lookup только с исходными scope/bytes/epoch/horizon | Component evidence дополнена managed lost-ACK case: два разных Fleet OS-процесса, настоящий gateway/plugin, один POST/inference/assistant без SSE для already-terminal run. Installed/running/native-crash recovery остаются. Negative/expiry/reset не разрешают redispatch. Default false; no historical backfill |
| Runs | Новые попытки сохраняют отдельные runtime run records; free-chat terminal packet атомарно сохраняет run, prompt delivery, optional assistant и durable events | Exact replay не меняет timestamps/cursor; task/PM не получают admission. Empty terminal не фабрикует ответ |
| Pinned recovery | Known accepted pending/running/waiting/stopping обходятся bounded keyset; pinned native terminal проверяется GET-only с original journal context | Нет второго POST/SSE worker; unknown/foreign/expired status удерживает capacity. Missed tool/approval history не реконструируется |
| EOF | EOF без terminal event требует status readback | Non-terminal/error оставляет waiting, а не ложный completed |
| Native controls | Steer/stop проверяют original accepted journal, свежие capabilities и pinned native GET; bounded exact ACK, без retry; run-wide approval закрыт и в adapter | Семь PG/HTTP и три unit проверки входят в358-case Linux gate; отдельный настоящий AIAgent steer/stop PASS. ACK не освобождает capacity и не доказывает SDLC/OS safe stop; durable command recovery, approvals и task admission остаются |
| Native stream bounds | Incremental UTF-8/CRLF framing, mandatory original run ID, JSON/header/byte/text/snapshot/deadline budgets; EOF не dispatch-ит partial frame | [Consumer profile](contracts/HERMES_EVENT_STREAM_V1.md); limits удерживают capacity, не останавливают Hermes и не восстанавливают missed tools/approvals. Проверки и scope — verification ledger |
| Stream | PostgreSQL events и per-session cursor; `Last-Event-ID`; проверка доступа на каждом poll | Durable replay; retention/expired-cursor snapshot ещё не реализованы |
| Redaction | Известные env secrets и derived token; stream snapshots удерживают split-secret suffix | Unit regression; произвольный неизвестный секрет не гарантированно обнаруживается |
| Legacy config | Secret-поля старого DB-конфига маскируются при чтении, валидные `secret_ref` сохраняются | Regression проверяет nested/array credentials и обычный task key |
| Config | Immutable snapshot config+skills, draft/validated/activating/active/failed | Desired/effective разные; activation drain, readback, tracked-runtime restart, rollback |
| Native renderer | Новые Hermes snapshots используют v2; legacy absent/1 сохраняет bytes/hashes. Listener/env/CORS закреплены, API aliases и malformed extra запрещены | Настоящий pinned loader прочитал два Rust-exported HOME без модели/listener; installed effective-config attestation остаётся open |
| Lifecycle | Per-agent supervisor lock для start/stop/restart/health и всей apply/rollback фазы; bounded parent kill/wait | Fresh state/drain checks, failed metadata commit удерживает journal/drain; cross-instance и descendant quiescence не реализованы |
| Managed native chat | Opt-in `native_supervisor_live`: настоящие gateway через Base launcher, Fleet config activation/outbox/mirror, separate HOME/cwd/SOUL/ports/token, original run status после restart | Lifecycle, lost-ACK/two Fleet processes, real AIAgent steer/stop и exact-action approval case с реальным terminal guard/effect и actual local JWT HTTP. Source archive13770 hash proof; модель local fixture. Не installed runtime, running/waiting-approval crash recovery, native unknown-control/decision lookup, полный config inventory, OS/descendant stop или task/PM admission |
| Exact approval context | Общий accepted journal/run/session/origin/credential guard для approvals и controls, fresh native capability/waiting-request readback и bounded exact200 JSON ACK | Legacy/task без admission запрещены; preflight hold uncertain не повторяется. Component/native evidence отдельно; loaded generation, fenced authority и unknown outcome recovery остаются |
| Delivery lock order | Session NO KEY UPDATE перед message; deterministic PG blocker/NOWAIT regression сначала FAILED, после исправления PASS | Unknown dispatch остаётся pending/held, event cursor атомарный; не общий DB/VM-clock fix |
| Readiness IO | Java headers/body 3 s / 16 KiB; Java/Hermes startup 60 s wall-clock | TCP/PG negative fixtures, не native configuration attestation или успешный SDLC |
| Failed rollback | Агент остаётся drained, effective revision не меняется; до file/runtime effects сохраняется защищённый дисковый activation journal | PostgreSQL regression + journal unit checks; автоматическое recovery после crash и operator API требуют дальнейшей реализации |
| Managed file durability | Linux fsync parent после rename/unlink; new skill directories синхронизируются leaf-to-root. Ошибка barrier удерживает journal/drain до подтверждённого восстановления | Component fault injection после visible rename, no effective head/spawn/second claim. Не power-loss simulation, Windows directory guarantee или native loaded-config attestation |
| Filesystem | Не присваивать непустую чужую папку, проверять marker и symlink/junction components | Guard действует на managed writes; защита от внешнего TOCTOU требует OS isolation |
| Provisioning | Существующий effective `.env` сохраняется; активный runtime не переподготавливается | Exclusive create и Unix `0600`; три regression tests |
| Readiness | Runtime health отдельно от readiness SDLC | `workflow_assignment_protocol_not_verified` блокирует весь автоматический SDLC |
| Shared UI | Совместимое расширение bearer SSE utility в Base | Cursor сохраняется при reconnect, credentials только в header |
| PM credentials | Opt-in выдача в creation continuation: immutable command/parent/origins journal, child ACK, fresh Base introspection и Tracker context, once-only audit | Миграция 000011 additive; default disabled; выдача не является admission, runtime handoff или live Base/Tracker acceptance |

## Контракт конфигурации

### B-SDLC-02: Подготовка Закреплённого Base Draft

Source slice от 2026-10-03: защищённый `POST /agents/{id}/config/base-package`
читает Git blobs точного Base pin, проверяет regular-file inventory 7/14,
schema/hashes, namespace/profile/modes и создаёт desired draft в существующем
config lifecycle. `dev_ops` Fleet API остаётся совместимым; package role — `devops`.
Путь задаёт оператор через env, не запрос клиента. HEAD и изменённые worktree
файлы не используются, network fetch/fallback отсутствуют.

В snapshot сохраняются pinned SOUL и allowlist skills; чужие enabled skills
отключаются только в draft. Проверка/активация сверяют snapshot заново, включая
содержимое и concrete agent ID. CAS защищает desired revision; при активации
role/namespace проверяются под row lock. Ошибки Git не раскрывают private content.

Проверено scoped Rust 1.88: 7 unit tests, включая настоящий private Git pin всех
семи ролей, и 4 PostgreSQL 17.6/HTTP tests. Проверены tamper/schema/hash/mode,
extra/missing/invalid UTF-8, proof/content drift, чужие skill IDs, drain и смена
роли перед активацией, concurrent desired/identity fencing. Отдельный legacy
config drain/rollback regression PASS; scoped infra clippy, fmt, frontend
typecheck/OpenAPI drift и ссылки документации PASS. OpenAPI regenerates from Rust.
Это не полный gate и не
установка. Physical/bundled inventory, effective Hermes config precedence,
assignment admission/prompt assembly и dispatch остаются отдельными gaps;
readiness blockers не снимаются. Полный автономный PDLC не принят.

Дополнительный source cut: effective readback сверяет snapshot с Git pin и
ограниченно обходит HOME skills, запрещая unlisted/nested/case-aliased `SKILL.md`,
symlink/junction и special files. Проверка read-only: чужие файлы не удаляются.
Legacy snapshot сохраняет прежнее поведение. Реальные Git blobs и matching-disk
подделка proof проверяются regression; native discovery вне HOME не сертифицирован.

Машинное чтение `/internal/runtime/v1/agents/{agent_id}/configuration` использует
fresh bounded Base PAT introspection, отдельно зарегистрированный subject и
точные read-only scopes конкретного агента. После файловой проверки повторно
читается effective head; ответ содержит только metadata и `runtime_ready=false`.
Observation не является config lease, assignment ACK или runtime receipt.
Проверки и env contract: [API](API.md#sdlc-configuration-observation),
[настройки](ENV.md#sdlc-configuration-observation), [тесты](TESTING.md#sdlc-foundation-checks).

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

Отдельный opt-in native gate теперь проверяет настоящие API/AIAgent/SQLite
на этом clean pin с локальной детерминированной моделью: dropped 202,
concurrent replay, process restart и interrupted inference без повторного run,
exact session/transcript, SSE/status и credential rotation. Это не capabilities
установленного агента и не полный gateway lifecycle. Evidence и границы:
[verification ledger](CHAT_CLARIFICATION_VERIFICATION.md#native-hermes-protocol-acceptance-4-october-2026).
Fleet теперь сохраняет immutable exact request/scope/horizon journal и optional
closed recovery capability до первоначального POST. Base opt-in native producer
и Fleet consumer проверены компонентно: positive original-key lookup без нового
run, prune/tombstone/reset и DB-lock expiry. Это не managed gateway acceptance;
installed rollout остаётся закрытым. Negative/uncertain proof не разрешает repeat.
Контракт и evidence: [Recovery v1](contracts/HERMES_RECOVERY_V1.md).
Raw `serve` у этого pin запускает dashboard; нынешний
Fleet argv требует Base compatibility wrapper, см. [runtime](RUNTIME.md).
Terminal `interrupted` трактуется как failed, без fabricated assistant reply;
это проверено отдельным fake Hermes HTTP regression.

## Оставшиеся блокеры по этапам

Fresh producer release audit (5 October): Tracker PR114 head8c80a41 differs
from the accepted Fleet snapshot in three of seven generated chat schemas and
does not include local Analysis/routing extensions. Workflow PR90 heade4fba60
does not include local Base admission/binding; its PM bind remains post-dispatch.
Neither local producer worktree nor healthy runtime may stand in for those
release contracts. See [exact compatibility boundary](contracts/CHAT_CLARIFICATION_CONTRACT.md#producer-release-compatibility-5-october-2026).

| Этап | Обязательная работа | Владелец |
| --- | --- | --- |
| 1 | Исполняемые versioned assignment/result/receipt контракты и capability matrix | Все четыре сервиса |
| 2 | Machine scopes, project access, проверенный central subject между сервисами | Tracker/Fleet/Workflow/CI |
| 2 | Tracker requirement revisions, confirmation exact revision, structured clarification | Tracker |
| 2 | Immutable task/root IDs, real parent/root decomposition, cycle validation | Tracker |
| 2 | Project role-agent snapshot, workflow/config revisions, deployment policy snapshot | Tracker |
| 2 | Assignment leases, heartbeat/fencing, outbox/inbox и monotonic side-effect checks | Tracker + consumers |
| 3 | Unique task-ID/agent-ID chat binding, замена агента без handoff старой истории | Fleet/Tracker |
| 3 | Managed native/Fleet приёмка prepared/submitted/accepted recovery после crash; operator reconciliation и missed tool/approval history | Fleet/Hermes |
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
Это составные display identifiers `role_mode`, не дополнительные mode keys.
Ключи Developer строго `initial/rework`, scope приходит независимо.
Канонический candidate-пакет находится в приватном
[Base agent-skills](https://github.com/FerrPOINT/services-base/tree/docs/base-sdlc-transfer-20261002/agent-skills).
Fleet потребляет exact Base commit; локальной редактируемой копии и fallback нет.
Нужен авторизованный доступ к pinned commit, credentials в product docs не входят.
Пакет не установлен и не включает автоматический SDLC. Versioned target contract:
[SDLC execution v1](contracts/SDLC_EXECUTION_V1.md).

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

Current approval recovery: authenticated pinned free-chat GET restores only the
native status document's current exact request. Two Fleet OS processes against
real Hermes verify no duplicate run/SSE/decision or assistant mirror. Historical
approval/tool replay, unknown decision outcomes, fenced task admission, PM
tools/continuation and descendant-safe stop remain independent prerequisites.
