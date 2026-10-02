---
name: repo-workflow
description: Вносить только разрешённые Git изменения задачи, проверять diff, commit/push и exact remote SHA.
---

# Работа в Git

В owner-issued workspace проверить инструкции, remote, branch, pinned base/head,
status и permitted paths. Непонятную dirt не присваивать и не сбрасывать; slot
карантинирует владелец. Не создавать произвольный checkout и не применять git clean
в общей директории. Изменения только в allowed paths, без secrets/cache/build output.
Запускать scoped checks; общий gate один раз на назначенной интегрированной вехе.
Перед передачей просмотреть весь diff, staged paths, затем commit/push только
свои разрешённые изменения, перечитать exact remote SHA и чистоту workspace.
При отсутствии diff не создавать пустой commit. Unknown push reconcile по исходному
key/ref; force push и самовольный merge/deploy запрещены.
