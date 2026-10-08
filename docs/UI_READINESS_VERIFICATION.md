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
- [Readiness browser regressions](../frontend/e2e/fleet-control.spec.ts): 9 PASS,
  0 failed/skipped/flaky, across Chromium, Firefox and WebKit at 375x812,
  1920x1080 and 2560x1440. Application query retries remain enabled; Playwright
  retries are disabled.

The tests cover initial `Running` / `Ready`, failed refresh yielding exactly
two `Unknown` badges, retained revision 7, keyboard retry and healthy recovery.
Browser checks also verify no horizontal overflow or page errors, and that
badges, error and retry are within the viewport and unoccluded.

## Mock Fixture Screens

The [generated manifest](assets/screens/readiness-refresh-error/manifest.json)
records nine PNGs with exact viewport dimensions and SHA-256 hashes. All nine
were visually inspected. Captures use `fullPage: false` after scrolling the
readiness section into view. These are mocked API fixtures in the default dark
theme; the existing 135-screenshot manifest is unchanged.

The owned preview used `http://localhost:43971`, strict port binding and
`reuseExistingServer: false`. Cleanup confirmed no remaining listener; all
verification handles are terminal. Failed attempts remain in private logs.

## Limits

The browser gate covers these nine focused UI cases. It does not establish live
SSO, physical runtime readiness, backend recovery or complete autonomous SDLC.
The build passes with Vite's large-chunk warning.
