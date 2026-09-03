# CI/CD

Expected CI jobs:

- backend formatting
- backend compile
- backend clippy with `-D warnings`
- backend tests
- migration clean DB up/status
- OpenAPI regeneration and diff check
- frontend API generation
- frontend typecheck
- frontend lint
- frontend format check
- frontend unit tests
- frontend build
- Playwright Chromium, Firefox and WebKit
- screenshot generation through `pnpm screenshots:local`
- screenshot manifest verification
- markdown link check through `pnpm markdown:check`

CI should fail on generated artifact drift, migration drift, unredacted secret
fixtures, missing screenshots and stale docs indexes.
