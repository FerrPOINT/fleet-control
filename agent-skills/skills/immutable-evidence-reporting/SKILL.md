---
name: immutable-evidence-reporting
description: Связывать фактические проверки и результаты с exact assignment, revision, SHA и immutable receipts.
---

# Проверяемое evidence

Сохранять Task/root/project, requirement/decomposition revisions, role/mode/scope,
run/cycle/attempt, fencing и input refs. Для каждого check указывать scenario,
команду/surface, дату, expected/actual, status и безопасный evidence ref/hash.
Отделять source review, local checks, CI, runtime health и user acceptance.
Не выдумывать PASS, remote SHA, receipt, screenshot или ссылку.
Git-changing роль подтверждает разрешённый diff, commit/push, remote SHA и clean
workspace; read-only роль не пушит reviewed branch. Evidence append-only.
Final Markdown comment: результат, coverage, проверки, evidence, ограничения.
Никаких reasoning, raw tool dumps, stack traces, tokens и credential output.
Старый receipt, другой SHA/revision и один child не доказывают root acceptance.
