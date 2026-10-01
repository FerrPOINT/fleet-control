# Chat Clarification Verification

Date: 2026-10-01. Status: verified foundation, incomplete approved vertical slice.
No real PM publication/resume or live Backlog acceptance is claimed.

## Source Baseline

Fleet foundation: `470f2856735bde5eff5bc969b6973a6d38044b12`.
Tracker implementation: `0cffd7d8c5fe5d9b12776c226635780beb583a4f`.
Workflow implementation: `a99a5d8`.
Existing unrelated sibling workspaces were not changed. Each service has its own branch.

## Verified Fleet Gates

- Linux Rust 1.88: format, all-target check, strict Clippy, workspace tests and source
  OpenAPI regeneration/diff. 69 library tests; 11 actual PostgreSQL foundation cases.
- Isolated PostgreSQL 17.6: clean ten-migration up, central-subject regression with its
  own configured database URL, migration 000010 down/reapply/status.
- Frontend: generated API drift, seven Tracker wire-contract comparisons, checker tests,
  typecheck, production build, lint and format; 146 unit tests passed.
- Playwright: 21 fixtures passed in Chromium/Firefox/WebKit. PM tabs have no document-level
  overflow at 375x812, 1920x1080 and 2560x1440, no serious/critical scoped axe violations,
  tablet Escape/focus restoration and keyboard tab navigation. Nine generated controller
  images have verified routes/viewports/content hashes; liveAcceptance is explicitly false.
- Controller browser evidence is fixture-only. The real chat component uses fixed-origin
  gateway clients; mocked APIs in the browser test do not dispatch Hermes or resume Workflow.

The library/workspace gate and targeted PostgreSQL cases are separate evidence. Tests with
an absent database variable must not be described as database acceptance. Native Windows
Rust linker availability and remote CI head status remain separate gates.

## Cross-Service Evidence

Tracker's own verification ledger records strict central/project/owner/machine authorization,
durable questions/answers/revisions/confirmations/outbox and a real PostgreSQL HTTP test
with a synthetic Central Auth issuer. It does not dispatch PM. Ordinary Clippy passes;
strict Clippy is blocked by six pre-existing `collapsible_if` warnings in unrelated code.

Workflow's own ledger records 1,932 unit and 52 PostgreSQL tests, focused PM contract tests,
Ruff/mypy and generated OpenAPI checks. Its continuation API consumes a trusted Fleet probe;
that callback server and the dispatch/resume orchestrator are not implemented in this slice.

The accepted snapshot [tracker-clarification-v1.json](contracts/tracker-clarification-v1.json)
was generated from the Tracker commit above, not authored as an independent API spec.
The checker resolves schema refs and compares fields/requiredness/types/nullability/formats/
enums. Constraints and business permissions are checked by service tests, not by that wire
normalizer. Rollout must compare the actual Tracker build with the same snapshot.

## Release Blockers

Creation saga, scoped PM structured tools, Tracker outbox/Fleet inbox projection, trusted
actual Hermes readback, terminal old-run verification and workflow resume/rebind, independent
readiness verifier, authorized server list search/counts/pagination and live restart/denial
acceptance remain open in [GAP_REGISTER](GAP_REGISTER.md).

Task-bound ordinary prompts and steer are intentionally rejected. The answer receipt means
Tracker saved an answer, not PM received it. No deployment switch, merge or production-ready
claim should bypass these gaps. Fixture images remain outside the live screenshot manifest.
