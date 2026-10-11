# Контекст сквозного Namespace

Fleet не владеет бизнес-Namespace. Admin владеет registry, Tracker — Task,
Forge — repository. Legacy Workflow namespace сохраняет собственную семантику.

`GET/PUT /api/v2/sessions/{id}/execution-context` хранит immutable
ExecutionContextV2 отдельно от strict SDLC v1. Ссылки включают instance UUID;
проверяются Namespace, Task и repositories через fixed owner readers.
Session ACL сохраняется: private chat принадлежит своему пользователю;
shared/operator permissions проверяются повторно в transaction.

Migration 0090 добавляет `session_execution_contexts`. Persisted original
command и verified projection сверяются при чтении. Повтор сначала читает
исходную запись, без remote revalidation. Другой actor/payload получает conflict.
Привязка ждёт отсутствия активных runs и dispatch outbox. SQL guards сериализуют
binding с user prompt/run/outbox writes через session row lock.

Adapter `namespace-context-v2/foundation-v1-disabled` сохраняет
`runtime_ready=false` и `dispatch_allowed=false`. Новый prompt и execution start
закрыты; история и terminal drain доступны. Namespace acceptance не включает
автономный SDLC и не повышает runtime gates.

Readers задаются `FLEET_CONTROL_NAMESPACE__TRACKER_URL/TRACKER_TOKEN_FILE`
и `FORGE_URL/FORGE_TOKEN_FILE`; credentials не приходят из user PAT.
`FLEET_CONTROL_NAMESPACE__MACHINE_SUBJECTS` закрывает обычные human routes
до mapping. HTTP ограничен fixed origin, 10 s, 64 KiB, без redirects.

`VITE_NAMESPACE_ENABLED=true` включает контекст в core Chats и legacy session
detail. Tracker deep link сохраняет Namespace/Task в URL. Повреждённая ссылка
закрывает composer и показывает ошибку. Обновление одной вкладки не меняет
контекст другой. Foundation #59 интегрирован с исходной историей; его старые
screenshots/checks не заменяют новую Namespace acceptance.

PAT без display metadata использует существующий active профиль по точному
central subject. Lookup не создаёт профиль, не меняет роль и timestamp.
Machine subjects проверяются перед profile mapping, включая SSE revalidation.

`POST /api/v2/sessions` creates an inert draft and verified v2 context without a pending run or dispatch outbox. The operation key has a reserved domain; original-key retry preserves the chat ID after missing owner ACK. Migration 0091 keeps even an unconfirmed draft closed to starts. Legacy v1 creation still queues its historical run.
