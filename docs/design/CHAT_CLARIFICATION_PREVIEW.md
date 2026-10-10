# Chat And Clarification Design Proposal

Status: displayed design approved by the user on 1 October 2026; API integration requested.
Fictional task UX-101. Design approval is not live implementation acceptance.
Entry point: `frontend/chat-preview.html#/`. Production `index.html` does not import this preview.
All decisions, messages and publication state are in memory. No authentication, API clients, runtime, credentials or Tracker writes are connected.

## Pages And Display Priority

The task/agent chat is one workspace, with Dialogue and Clarification tabs. It must not combine transcripts from different concrete agents. This preview shows only a PM chat; it does not create a task binding.

Always visible: task key/title, concrete agent display name, stage, human waiting reason, privacy, current requirements revision, transcript and composer. During an active run the integration must expose an explicit steer instead of opening a parallel run. Runtime health and business waiting are different signals.

Desktop aside / mobile context drawer: immutable task/root identity, owner, workflow/config revision, execution/attempt, checkpoint, requirements checklist, acceptance criteria, results and verified CI/CD receipts. Display absent data as unknown/unavailable, not fabricated readiness.

Details on demand: runtime session/run IDs, model/provider, tool events, logs and diagnostic protocol codes. These do not precede the conversation on mobile.

## Clarification Contract

Task Tracker owns question IDs, versions, answers, requirement revisions and publication. Fleet renders them within the PM task chat and controls runtime checkpoint/resume. A prompt, tool permission approval and a business clarification are separate types.

Required question fields: ID, task/root ID, requirement reference/revision, request/execution ID, question text, rationale, required flag, answer mode (single/multiple/text), option IDs/labels/consequences, optional PM recommendation and sources, state, answered-by/time. PM suggestions are not selected automatically and do not authorize dangerous actions.

Submitting requires explicit choice or required custom text, authenticated owner/project access, optimistic question/revision preconditions and an idempotency key. A changed question must reject a stale answer and preserve the local draft. Retry after unknown acceptance requires readback. Closed/cancelled requests cannot resume an unrelated execution.

All required answers close the clarification checklist, not the publication gate. Human confirmation applies to an exact requirements revision, including scope, constraints, assumptions, scenarios and acceptance criteria. The prototype's abbreviated confirmation is not the future authoritative full requirements document. Publication still requires backend readiness and prerequisites.

Later-stage critical escalation returns a scoped decision to PM/root requirements, retaining source stage, impact, checked evidence, recommendation and checkpoint. Infrastructure failures go to operator rather than becoming business questions. New answers do not automatically grant runtime tool permissions.

## Preview Actions

- Dialogue/Clarification navigation is hash-query state and supports browser history.
- Select one answer, optionally comment, or provide a required custom answer.
- Submit creates an in-memory transcript entry and marks question answered.
- Review and explicit checkbox confirmation change only simulated publication state.
- Context opens in an accessible Base dialog on smaller screens.
- `state=read-only` disables writes. `state=conflict` demonstrates a stale answer without discarding input.

## Integration Requirements At Preview Baseline

Tracker: clarification list/detail/versioned answer/readback; full requirements revision/detail/diff; checklist readiness and exact-revision confirmation.
Fleet: immutable task/agent chat identity; structured waiting events; execution/attempt/checkpoint references and safe runtime resume.
Workflow: explicit checkpoint/rebind and terminal receipt. None of these are supplied by the mock or by a title containing a task key.

## Acceptance Before Integration

Review desktop and mobile screenshots with the user. Verify keyboard radio navigation, focus/closing context and confirmation dialogs, custom-answer validation, read-only/conflict behavior, no external API requests, and no preview code in the production bundle. Approval must be explicit and recorded here before integrating controllers.

## Verification Evidence

Local preview: <http://127.0.0.1:55494/chat-preview.html#/?view=chat>.
Clarification: <http://127.0.0.1:55494/chat-preview.html#/?view=clarify>.

- Five focused Vitest tests pass: explicit selection/separate revision confirmation, custom answer validation, conflict draft preservation, read-only actions, keyboard tab navigation.
- Typecheck, focused ESLint, isolated preview build and production build pass. Production output was checked for preview fixture leakage; no matching fixture strings were found.
- Browser screenshots cover chat and clarification at 375x812, 1920x1080 and 2560x1440, plus the requirements review dialog at 768x1024. Mobile composer and answer action remain visible; both main views have no document-level horizontal overflow at these sizes.
- Browser radio-key navigation and context dialog closing were checked. Network capture recorded 28 requests to the preview origin only, with no business API requests. This is a preview isolation check, not an end-to-end runtime acceptance.
- [Generated screenshot manifest](../assets/design/chat-clarification/manifest.json) records the captured proposal screens and file hashes. It is separate from production route evidence.

Design approval recorded on 1 October 2026. Tracker storage/confirmation and Workflow
continuation contracts now have separate implementations; live integration remains incomplete.
See [implementation plan](../CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md).

## Production Controller Fixture Evidence

The production `/chats/:sessionId` component has separate browser evidence using fixture API
responses, not a live PM. Three browsers verify explicit choices, preserved drafts and exact
confirmation; the three tabs are captured at 375x812, 1920x1080 and 2560x1440.
The [generated controller manifest](../assets/design/chat-controller/manifest.json) sets
`liveAcceptance=false`. Regenerate after the focused Playwright flow with
`node scripts/publish-chat-controller-evidence.mjs` from frontend. This does not update or
claim current live production screenshots.
