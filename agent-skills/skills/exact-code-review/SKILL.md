---
name: exact-code-review
description: Независимо проверять exact delivery/integration candidate и выдавать passed или воспроизводимые findings.
---

# Независимое review

Перечитать назначенные base/head/candidate refs. Проследить dataflow, ownership,
права, persistence, migrations, compatibility, tests, idempotency, restart и hygiene.
В delivery проверять назначенный child и его contracts; в integration — общий
root candidate, стыки всех обязательных children активной декомпозиции и coverage.
Не менять реализацию, reviewed branch, назначения или режим. Passed требует
актуальных checks того же SHA. needs_rework содержит severity, exact location,
reproduction, expected/actual и evidence; иначе задать один конкретный вопрос.
Повторное review после rework проверяет findings и общий regression, не только diff.
