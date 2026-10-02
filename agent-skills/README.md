# Candidate skills автономного SDLC Base

Статус: configuration-only candidate, не установлен. Владельцем пакета является
Fleet Control. Это не доказательство готовности исполнения SDLC.

`manifest.json` фиксирует семь namespace/profile, физические allowlists,
основные инструкции и SHA-256 нормализованного LF UTF-8 содержимого каждого файла.
`sources.native.revision=SELF` означает точный commit, содержащий manifest;
Workflow candidate закрепляет этот commit извне, без самоссылки SHA.

## Применение в будущей реализации

1. Проверить commit, manifest, hashes и отсутствие дополнительных файлов.
2. Подготовить validated config revision средствами Fleet, не из модели.
3. Для каждой роли материализовать только её `physicalSkills` и `roleInstruction`
   в отдельном namespace/profile, HOME и физическом каталоге skills.
4. Активировать после drain, readback и подтверждённого rollback path.
5. Проверить assignment admission и разрешённые capabilities; до этого не запускать.

Раздельные HOME и каталоги не являются полноценной ОС-песочницей.
Не передавать общие admin tokens, Docker socket, SSH keys и credential stores.
Manifest проверяет установку и допустимые skills, но не выбирает routing, mode,
priority, workspace или исполнителя. Это решения Tracker и владельца ресурса.

## Проверка без runtime

```bash
python agent-skills/verify_package.py
python -m unittest discover -s agent-skills/tests -v
```

Candidate Workflow использует отдельный файл `base_sdlc_catalog_v1.json` в
Project Workflow. Техническая schema сохраняет legacy identifiers; слово
Business в compatibility marker валидатора не возвращает прежнего владельца.
Публичный Workflow CLI остаётся `step/history`. Другие операции в skills —
требуемые capabilities, а не утверждения о существующих CLI-командах.

Контракт исполнения: [SDLC v1](../docs/contracts/SDLC_EXECUTION_V1.md).
