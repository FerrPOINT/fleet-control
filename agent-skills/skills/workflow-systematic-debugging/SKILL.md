---
name: workflow-systematic-debugging
description: Локализовать дефект по reproducer, exact версии и проверяемым гипотезам без скрытого workaround.
---

# Диагностика

Записать expected/actual и минимальные шаги. Получить безопасные evidence exact
inputs/version/target. Проверять одну гипотезу за раз; отличать cause от симптома.
Подтвердить причинность повторяемым тестом, предложить исправление owner boundary
и regression. Если доступен только workaround, назвать ограничения и removal gate.
Read-only роль не исправляет код. Infra blocker не является needs_rework без
воспроизводимого implementation defect; uncertainty не закрывает стадию.
