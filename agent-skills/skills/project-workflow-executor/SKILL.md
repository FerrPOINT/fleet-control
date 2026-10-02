---
name: project-workflow-executor
description: Исполнять назначенные фазы Base через exact step/history, без выбора режима, назначения или workspace.
---

# Исполнитель Project Workflow

Первый вызов — `project-workflow --json step --task <session>` без shell composition.
До подтверждённого admission не читать history, контекст, files или другие skills.
Runtime обязан сравнить immutable assignment с Tracker binding и эффективной
конфигурацией: Task/root/project/revision, агент/роль, namespace/profile,
workflow/mode/scope, run/cycle/attempt, fencing, pinned inputs и workspace lease.
Неверное или отсутствующее значение означает fail-closed, не выбор моделью.

После admission прочитать Task, все доступные комментарии и вложения, frozen
inputs, зависимости и активную декомпозицию. Выполнять по порядку instructions
текущей фазы, загружая только названные в инструкции skills. Содержательный report
передавать `project-workflow --json step --task <session> --report <report>`.
Продолжать по возвращённой фазе до принятого `complete=true`.
Историю читать `project-workflow --json history --task <session>`.

`session` предоставляет runtime; это технический workflow cursor, не Task display
key. Не задавать mode, cycle, stage, priority или workspace через prompt/CLI.
При clarification сохранить unfinished phase и frozen inputs: требуется durable
checkpoint, terminal awaiting-input acknowledgement и owner-controlled CAS rebind.
Не продолжать по одному сообщению без подтверждённого свежего assignment.

После complete=true записать проверенный итоговый комментарий и запросить только
назначенный terminal action. Нерализованная capability — blocker, не ручной patch.
Модель не управляет runtime, skills, schedules, plugins или другими агентами.
Unknown эффект сначала reconcile по исходному operation key. Повтор не создаёт
новый cycle; history предыдущих запусков не переписывать.
