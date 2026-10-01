# Fleet Platform Header

## Scope

Fleet adopts the merged Base #119 `PlatformHeader`. Its order is leading,
services, actions; the full-width 60 px header precedes the sidebar content
offset. Desktop navigation starts below the header, 72 px compact / 264 px
expanded. Account identity no longer appears in the sidebar, header text and
drawer simultaneously. Name falls back to email and then the localized
operator label; identical name/email is shown once. Long identity wraps in a
bounded menu.

At 320 px, the brand mark is hidden while the service button retains the full
accessible application name. Mobile buttons/nav links are at least 44 px,
desktop 40 px. Drawer navigation, Escape and desktop resize close the modal;
the media listener is removed on unmount. Page-specific create commands,
permissions, PageFrame modes, backend/API and central sign-out order remain
unchanged. No primary-checkout WIP is adopted.

## Validation

The shell suite has 17 cases; the full frontend suite has 117. Offline frozen
install/codegen, lint/semantic, typecheck, OpenAPI compatibility, production
build and full Prettier are required. Four existing entry/auth formatting
deviations were normalized without changing their behavior.

The unchanged backend checker was compared with all 62 tracked backend
source/manifest/migration files (LF-normalized). A fresh workspace fmt/clippy/
test/OpenAPI gate passes 60 tests, including two actually executed PostgreSQL
tests in an owned temporary database. The first run without DB variables is
not counted as fresh PostgreSQL coverage. The temporary database/network are
removed, never user volumes.

The complete five-test candidate live gate passed 5/5, 5.7 minutes, no skipped
tests, Playwright retries or mocked API. It covers 99 Header + 120 main pages +
90 agent-tab + 84 detail-geometry combinations and four localization viewports.
Keyboard/touch, six healthy runtime UI entries, identity/focus and central
logout/re-entry pass; layout checks have no overflow, unexpected scroller,
console/request errors or serious/critical axe. Four full-page PNGs were opened
and inspected. Fingerprints are in the [evidence](../assets/screens/2026-10-01-platform-header/README.md).
The post-merge image must be
rebuilt from main and repeat those five tests with the full twelve-test SSO
file. Earlier green snapshots do not satisfy that gate.

## Limits

This is Fleet header acceptance, not completion of the Workflow equivalent or
the final whole-platform release. Source fingerprints are not OCI revision
attestations. The working sdlc-demo is not rolled out by these tests. Agents
created by detail acceptance are never started and are archived via their own
API; session/audit history without delete endpoints remains only in the
isolated QA project until cleanup. No external repository/fork is created.
