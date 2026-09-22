# Fleet Control README Evidence Plan

> **Status 2026-09-22:** historical record of the initial README migration. The active Base contract is desktop-only; auth and narrow-viewport captures remain UI QA, not README or manifest evidence.

## Evidence Decision

- Do not use auth or narrow-viewport captures in the root README or manifest.
- Fleet dashboard, alerts and session views remain in the complete manifest rather than root README because their synthetic fixtures expose agent names, namespaces, UUID-shaped values, locale-specific future timestamps and roadmap language.

## Execution

1. Add a test-first README validator for anchors, reviewed assets, local-image existence and accidental local-path/placeholder leaks.
2. Add a standalone CI README job.
3. Replace the repeated route/gallery matrix with current runtime boundaries and an evidence-first entry point.
4. Preserve the generated 82-screen desktop manifest as the route-level evidence source.
5. Verify backend/frontend gates, local Compose liveness/metrics, browser smoke and hosted CI before publishing.
