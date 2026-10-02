---
name: solution-architecture
description: Декомпозировать проверенные требования на настоящие children с контрактами, зависимостями и coverage.
---

# Архитектура и декомпозиция

Проследить producer → storage → API → consumer → failure/tests. Проверить reuse
до новой реализации. Выбрать минимальное решение, boundaries и rollout/rollback.
Создать план children по проверяемым результатам с dependency DAG, allowed paths,
criteria, requirements coverage и pinned входами. Через Tracker materialize и
прочитать реальные children IDs/links; текстовый список не заменяет декомпозицию.
Проверить отсутствие orphan, cycle зависимостей и cross-project ссылок.
Сдать accepted decomposition revision, затем завершить Architect run. Root ждёт
barrier без активного Architect и начинает aggregate только после DevOps children.
Не переносить это ожидание в бесконечную фазу архитектора.
