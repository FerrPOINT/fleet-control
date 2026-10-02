---
name: workflow-writing-plans
description: Формировать исполняемый план с владельцами, входами, зависимостями, проверками и rollback без скрытых решений.
---

# План исполнения

Записать goal/non-goals, owner, exact inputs/revisions, reused capabilities и
недостающие интерфейсы. Для каждого шага назвать результат, evidence, проверки
и stop condition. Разделить source/configuration/target/runtime evidence.
Предусмотреть negatives, tenant, versioning, idempotency и partial failure.
Зафиксировать порядок интеграции и rollback без удаления чужих данных.
Не заменять назначенные фазы собственным workflow и не менять routing.
