---
name: tracker-operator
description: Читать назначенный проект и использовать только role-scoped guarded Task capabilities Tracker с CAS и operation key.
---

# Оператор Task Tracker

Backend вычисляет project context и права из Task. Context token даёт только
разрешённое чтение, не право менять Task или завершать стадию. Не передавать
произвольный project ID. Читать Task, requirement revision, комментарии,
вложения, links, children и evidence с проверкой доступа, не создавать shadow store.

Требуемые capabilities, пока не гарантированные текущим CLI: updateDraft,
publishDraft, acceptDecomposition, appendStageComment, requestClarification,
completeAssignedStage. Это имена контракта, не команды shell. При отсутствии
одной из них остановиться с точным blocker; не обходить generic Task patch.

PM меняет только разрешённые поля текущего Draft: summary/description, priority,
estimates/dates, component/milestone/labels, assignee, parent/dependencies/links,
подзадачи и ссылки существующих вложений того же проекта. Project/kind/stage/status
не входят в универсальный patch. Название 3–4 смысловых слова, максимум 80 символов.
Публикация требует подтверждения exact revision и guarded CAS. Подтверждённый
Backlog автоматически ставит Analysis в очередь backend, а не модель.

Architect материализует настоящие children и зависимость `dependent → dependency`
через владельца, с coverage requirements и активной revision. Никаких fake IDs.
Reviewer/Tester передают passed либо frozen needs_rework findings с reproduction,
expected/actual и evidence. Tracker выбирает следующий stage/mode; агент этого не делает.
Повтор операции требует тот же key/payload; stale revision/fencing отклоняются.
Новые требования после публикации — новая редакция и invalidation evidence,
не тихая правка pinned history. Approval и clarification не взаимозаменяемы.
