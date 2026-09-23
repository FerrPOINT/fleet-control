# Session detail mutation QA

Дата проверки: 2026-09-22.

## Контур

- Production-сборка Fleet Control в общем Docker Compose: `http://localhost:7742`.
- Настоящие Central Auth SSO и platform shell.
- Session, directory и mutation endpoints перехвачены fixture-слоем; рабочие
  сессии, сообщения и runtime runs не изменялись.
- Наполненная сессия содержит два запуска, сообщение и участников.

## Сценарии

- Полная страница: 375, 1920 и 2560 px во всех трёх темах.
- Сообщение: 503, сохранённый черновик, повтор с тем же idempotency key, успех.
- Делегирование: 503, сохранённые поля, повтор с тем же idempotency key, успех.
- Steer: нормализация payload, pending только внутри выбранного run; соседний
  run остаётся доступным.
- Stop: уникальные accessible names, Escape без POST, подтверждение, pending
  lock, 503 внутри открытого диалога, повтор и обновление только нужного run.

## Результат

- 15/15 визуальных состояний прошли.
- Нет horizontal overflow и видимых mobile targets меньше 40 px.
- Нет serious/critical axe violations во всех темах.
- Нет console/page errors и неожиданных HTTP 4xx/5xx.
- Выполнены только перехваченные записи: message 2, delegation 2, steer 1,
  stop 2. Payload и стабильность idempotency keys проверены.

Полные машинные метрики и список запросов находятся в `results.json`.
