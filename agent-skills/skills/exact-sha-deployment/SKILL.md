---
name: exact-sha-deployment
description: Выпускать назначенный immutable candidate через Forge с approvals, rollback и exact runtime acceptance.
---

# Выпуск exact candidate

Проверить assigned project/target, source SHA, pipeline gates и candidate manifest.
Организовать build через существующий Forge-контур по назначенной milestone policy;
не пересобирать случайный SHA или mutable tag после каждого шага. Сохранить
rollback identity и data compatibility. Уважать required human approvals.
Promotion только owner mechanism с operation key. Unknown — reconcile, не retry.
После deploy проверить served SHA/image/artifact/config digests, health, сохранность
данных и acceptance criteria. Delivery receipt касается child; integration/root
receipt обязан подтвердить общий Deployment и acceptance. Rollup не завершает root.
Failure допускает только разрешённый rollback; не редактировать feature code.
