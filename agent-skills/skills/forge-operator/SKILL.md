---
name: forge-operator
description: Работать с назначенными Git, pipelines, workspace и receipts Forge без подмены candidate или обхода approvals.
---

# Оператор Forge

Работать только с проектом, веткой, pinned SHA, workspace и allowed paths из
assignment. Forge остаётся владельцем Git и CI/CD, а не только источников.
Не передавать себе общий PAT, SSH key, Docker socket; не создавать произвольный
checkout или deployment target. Read-only роли не пушат проверяемую ветку.

Проверенный существующий read CLI: `cicd-cli pipeline show --id <PIPELINE_UUID>`.
Git readback: `git ls-remote origin <assigned-ref>` в выданном workspace.
Не подставлять main вместо assigned-ref. Другие команды сверять с текущим help
и правами; CLI `deployment create --status success` сам по себе не доказывает deploy.

Требуемые будущие capabilities: prepareAttemptWorkspace, checkpointWorkspace,
publishCandidate, lookupOperation, prepareVerificationTarget, promoteCandidate,
closeVerificationTarget и acceptanceReceipt. При их отсутствии фиксировать blocker,
не выдумывать CLI и не выпускать продукт вручную вместо Hermes.

Developer отдаёт permitted diff, commit/push и remote SHA при изменениях; пустой
commit не создавать. Build policy и проверенный candidate receipt предоставляет
Forge; сборку и promotion организует DevOps, без нового Developer mode build.
Reviewer/Tester проверяют назначенный exact candidate и gates, а не выбирают другой.
DevOps получает immutable source/artifact/image/config digests, approvals,
rollback baseline, затем проверяет served identity, health и acceptance.
Unknown dispatch/push/deploy сначала lookup/reconcile исходной операции.
