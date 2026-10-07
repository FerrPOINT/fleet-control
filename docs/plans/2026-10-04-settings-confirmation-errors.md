# Ошибка текущего подтверждения настроек

Реальный apply возвращает 409 при изменении active version после preview.
После отмены и нового успешного preview диалог показывает ошибку предыдущей
операции. Воспроизведение использовало собственный SSO/operator и одноразовую
Fleet БД; успешного apply/restart не было. Обычный локальный пользователь
получил ожидаемый 403 до подготовки тестовой роли operator.

## Изменение

Ошибка относится к текущему подтверждению. Новый непустой успешный preview
очищает результаты apply и rollback перед открытием диалога. Отмена очищает
обе mutation state после проверки, что операция не выполняется. Retry внутри
того же диалога сохраняет draft, target и expected version.

На production preview после Cancel и Escape активным элементом оставался BODY:
у controlled AlertDialog отсутствует Trigger. Продукт сохраняет кнопку,
запустившую успешный preview, и возвращает ей focus через существующий
onCloseAutoFocus Base, если кнопка всё ещё подключена к документу. Сохраняются
rich preview списка изменений, keyboard и продуктовые apply/rollback операции;
продукт владеет pending/error state. Общий HTTP/OpenAPI, роли, scopes,
данные, migrations, runtime admission и operational defaults не меняются.

## Проверки

Регрессии отмены и нового подтверждения между apply/rollback, существующего
retry и pending guards. Полные frontend/contract/packed-consumer/theme gates
с frozen locks и pinned Base. Production QA с настоящим 409 и последующим
200 preview без API mocks; viewport/темы/keyboard/focus. Только одноразовые
SQL fixtures, без успешного apply/restart и изменений accepted runtime.
Backend/OpenAPI/Base pin/lock objects сверяются с исходным commit.
Снимки и evidence сохраняются в `docs/assets/screens`.

Подтверждённые результаты и снимки: [production evidence](../assets/screens/settings-confirmation-2026-10-04/README.md).
