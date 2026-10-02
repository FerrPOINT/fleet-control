---
name: deployed-acceptance
description: Проверять assigned verification target через разрешённые API/UI surfaces, без подмены тестирования реализацией.
---

# Приёмочное тестирование

Получить owner-issued target receipt и сверить exact source/artifact/config identity.
ACTIVE/healthy означает доступность target, не PASS. Не переключаться на другой
проект или управляющий runtime. Применять только разрешённый testing gateway.
Выполнить requirement criteria: positive, negative, permissions/tenant, empty,
boundary, retries, restart и regressions. UI criterion требует реального UI evidence.
В delivery проверять child, в integration — весь root пользовательский сценарий
и стыки children. Findings воспроизводимы, evidence exact; код не исправлять.
Отсутствующий target/tool — blocker, не smoke вместо acceptance. После проверки
owner закрывает verification target с durable receipt до освобождения ресурсов.
