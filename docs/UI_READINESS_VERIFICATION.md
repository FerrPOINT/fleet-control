# UI Readiness Refresh Verification

## Scope

On the agent configuration page, a failed readiness refresh shows `Unknown`
for both runtime and SDLC status. The cached effective revision, existing error
panel and retry control remain visible. A successful retry restores the
endpoint's statuses. Only the two `readiness.isError` badge conditions change;
API, backend readiness, authentication, migrations and locale strings are unchanged.

## Verified Checks

Fleet baseline: `c8093aace07e54436893c5f7e35df1f968690266`; pinned Base:
`875cac2edf1a18c3a8a59e2f67256d02a8fc04e4`. On 2026-10-09, using Node 22.20.0
and pnpm 10.28.1:

- Both frozen installs, typecheck, lint, format, build and all 159 unit tests: PASS.
- OpenAPI drift/compatibility, UI contract, packed Base consumer, effective
  theme contrast, Markdown links and existing 135-screenshot verification: PASS.
- [Focused component regression](../frontend/src/pages/agent-detail/config-revisions.test.tsx):
  PASS with query retries disabled.
- [Readiness browser regressions](../frontend/e2e/fleet-control.spec.ts) on native
  Windows: 9 PASS,
  0 failed/skipped/flaky, across Chromium, Firefox and WebKit at 375x812,
  1920x1080 and 2560x1440. Application query retries remain enabled; Playwright
  retries are disabled.

The tests cover initial `Running` / `Ready`, failed refresh yielding exactly
two `Unknown` badges, retained revision 7, keyboard retry and healthy recovery.
Browser checks also verify no horizontal overflow or page errors, and that
badges, error and retry are within the viewport and unoccluded.

Native Windows run `r4` rechecked HEAD `0fc8826`. The Linux investigation below
supersedes its keyboard procedure. Earlier unit/build/contract checks above
remain historical checks; this test-only correction does not rerun a build.

## Linux WebKit Keyboard Regression

