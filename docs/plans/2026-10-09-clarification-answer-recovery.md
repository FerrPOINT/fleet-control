# Clarification: Recovery Of An Unknown Answer

Date: 2026-10-09. Scope: production chat controller, not live PM execution.
Baseline: Fleet integration778abf7 with SDK19a7a381; no backend, public API,
database, credential or accepted-runtime changes.

## Defect And Change

Two new regressions failed on the original controller. After an unknown answer
outcome, switching questions allowed a second mutation to replace the original
command. A newer question version also allowed transferring the retained draft
under a new key before the original outcome was established.

The controller now blocks new answers, editing and draft transfer while an answer
outcome is unknown. Its separate recovery action uses the original mutation's
question ID, expected versions, payload and key regardless of current selection.
The original question title is captured for display rather than taken from a
newly selected question. Ownership and fresh context/question permissions still
gate recovery. Acknowledgement unlocks subsequent answers, clears only the
original draft and does not confirm requirements or dispatch a runtime run.

## Executed Evidence

- Initial negative reproduction: two tests fail at the intended enabled-control
  assertions before the controller fix.
- Final Node22.20.0 unit suite: 36 files,331 tests pass. Three regressions cover
  switching questions, incoming question versions and unlock after acknowledgement.
- Generated TypeScript from the unchanged OpenAPI and typecheck pass.
- ESLint, semantic classes, targeted Prettier and120 Markdown link checks pass.
- Production Vite build passes (3870 modules); its existing large-chunk warning
  remains a warning, not a runtime or PM acceptance result.
- Final browser scenario passes3/3 in1.0m across all three engines. Mobile and
  desktop captures were visually inspected; the original question is identified
  separately from the currently selected question.
- Browser fixture runs and generated screenshots are recorded in the
  [manifest](../assets/design/clarification-uncertain/manifest.json).
  The manifest is verified against PNG hashes, dimensions, routes and the complete
  Chromium/Firefox/WebKit by375/1920/2560 matrix. Existing runtime-control evidence
  remains separately verified, not overwritten or relabeled.

Reproduction with the pinned Node runtime from the frontend directory:

```text
pnpm exec vitest run
pnpm typecheck
PLAYWRIGHT_BASE_URL=http://127.0.0.1:4173 pnpm exec playwright test e2e/runtime-controls.spec.ts --grep "uncertain clarification" --workers=1 --output=test-results/clarification-uncertain-final-20261009
node scripts/runtime-controls-evidence.mjs --scenario clarification-uncertain --input test-results/clarification-uncertain-final-20261009
node scripts/runtime-controls-evidence.mjs --scenario clarification-uncertain --verify
node scripts/runtime-controls-evidence.mjs --verify
```

The fixture serves its own production component bundle and mocked HTTP; no
accepted service is restarted. Screenshot evidence has `liveAcceptance=false`.

## Remaining Scope

This fix preserves an original command for the mounted chat lifetime; it does
not add durable browser answer recovery after reload. Tracker remains the
authoritative answer/idempotency store. Live PM tools, pre-model admission,
checkpoint/rebind and delivery acceptance remain separate open requirements.
No fixture, saved answer or passing UI test proves those requirements.
