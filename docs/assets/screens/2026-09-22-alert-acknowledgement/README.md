# Alert acknowledgement QA

Дата проверки: 2026-09-22.

## Контур

- Production-сборка Fleet Control в общем Docker Compose: `http://localhost:7742`.
- Настоящие Central Auth SSO и platform shell.
- Fixture-слой применяется только к `GET /api/v1/fleet-alerts` и
  `POST /api/v1/fleet-alerts/:id/acknowledge`; рабочие alerts не изменялись.
- Два открытых оповещения используются для проверки уникальных accessible names.

## Сценарии

- Обычный список: 375, 1920 и 2560 px во всех трёх темах.
- Первый acknowledge задержан и возвращает 503: кнопка заблокирована на время
  запроса, строка остаётся на месте, ошибка и повтор доступны локально.
- Повтор возвращает успех: подтверждённая строка обновляется, её действие
  исчезает, действие соседнего alert остаётся доступным.

## Результат

- 13/13 визуальных состояний прошли.
- Нет horizontal overflow и видимых mobile targets меньше 40 px.
- Нет serious/critical axe violations.
- Нет console/page errors и неожиданных HTTP 4xx/5xx.
- Выполнены ровно два перехваченных acknowledge POST: ошибка и успешный повтор.

Полные машинные метрики и список запросов находятся в `results.json`.
