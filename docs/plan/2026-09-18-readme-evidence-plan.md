# Fleet Control README Evidence Plan

> **Status 2026-09-18:** active Base README migration wave. Scope is documentation, existing reviewed blank-login evidence and a structural CI gate; no runtime supervisor, API or schema behavior changes.

## Evidence Decision

- Root README uses the blank, unbranded initial operator sign-in screen at `1920x1080` and `375x812`: both contain no credentials, PII, URLs, filesystem paths or operational data.
- Fleet dashboard, alerts and session views remain in the complete manifest rather than root README because their synthetic fixtures expose agent names, namespaces, UUID-shaped values, locale-specific future timestamps and roadmap language.

## Execution

1. Add a test-first README validator for anchors, reviewed assets, local-image existence and accidental local-path/placeholder leaks.
2. Add a standalone CI README job.
3. Replace the repeated route/gallery matrix with current runtime boundaries and an evidence-first entry point.
4. Add an indigo/ice local banner and preserve the full 132-screen manifest as the route-level evidence source.
5. Verify backend/frontend gates, local Compose liveness/metrics, browser smoke and hosted CI before publishing.