Both published-head CI runs failed only the three readiness WebKit viewports:
[37857695332](https://github.com/FerrPOINT/fleet-control/actions/runs/37857695332)
on `0fc8826` with `Shift+Tab`, and
[37860129682](https://github.com/FerrPOINT/fleet-control/actions/runs/37860129682)
on `d3470e6` with `Shift+Alt+Tab`. The failed-refresh assertions had passed;
the failure was `not.toBeFocused()` after reverse traversal.

Local reproduction uses Ubuntu 24.04.4 LTS in WSL2, private Linux Node
`v22.20.0`, the existing locked Playwright `1.61.1`, and its Linux WebKit
`26.5`, revision `2311`. No repository dependencies or CI settings changed.

The minimal input/button fixture isolates the setup effect. The application
reproduces the failing mouse-start and successful keyboard-start cases:

| Setup                                                   | Reverse key                | Result                      |
| ------------------------------------------------------- | -------------------------- | --------------------------- |
| Mouse click Retry, then focus the already focused Retry | Shift+Tab or Shift+Alt+Tab | Retry remains focused       |
| Mouse click Retry, then bring the page to front         | Shift+Tab or Shift+Alt+Tab | Retry remains focused       |
| Focus Retry, then keyboard Enter                        | Shift+Tab                  | Previous button gains focus |
| Previous button focused by Shift+Tab                    | Tab                        | Retry regains focus         |

Failing events reach Retry with `isTrusted=true`, `defaultPrevented=false`, and
`document.hasFocus()=true`; no focusout/focusin follows the reverse Tab. This
reproduces without React or application handlers. Linux Tab also visits adjacent
buttons without Alt. The cited
[Playwright maintainer explanation](https://github.com/microsoft/playwright/issues/5609#issuecomment-832684772)
specifically describes **Mac** Option behavior, not a Linux default.

The evidence isolates a mouse-origin reverse-navigation behavior in the tested
WebKit browser rather than an application focus trap or an unfocused window.
Its internal cause is consistent with WebKit keeping a descendant navigation
starting node after a click, separately from the focused button: see
[Document::focusNavigationStartingNode](https://github.com/WebKit/WebKit/blob/main/Source/WebCore/dom/Document.cpp)
and [FocusController](https://github.com/WebKit/WebKit/blob/main/Source/WebCore/page/FocusController.cpp).
That internal explanation is an inference from upstream source and the controlled
experiments, not an instrumented assertion about the compiled browser internals.

The correction starts failed refresh using focused Retry plus keyboard `Enter`.
After the normal HTTP retries, it asserts that Retry **still** has focus instead
of calling `focus()` again. It retains both departure and return assertions:
`Shift+Tab` leaves Retry, `Tab` returns, and `Enter` performs healthy recovery.
Recorded application events show `Retry -> Save configuration -> Retry` at all
three viewports. Alt is now limited to macOS WebKit; macOS was not locally tested.
Application retry settings, assertion timeouts and viewport/browser coverage are
unchanged. Production component blob remains
`d337e82609d128c927d43ebdef27c81276c8880a`.

The [generated manifest](assets/screens/readiness-refresh-error/manifest.json)
records source HEAD `d3470e60e97d8fbf70ded9aa4310728dc3a7eb8d` and corrected test
blob `a75f3ae5fbc56c1e5de7946d24149bde032bfaa4`, plus actual run statistics.

| Actual local gate                                                | Passed | Failed / skipped / flaky | Playwright retries |
| ---------------------------------------------------------------- | ------ | ------------------------ | ------------------ |
| Linux WebKit, uninstrumented spec, all three viewports           | 3      | 0 / 0 / 0                | 0                  |
| Linux WebKit, same cases with event recording                    | 3      | 0 / 0 / 0                | 0                  |
| Native Windows, Chromium / Firefox / WebKit, all three viewports | 9      | 0 / 0 / 0                | 0                  |

Private raw evidence is retained under workspace `.local/fleet-webkit-proof-20261009/evidence`:
`minimal-focus.json`, `minimal-followup.json`, `minimal-keyboard-entry.json`,
`linux-baseline-localhost`, `linux-correction-events`, `linux-webkit-final`, and
`windows-regates`. The latter runs include JSON reports, traces, logs and receipts.
The failed reproduction is retained separately from the passing correction.
These local receipts precede publication of the corrected patch. Neither failed
CI run is reported as green; published-head CI must pass independently after
review and publication.

## Mock Fixture Screens

The [generated manifest](assets/screens/readiness-refresh-error/manifest.json)
records nine PNGs with exact viewport dimensions and SHA-256 hashes. All nine
were visually inspected. Captures use `fullPage: false` after scrolling the
readiness section into view. These are mocked API fixtures in the default dark
theme; the existing 135-screenshot manifest is unchanged.

The earlier `r4` preview used `http://localhost:43971`. The Linux correction and
Windows regates serve the existing `frontend/dist` on separately bound private
loopback ports; no existing listener is reused. Each helper closes its browser
and HTTP server in `finally`, and receipts record port and `serverClosed`.
Final cleanup checks confirm no owned listener or browser process remains.

Linux Node and the browser cache remain under the task-owned
`/home/sdlc1-runner/.local/fleet-webkit-proof-20261009` for reproduction. Only the
missing WebKit shared-library packages were installed with
`apt-get --no-install-recommends`: 42 new packages including dependencies,
0 upgraded and 0 removed. No global Node symlink or system GTK preference was
changed. Docker, Rust, builds, database commands and protected runtime changes
were not used; live Forge was not accessed.

## Review integration 2026-10-09

Интегрирован актуальный main с Base pin
`19a7a381ae6dbea61a643bb96189e483fa64df5c` и исправлениями process-log/heartbeat.
Component regression расширена на три легитимных состояния с разными agent IDs:
running/ready, running/blocked, stopped/blocked. Все три проходят; lint изменённых
файлов и финальная UI-сборка проходят. Production component остаётся прежним;
backend/API/configuration не меняются.

Дополнительно Codex in-app browser проверил финальный built UI на отдельном
`http://localhost:7998` с API/SSO fixtures из committed E2E helper. После настоящего
HTTP 503 видны ровно два «Неизвестно», прежняя редакция 7, error panel и retry.
Shift+Tab переводит фокус на «Сохранить конфигурацию», Tab возвращает на «Повторить»,
Enter после восстановления fixture возвращает «Работает»/«Готов» без ошибки.
Сняты новые screenshots на 375/1920/2560; мобильного horizontal overflow нет.
Загруженный `/assets/index-BYYdn9pj.js` имеет SHA256
`3d1a8ea8e2cbc1f5fe7afa6fde33d98c97df51b533c919aa0f4744b1b8d67d30`,
совпадающий с final local build. Evidence хранится в приватном workspace review
ledger; это UI fixture proof, а не проверка установленного runtime. Свежий
exact-head hosted CI остаётся отдельным merge gate.

## Acceptance limits

The browser gate covers these nine focused UI cases. It does not establish live
SSO, physical runtime readiness, backend recovery or complete autonomous SDLC.
The build passes with Vite's large-chunk warning.
