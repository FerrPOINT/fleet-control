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

Historical planning baseline, not the current integration or acceptance status:

- Fleet: merged foundation 470f2856735bde5eff5bc969b6973a6d38044b12.
- Tracker baseline: 569c1b9; PM contract implementation: 0cffd7d8c5fe5d9b12776c226635780beb583a4f.
- Workflow baseline: 7a720b1; PM continuation implementation: a99a5d8.
- Known prerequisite: Tracker's central-auth project bypass must not be inherited by SDLC.
- At that baseline, preview tests/builds had passed; runtime integration and live acceptance were still outstanding.

## Implementation Ledger

This ledger describes implemented source at integration `80b891b`, not a live
acceptance receipt. Historical baseline refs above are unchanged. Exact receipts
and failed attempts remain in the [verification ledger](CHAT_CLARIFICATION_VERIFICATION.md);
[CURRENT_STATE](CURRENT_STATE.md) and [GAP_REGISTER](GAP_REGISTER.md) track open gates.

### Implemented And Source-Reviewed

- Immutable task bindings, paginated history, protected Tracker gateway and production
  dialogue/clarification/requirements tabs; owner/project/human checks, durable
  answer commands and separate answer-save/continuation outcomes.
- Owner-only Draft/input/reservation/chat creation saga and typed API/client. With
  dispatch disabled it stops at `awaiting_admission`; the opt-in runtime path also
  returns `awaiting_runtime_acceptance` or `runtime_accepted`. Native acceptance is
  not business completion, and the production creation form is not yet connected.
- PM initial dispatch through existing Workflow/Hermes APIs, persisted immutable
  request/key/identity, capacity reservation, current assignment and effective-config
  checks. Bounded original-key replay requires attested current container proof;
  missing/expired/foreign evidence stays held, never a new key or implicit takeover.
- Restricted PM MCP tools for Tracker reads/question/revision publication and Workflow
  steps/checkpoints; durable operation journals, current-run/assignment/version fences,
  exact-run instruction receipts and server-only credentials. Tools cannot answer or
  confirm as the human owner. Receipt validation is not proof of model compliance.
- Saved-answer checkpoint continuation with old-run terminal/safe-stop readback,
  stable resume identity, Workflow rebind and a fresh run's instruction receipt.
  Accepted-run following, durable stream/final projection and terminal/capacity
  reconciliation exist; disconnect/EOF is not completion.
- Owner controls and trusted exact-request approval readback are integrated. Ordinary
  task-bound send remains denied; active PM steer/stop use current guarded authority.
  Idle PM prompts remain explicitly unsupported, without fabricated checkpoints or
  fallback private-chat dispatch. Hermes is unchanged; no custom pre-model hook,
  host controller or second scheduler is required.
- Server-only Base delegation, strict Tracker metadata decoding and cursor persistence,
  immutable projection/version pins and opt-in authenticated polling. Each cycle
  rechecks Base subject/scope and Tracker project access; polling is not PM publication.

### Qualified Evidence And Limits

Typed DTO drift checks and authentic Rust OpenAPI generation have recorded evidence;
producer fixtures and HTTP/PG component checks retain their exact source scopes.
Corrected frontend sourceaca/fc0 passes all23 gates/461 units and47 tests per browser
engine, with nine opt-in live-only skips each;135 catalogue and nine PM views are
imported with provenance. These are fixture captures, not live service acceptance.
Reviewed source and authored backend regressions do not substitute for the full
backend gate. See [CURRENT_STATE](CURRENT_STATE.md#current-evidence) for current
qualification. The afab/sourcecb1 failure and source5bc experiment are historical
attempts, not the latest status or evidence of a proven fix or full acceptance.

### Remaining Before Release And Live Acceptance

Connect the existing PM creation API only after explicit form approval. Complete
current-source backend/PG/HTTP and migration qualification, real authenticated
Tracker outbox delivery, PM tools/continuation/restart/negative flows, and owner
exact-revision confirmation through Backlog. Qualify native provisioning/config
activation/recovery and two-agent isolation; verify the remaining interactive/live
flows rather than treating fixture screenshots or runtime ACKs as business success.
General non-PM execution/workspace authority and automatic Java SDLC remain separate
compatible-producer/consumer gaps; the initial PM Draft path does not fabricate
task-workspace claims to close them. The approved target above remains unchanged;
no complete PM, native-runtime or full-SDLC acceptance is claimed.
