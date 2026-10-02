# Chats And PM Clarification Implementation Plan

Approved scope: user request on 1 October 2026. Status: implementation in progress,
not live accepted. The preview design is approved; fixture evidence is not runtime evidence.

## Goal And Boundaries

Deliver Tracker Draft -> concrete PM chat -> structured clarification -> final
requirements revision -> owner confirmation -> Backlog. Keep separate chats per
agent and task; retries continue the same chat. Leaders, a second scheduler and
the complete autonomous SDLC are outside this vertical slice.

Tracker owns questions, answers, revisions, confirmations and business state.
Fleet owns chat history, runs and projections. Workflow owns steps and checkpoints.
Base supplies shared UI/auth primitives, never clarification business logic.

## Backend And Contracts

- Resolve cross-service users by verified central subject, not email or local UUID.
- Enforce strict project access and task ownership regardless of central-auth bypass.
  Operator read-all does not authorize owner confirmation or answering as the owner.
- Add immutable Tracker instance/project/task/root binding and unique instance/task/agent
  session identity. Legacy task_key/title are display-only and never auto-migrated.
- Add versioned clarification requests/questions/answers, immutable requirements
  revisions, exact-revision confirmations, idempotency records and transactional outbox.
- Question modes are single/multiple/text; options have stable IDs and consequences.
  PM recommendation never preselects an answer. States: open/answered/superseded/cancelled.
- Tracker public operations live below /api/v1/issues/{id}/sdlc. Only scoped PM machine
  identity may create questions and requirements revisions.
- Fleet exposes task-context, clarification/requirements reads and owner commands through
  a protected Tracker gateway. Cursor-paginated transcript history preserves legacy API.
- Persist a creation saga before dispatching PM. Structured PM tool calls, not Markdown
  parsing, create questions. Validate assignment/execution/agent/version on every write.
- Clarification waits preserve checkpoint. Resume only after old run is terminal or safely
  stopped, using the same execution and a new runtime run with verified workflow rebind.
- Separate answer persistence from delivery. Read back unknown acceptance before retry.
  Same idempotency key/payload replays; changed payload conflicts. Never duplicate a run.
- Answers do not publish requirements. Owner reviews full revision and diff, then confirms
  exact revision/content hash. Confirmation atomically checks checklist and prerequisites.
- Outbox/inbox events project into durable Fleet stream once; reconnect loads a snapshot.

## Production UI

- Reuse Base presentation with real controllers, keeping isolated fixture entry outside
  production. Chats use dialogue/clarification/requirements tabs and URL question selection.
- Lists retain own-user default, avatars, permitted multi-user filter, server search/counts
  and pagination. Preserve filtered return context through authentication and navigation.
- Keep task, concrete agent, stage, privacy and human waiting reason visible. Runtime health,
  command acceptance/delivery and business waiting are separate signals.
- Render transcript/stream/delivery failures without scrolling readers away from history.
  Composer performs explicit capability-gated steer for active run, blocks duplicate pending
  or uncertain dispatch, and cannot bypass a structured clarification wait.
- Show open/answered questions, progress, rationale, consequences, optional comment and custom
  answer. Preserve memory drafts between tabs, warn on navigation, retain input on conflict.
- Show full requirements, acceptance, constraints, assumptions and actual revision diff.
  Changed revision invalidates the confirmation form; publication is a separate action.
- Desktop aside/mobile drawer shows authorized context, attempts/checkpoint and verified
  evidence; absent data stays unknown. Logs and runtime identifiers are details on demand.
- Handle loading/empty/read-only/denied/stale/conflict/partial success independently.
  Tracker failure must not hide Fleet transcript. Tool approval is not business clarification.

## Sequence And Evidence

1. Save plan/design approval/contract and pin baseline; add contract tests.
2. Implement authorization, additive migrations, binding and idempotency.
3. Implement Tracker storage/gates, Fleet gateway/projection and Workflow checkpoint/resume.
4. Verify backend vertical flow, then connect production chat/clarification/requirements.
5. Synchronize documentation indexes, CHAT, architecture/data/API/runtime/security,
   operations/testing, ADRs, traceability and gap register with actual code.
6. Update CURRENT_STATE and generated production screenshots only after verification.

## Acceptance

PostgreSQL tests cover ownership/scopes, concurrent duplicates, conflicts, binding uniqueness,
stale revisions, event replay and crash/unknown dispatch recovery. Workflow tests cover safe
stop/checkpoint/rebind, late answers and unavailable capability. Frontend tests cover all answer
modes, custom input, no preselection, preserved drafts, partial success and exact confirmation.

Live acceptance: real PM creates questions, owner answers, PM creates final revision, owner
confirms, Tracker stores confirmation and Backlog. Service restart cannot create duplicate chat,
answer or run. Negative acceptance denies foreign transcript/counts/stream and operator proxy
confirmation. Unavailable dependencies cannot produce false success.

Run repository quality gates, PostgreSQL migrations/integration, OpenAPI drift, redaction,
Compose health and docs links. Browser acceptance covers Chromium/Firefox/WebKit, keyboard,
375x812/1920x1080/2560x1440 plus tablet dialogs. Separate PRs per service; opt-in rollout only
for compatible projects. Disable stops new assignments without deleting history.

## Baseline

- Fleet: merged foundation 470f2856735bde5eff5bc969b6973a6d38044b12.
- Tracker baseline: 569c1b9; PM contract implementation: 0cffd7d8c5fe5d9b12776c226635780beb583a4f.
- Workflow baseline: 7a720b1; PM continuation implementation: a99a5d8.
- Known prerequisite: Tracker's central-auth project bypass must not be inherited by SDLC.
- Preview tests/builds passed previously; live contracts and acceptance remain to be implemented.

## Implementation Ledger

Implemented: Fleet immutable bindings/history/gateway and production chat tabs; Tracker
versioned state and owner/machine boundaries; Workflow checkpoint/continuation contract.
Verified Fleet evidence is recorded in CURRENT_STATE; service-specific tests belong to
their own repositories. Preview approval does not close live acceptance.
Typed response DTOs and the accepted Tracker wire contract now have source-generated drift
checks. See [verification ledger](CHAT_CLARIFICATION_VERIFICATION.md) for the boundary
between local contract evidence and the unimplemented runtime integration.

The follow-up implements trusted machine-only Hermes readback and exact-request
approval decisions. Readback has authenticated test-runtime evidence; it is not
an implemented PM dispatch/continuation orchestrator.

Remaining before release: creation saga, runtime structured tools/scoped assignment,
authenticated Tracker outbox polling into the implemented Fleet inbox, integration of trusted readback into PM delivery/rebind,
prerequisite verifier, live restart/negative acceptance and current production screenshots.
Task-bound chat ordinary send/steer is deliberately blocked until these contracts are wired.

Implemented follow-ups: bounded server-only Base delegation client, transactional
PM terminal/capacity reconciliation, strict metadata-only Tracker decoder and
immutable projection/version pins. Producer HTTP snapshots from Tracker's own
PostgreSQL tests cover all nine event types without rewriting source digests.
These are foundations and contract evidence, not actual PM tools/delivery/resume.
