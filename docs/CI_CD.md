# CI/CD

The current workflow is `.github/workflows/ci.yml`. Its foundation gates are:

- backend formatting
- backend compile
- backend clippy with `-D warnings`
- backend tests
- migration clean DB up/status, rollback the four new revisions and reapply
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
- installed Base snapshot and route contract check through `pnpm ui:check`
- browser evidence artifact retained for 14 days

These workflow edits have not yet run on GitHub. Browser jobs use fixtures;
opt-in live checks remain skipped without a live acceptance environment.
Screenshot regeneration and manifest verification do not prove a real deployment.

Both jobs depend on the sibling `services-base` checkout and the configured
`SERVICES_BASE_TOKEN`. Compatible Base changes must be published before Fleet
can pass from a clean clone; a local dirty checkout is not a released dependency.

Cross-service contracts, mandatory secret smoke and Docker compose health smoke
remain rollout gates in [SDLC implementation](SDLC_IMPLEMENTATION.md). A green
foundation job alone must not be presented as 100% SDLC acceptance.
