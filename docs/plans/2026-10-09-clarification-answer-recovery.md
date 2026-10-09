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
newly selected question. Ownership and successful context/question reads still
gate recovery. New-answer permission gates new answers, not acknowledgement of
an original retained command. Acknowledgement unlocks subsequent answers, clears only the
original draft and does not confirm requirements or dispatch a runtime run.

## Initial Executed Evidence

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

## Closed-Question Follow-Up

The pinned Tracker source357caa7a computes `can_answer` from owner, stage and
whether any question remains open. Saving the final answer therefore closes
that flag before a client with a lost response can acknowledge its original key.
The original UI incorrectly disabled that recovery action. A new regression
reproduces this exact disabled-button failure before the follow-up fix.

Explicit replay now requires the retained original variables, current session
ownership, a task binding, available Tracker context and successful question
reads; it does not require permission to create another answer. The submit and
editing controls still require `can_answer`. No key, body or question version is
regenerated. Fleet gateway authorization is unchanged. Tracker357caa7a authorizes
the owner and project before exact idempotency replay, and replays before applying
new-answer business gates; this source read is not deployed HTTP acceptance.

Node22 passes39 chat-page cases and334 total cases across36 files. New negative
cases deny replay after a changed session owner or failed fresh Tracker access.
Typecheck, targeted ESLint and Prettier pass. The first attempted reproduction
had a fork-worker startup timeout and executed no tests; the next attempt
reproduced the actual product defect. Two initial negative tests were corrected
to await React Query notifications rather than inspect before rendering updates.
The follow-up browser run `clarification-closed-final-20261009` terminates3/3
PASS in2.7m across Chromium/Firefox/WebKit. It also closes the question/permission
snapshot after the second lost response and proves the third command still
equals the original. Nine current captures replace the generated manifest;
hashes, dimensions and routes pass, and mobile/desktop captures were opened and
visually inspected. Vite build passes3870 modules in13.82s with the existing
large-chunk warning unchanged. The earlier evidence above is historical, not
relabeled as this new run.

Independent source review of8e5d75d finds no actionable product defect, but notes
that a disabled radio alone cannot prove that false permission has rendered.
The successor tests explicitly await the changed waiting reason, false context
permission and answered question before replay. Reintroducing only the old
`can_answer` retry guard makes the ordered regression fail at its expected
enabled-button assertion; restoring the production guard makes it pass. The
production component is byte-identical before and after that controlled negative
check. The full334-case suite passes again, as do typecheck/lint/format.
Ordered browser run `clarification-closed-ordered-20261009` passes3/3 in34.2s;
it waits for rendered closed-question context before the identical third command.
Its nine generated captures replace the manifest, pass verification and were
visually checked on mobile/desktop. This is test-ordering evidence, not a new
product or authorization change.

## Remaining Scope

This fix preserves an original command for the mounted chat lifetime; it does
not add durable browser answer recovery after reload. Tracker remains the
authoritative answer/idempotency store. Live PM tools, pre-model admission,
checkpoint/rebind and delivery acceptance remain separate open requirements.
No fixture, saved answer or passing UI test proves those requirements.
