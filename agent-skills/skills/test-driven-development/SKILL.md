---
name: test-driven-development
description: Реализовывать initial/rework через воспроизводимые RED-GREEN-REFACTOR проверки frozen требований или findings.
---

# Разработка через проверки

Связать тест с requirement или frozen finding. По возможности получить реальный
RED на production behavior, затем минимальный GREEN и refactor. Не ослаблять
assertions и не выдавать тест константы/mock за реализацию. Проверить позитивный
и отрицательный сценарии, применимые permissions, retry/stale/partial failure.
В rework исправить каждый finding текущего cycle и выполнить regression.
Во время срезов только быстрые package checks; полный build/CI не запускать после
каждого исправления. Общий gate следует политике Forge назначенной вехи.
Сохранить результаты exact SHA; PENDING и skipped не являются PASS.
