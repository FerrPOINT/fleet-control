# Реальная Base → Tracker PM credential interoperability

Opt-in TCP integration test использует production Fleet `PmCredentialIssuer`,
реальный Base auth-server и Tracker server с настоящими PostgreSQL migrations.
Base/Tracker source не меняются; shared/accepted runtime не используется.
Это issuer/current-context interoperability, не Fleet creation/coordinator saga,
native admission, execution lease, Workflow acceptance или model dispatch.

## Запуск

`--cargo-cache-dir` принимает существующий локальный Cargo cache вместо named
volume. Без `--rustup-cache` используется toolchain выбранного image; runner
проверяет `rustc 1.88.0` перед сборкой. Caches установленных приложений не нужны.
Cargo data монтируются в `/cargo`; binaries toolchain остаются в image. Каталог
cache без `bin/` не перекрывает `rustc` и `cargo` из выбранного Rust image.

Python 3.11+, Git, Docker Compose v2 и локальные images
`rust:1.88.0-bookworm`, `postgres:17.6-alpine` обязательны.
Rustup/Cargo caches задаются как существующие external volumes; offline cache
должен содержать locked dependencies обоих producers и Fleet. Отдельный named
target cache не должен одновременно использоваться другим QA.
Существующий target cache переиспользуется только при совпадении task/purpose
labels этого harness; чужой volume не принимается. External Cargo/Rustup caches
не удаляются и не переименовываются.
При неполном cache явный `--allow-registry` разрешает только обычные locked Cargo
downloads; lockfiles и Git/SDK pins не изменяются. Сам test всегда offline.

```text
python scripts/pm_credentials_live/run.py --base <Base-checkout> --tracker <Tracker-checkout> --sdk <Base-Git-checkout-with-9408802> --cargo-cache <existing-cargo-volume> --rustup-cache <existing-rustup-volume>
```

Default producer refs: Base `ddfb436bf2b3253561672c92b2dbc06803cabf90`,
Tracker `af6ed1ee26f6d26534a0dd1526e3b4d168962160`.
`--base-ref`/`--tracker-ref` позволяют явно выбрать другой immutable commit.
Fleet snapshot — HEAD плюс только новый test file; его SHA256 записывается в evidence.
SDK каждого consumer берётся из `.namespace-base-revision`, если этот cohort
закреплён проектом, иначе из `.base-revision`, как в штатной сборке. Fleet и
Tracker получают отдельные соседние snapshots `services-base`, поэтому их pins
могут отличаться. Каждый pin должен быть полным SHA существующего commit;
repin/checkout/reset нет. Оба SHA записываются в evidence `sdk_pins`.
Git archive extraction допускает только directories/regular files и не копирует
private skills/package в QA Rust SDK snapshots.

## Fixture И Проверки

Harness создаёт уникальный `sdlc-qa-pm-live-*` Compose project, labels task/purpose,
отдельную сеть и две tmpfs disposable БД. Выполняются только targeted locked
build двух producer binaries, compile/clippy нового test target и его rustfmt check.
Нет полного workspace build/test gate. Main QA targets не используются.

Base registration включена только в disposable fixture. Два account создаются
через `/auth/register`, PM parent выдаётся через `/auth/tokens` с ровно
`task-tracker:read/write`. Policy разрешает exact PM central UUID и prefix
`task-tracker:sdlc:pm:`. Base restart завершает policy configuration до первой
Tracker JWKS initialization; human login повторяется после restart.

Tracker migrations выполняются его настоящим server. Seeds создают только active
local users с отличающимися UUID/actual Base central subjects, project, board и PM
membership. Draft/input/assignment/execution/history не подделываются SQL:
owner JWT создаёт Draft и PM reservation через production HTTP routes.
Agent UUID — declared selector, не реальный Fleet runtime/readiness proof.

Test derives exact execution identity из actual reservation и owner readback,
вызывает production `PmCredentialCommand`/`PmCredentialIssuer`, читает Base
introspection и Tracker current context. Проверяются canonical producer DTOs,
exact assignment/subject/permissions, одинаковые replay child ID/secret/expiry,
single persisted Base delegation, actual parent lineage и distinct local identities.
Negative requests проходят actual Tracker server boundary: foreign task, legacy
issue route, reservation readback и owner-only confirmation. После authenticated
Base parent revocation child introspection/context и новое issuance возвращают 401.
Empty 403 body legacy middleware допустим: проверяется точный статус;
successful producer DTOs по-прежнему требуют полный JSON round-trip.

Credentials существуют только в memory и temporary `fixture.json`, который удаляется
в finally; ни assertions, ни evidence не печатают bearers. Compose/evidence не
содержат parent/child secrets. Disposable local JWT config secret не production data.
Producer containers получают только binary mount, без test credential fixture.
Проверки останавливают проект через exact `down --remove-orphans` даже при ошибке;
volumes/caches не удаляются. Source/evidence остаются под ignored `tmp/` Fleet.
При прерывании процесса использовать exact project/compose path из вывода для cleanup.

## Проверенное Evidence

2026-10-04: Compose `sdlc-qa-pm-live-af76a8a560fd`, Linux Rust 1.88.0,
PostgreSQL 17.6. Один actual integration test PASS, scoped clippy `-D warnings`
и rustfmt PASS; build/test/cleanup exit codes `0/0/0`. После exact down
контейнеры и сеть проекта отсутствуют; private fixture удалён, caches сохранены.
Fixture bootstrap использовал explicit `postgres` database. Producer refs и SDK
соответствуют указанным выше default pins; Fleet source snapshot:
`605e19b1556278fb7acef5b917ab047856053a7f` плюс этот test.
Test SHA256:
`80fff1364630049ea736910361bc104b16a340eec51002e958510d8221db8fba`.
Harness сохраняет `evidence.json` с source/test/script hashes и exit codes
под ignored `tmp/pm-credentials-live/<project>-<unique>/`; secrets туда не входят.

## Границы Доказательства

Это один ignored integration test с несколькими positive/negative checks.
Harness не выдаёт synthetic ACK, не создаёт trusted admission receipt, не меняет
catalog defaults/SDK pins и не запускает Hermes. Durable Fleet journal/coordinator
и assignment replacement/lost-ACK transport fault injection — отдельные тесты.
