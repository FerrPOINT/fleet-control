# Требования к конфигурации семи Hermes для SDLC Base

Статус: **configuration requirements**, 2026-10-03; partial source implementation,
не installed config и не complete native admission. Дополнение к
[execution contract](contracts/SDLC_EXECUTION_V1.md).
[Candidate example](examples/sdlc-runtime-requirements.v1.json) — структурированный
requirements packet, **не** DTO существующего /agents API, config.yaml,
Compose override или install command. Его нельзя передавать activation endpoint.

## Владение и source-only граница

Base владеет приватными role-инструкциями, 14 skills и manifest.
Workflow — фазами/modes; Tracker — назначением/очередью; Forge — workspace.
Fleet владеет concrete agent identity, effective config и материализацией.
В этом пакете нет копий приватного содержимого, credentials или active settings.

Реализованы source-only pinned package preparation в desired draft, Git/snapshot
verification, closed managed HOME readback и отдельное machine observation API.
Последнее всегда возвращает `runtime_ready=false`. Существующий config lifecycle
используется без второго installer; native-loaded model/tools/limits, assignment
proof и автоматический dispatch остаются target. Проверки файлов не заменяют эти
доказательства. Подробности: [API](API.md#sdlc-configuration-observation).

Выявлен unresolved owner mapping: manifest `namespace` — символическое имя,
а Fleet `namespace_id` — opaque Workflow identity; `namespace_name` хранится
отдельно в binding. Текущий package guard сравнивает ID с символом и не допускает
настоящий numeric-ID binding. Это blocker, не допустимый alias или успешная
namespace verification. Требуется fresh owner ID/name/workflow/profile readback
и frozen mapping; ослаблять сравнение, подменять catalog либо fixtures нельзя.

Потребитель требует доступ к exact Base commit. SELF в manifest — содержащий
commit; закреплённый skills commit не равен SDK .base-revision.
Неизвестный/недоступный pin блокирует подготовку: fallback на Fleet или локальную
папку запрещён. После squash merge требуется доступный accepted commit и hash
reconcile до установки, а не сохранение недоступной PR-only revision.

## Семь исполнителей и привязка

| Role | Namespace | Profile | Допустимые modes |
| --- | --- | --- | --- |
| project_manager | hermes-project-manager | hermes-sdlc-project-manager | draft |
| analyst | hermes-analyst | hermes-sdlc-analyst | analysis |
| architect | hermes-architect | hermes-sdlc-architect | decomposition |
| developer | hermes-developer | hermes-sdlc-developer | initial, rework |
| reviewer | hermes-reviewer | hermes-sdlc-reviewer | delivery, integration |
| tester | hermes-tester | hermes-sdlc-quality | delivery, integration |
| devops | hermes-devops | hermes-sdlc-operations | delivery, integration |

Это ссылки на существующий Base manifest, не новый registry routing.
Каждому role binding назначается конкретный Fleet agent ID и отдельный
HERMES_HOME/skills directory в существующем agents/agentN layout.
Нельзя использовать один Hermes со сменой role prompt или общий HOME.
Developer delivery/aggregate — scope, не дополнительные modes.
Конкретные ID и paths материализует Fleet; модель их не выбирает.

## Effective configuration и tools

Configuration revision должна связывать Base package commit/manifest hash,
role instruction hash, physical allowlist, Workflow catalog commit/hash,
model/provider selection, context/output limits, turn budget, terminal timeouts,
effective toolsets и capability permissions. Credentials передаются только
owner-approved secret references, не в этом example или prompt.

Оператор задаёт модель и лимиты через существующий Fleet config boundary.
Значения не выводятся из legacy DEV overrides. Runtime precedence проверяется
на реальном Hermes adapter; конфликт YAML/environment/CLI должен быть
разрешён и отражён в effective readback до activation. Неизвестный effective
output limit либо unsupported capability блокирует admission.

Example содержит null для concrete IDs и model/limits; catalog source уже закреплён:
это недостающие installation inputs, не допустимые runtime defaults.
Explicit null должен блокировать установку до заполнения owner-конфигурацией.

Terminal/skills нужны для принятого workflow CLI и чтения разрешённых skills.
code_execution — наблюдавшаяся legacy capability, **не автоматически выданное
право** Developer. Она подключается только если approved Fleet policy, sandbox
и Hermes capability tests подтверждают необходимость и безопасное исполнение.
Toolset name не заменяет authorization/allowed paths или OS isolation.

Не передавать общий admin token/PAT, credential store, SSH key или Docker socket.
Context identity — least privilege; context token read-only, не terminal token.
HOME-разделение не считается OS sandbox.

## Подготовка, активация и rollback — полная цель

Переиспользовать существующий Fleet configuration lifecycle, не второй installer:

1. Read pinned package, schema/hashes/allowlists; проверить доступность counterpart capabilities.
2. Создать candidate config snapshot и ограниченный before/after diff; не менять active files.
3. Validate role/namespace/profile/modes, skills, paths, provider и effective limits.
4. Drain active run с durable checkpoint/terminal ACK; unknown state reconcile.
5. Materialize только разрешённые файлы, сохранить ownership/mode, выполнить
   atomic activation средствами owner; failure не публикует частичный snapshot.
6. Read back installed hashes, effective model/tools/limits и readiness.
7. При провале вернуть previous exact config; восстановить binding owner protocol.
   Историю Tasks/chats/audit и Git evidence не переписывать.

Drain/activate endpoints уже существуют, но SDLC materialization/admission и
указанная full verification остаются B-SDLC-02. Не заявлять готовность по наличию
endpoint, успешной записи файла или health процесса.

Серверные donor helpers использовали dry-run, backups, hashes и atomic replace,
но не доказывали полный lifecycle. Их regex replacement и старые runtime
scripts не импортируются. Документируется полезная логика, а не новая зависимость.

## Prompt assembly и admission

Pinned role instruction → trusted backend assignment → current phase →
instruction-level allowed skills. Assignment/configuration проходят проверку
до модели; effective permissions не берутся из текста.
Runtime не принимает model-selected role/mode/scope/workspace или произвольный project.
Missing capability — точный blocker, не старый bridge или операторская работа за агента.

## Implementation acceptance

- Семь separate IDs/HOME; extra skill и wrong role/mode/profile отклоняются.
- Недоступный package, mismatch hash/catalog и unresolved example null fail-closed.
- YAML/env/CLI конфликты проверены against actual adapter; readback показывает effective values.
- Toolsets не расширяют allowed actions/paths; unsupported code execution блокируется.
- Dry-run не меняет active config; failed validation/drain/activation сохраняет прежний snapshot.
- Restart/unknown activation, loss of ACK и repeated operation reconcile исходный snapshot.
- Rotation во время run не подменяет pinned config; secrets не попадают в логи/чат.
- Readiness процесса, installation readback и autonomous acceptance — разные gates.

Source-only проверки example: семь ролей, точный Base pin, namespace/profile и
instruction refs, non-installable flag, отсутствие concrete endpoints/secrets.
Они выполняются verifier Base; реальный runtime acceptance здесь **NOT RUN**.
