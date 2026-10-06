# Chats/PM Consumer Handoff: 6 October 2026

This packet is based on published Fleet
`08c6c56229ee308050176293e34d65f087b5b112`, on independent branch
`feat/chats-pm-consumer-20261006`. It changes the Chats consumer and its tests;
it does not enable PM dispatch or complete the SDLC release. The integration
checkout, runtime supervisor/adapter, container lifecycle/configuration, journals,
migrations, Base, producer repositories and accepted installation are unchanged.
The previously approved composition and legacy `/sessions` remain supported.

Consumer code commit:
[`9456647533b7ae24cd3647bc067b15bc115f4773`](https://github.com/FerrPOINT/fleet-control/commit/9456647533b7ae24cd3647bc067b15bc115f4773).
The following documentation/evidence commit belongs to the same independent
[branch](https://github.com/FerrPOINT/fleet-control/tree/feat/chats-pm-consumer-20261006).
Integrate the packet's two commits, not all historical commits between main and
its integration baseline.

## Implemented

- Unknown answer outcomes hold every question and stale-draft transfer in the
  chat. A retry uses the captured original question/version/revision/payload/key,
  including when another question/version is displayed. Readback refreshes run
  before the explicit original-command retry is enabled. No automatic POST retry.
- Confirmation mutation belongs to the session workspace, not the mounted tab.
  Pending, acknowledged and unknown results survive tab changes. A new displayed
  revision/hash resets consent; uncertainty holds new confirmations. An explicit
  retry uses the original revision/hash/key, even when a newer document exists.
  Owner/access checks still run server-side; operator read access never grants consent.
- Gateway success requires exact HTTP 200 and a receipt matching original owner,
  question/version/revision/options/text/comment or task/revision/hash/Backlog.
  Nil receipt IDs and unrelated successful DTOs become dependency uncertainty.
  This protects the consumer; Tracker remains the owner of atomic idempotency,
  prerequisites and business transitions. No optional routing field is added by UI.
- Read-only questions/revisions remain available after assignment replacement,
  subject to unchanged immutable binding and fresh Tracker project authorization.
  Mutations still require the current concrete agent; revoked access denies reads.
- Reconnect clears transient deltas and refreshes transcript, runs, controls,
  context, questions, requirements, approvals and control history. SSE cursor and
  replay remain in the pinned Base utility. An unfinished history page is not
  cancelled; a subsequent refresh catches events that arrived during its load.
- Cached questions/documents stay readable on failed refresh with an explicit
  stale notice and disabled actions. Failed first history load does not claim an
  empty transcript. Controls have an explicit read-only retry. Runtime-health,
  prompt delivery, runtime terminal state and Tracker stage remain distinct.

## Verification And Evidence

Source locations: [Chats](../frontend/src/pages/chat-detail/index.tsx),
[consumer regressions](../frontend/src/pages/chat-detail/index.test.tsx),
[gateway](../backend/api/src/routes/task_chats.rs),
[PostgreSQL/HTTP history test](../backend/infra/tests/sdlc_foundation.rs),
[browser scenarios](../frontend/e2e/fleet-control.spec.ts).

Node 22.20.0, pnpm 10.28.1, frozen lockfile and Base
`cbb4e99230420dc2659431b1c9fb5090e5c940f0` are used. Frontend typecheck, lint,
252 Vitest tests, build, format, OpenAPI generated-client/compatibility and seven
local chat-contract comparisons pass. The seven-schema local snapshot is not
proof of parity with published Tracker114; see the producer boundary below.
The build still reports the existing production chunk-size warning (>500 KiB).

Rust 1.88.0 `cargo fmt --all --check`, locked/offline workspace check with all
targets, API clippy with warnings denied, six task-chat API tests and seven
focused PostgreSQL/HTTP tests pass on the final backend bytes. Tests cover binding
immutability/concurrency, private history/idempotency, and read access after
assignment replacement with denial after project access revocation.
[Sanitized backend validation](assets/design/chats-pm-consumer/backend-validation.json)
records commands, source hashes, test groups and exact Compose cleanup. The owned
QA project has no remaining containers or networks; installed runtime is unchanged.
Workspace Docker audit found no violations on the two reachable endpoints but
was incomplete because `sdlc1-runner` was unavailable; it is not a global audit pass.

Chromium/Firefox/WebKit pass nine focused fixture browser cases: PM questions,
exact confirmation, keyboard tabs/drawer/Escape, three viewports, history order,
legacy first prompt and held delivery after reload. Screenshots for dialogue,
single/multiple/text (no preselection), requirements, confirmation and diff use
375x812, 1920x1080 and 2560x1440. Their source/hash manifest and preview files are
kept in [consumer evidence](assets/design/chats-pm-consumer/manifest.json).
These are production components with fixture API, **liveAcceptance=false**.

The installed Fleet `http://127.0.0.1:7742/chats` was opened through the browser
without changing runtime. It redirected to the real SDLC login; no authenticated
owner task, PM publication, confirmation or cross-user denial was exercised.
[Live login observation](assets/design/chats-pm-consumer/live/auth-required-1920.jpg)
is separated from preview. It is a 1920x1080 observation, not three live chat views.
No credential reset or installation change was used to bypass that boundary.

Preliminary verification failures are retained: a backend source export omitted
Base workspace manifests, then compile-time native-test helper scripts; both
owned Compose projects were cleaned. Browser capture initially reloaded before
fixture SSO finished (Firefox), then captured requests while navigating (WebKit).
The final fixture switches question modes through reconnect refresh. Image scale
was corrected to CSS pixels before publishing the screenshot manifest.

## Read-Only Published Producer Boundary

Rechecked 6 October via GitHub API:

| Producer | Published source | Current boundary |
| --- | --- | --- |
| [Tracker114](https://github.com/FerrPOINT/task-tracker/pull/114) | `8c80a41fae3bf1c10439ddb7e536b05bf320340d`, open, base main, conflicting | Owner questions/answers/revisions/confirmation and metadata outbox are published source. Analysis/routing/reservation extensions in other local checkout are not published compatibility evidence. |
| [Workflow90](https://github.com/FerrPOINT/project-workflow/pull/90) | `e4fba60f55aaefb2fa62cb2d6c151e075d7d5b37`, open, base master | PM bind/checkpoint/resume/rebind/readback are published source; minimal handshake finalizes ordinary bind after Fleet dispatch. It cannot authorize the first model POST. |

Tracker114 copies answer text/comment exactly and treats selected options as a set;
the gateway receipt comparisons follow that source behavior. UI sends only legacy
confirmation content_hash/idempotency_key. It does not select a routing policy.
The existing integration contract snapshot differs from three published Tracker
schemas; passing its local verifier must not be presented as deployed parity.
Neither open PR, source capability nor runtime health supplies installed admission.

## Required Runtime/Producer Handoff

Task chat send/steer remains fail-closed. To finish the PM vertical slice, producer
owners and the runtime task must supply the following exact, persisted authorities:

1. **Predispatch admission**: Tracker current owner CAS, reserved execution ordinal,
   immutable input snapshot/hash, assignment/execution/agent/version fence and lease;
   Fleet effective config/chat/workspace receipts; non-circular Workflow execution
   claim and native first-step/catalog evidence; live Base delegated credential.
   Workflow90's already-running callback cannot replace this admission. A short
   lease alone must not be claimed and left stranded by unsupported dispatch.
2. **Structured PM tools**: trusted runtime run context plus delegated credential,
   never browser-selected author/assignment/endpoints. Publish questions/revisions
   only under the current assigned fence. Preserve initial binding while native
   acknowledgement/readback is unresolved. Do not parse prose into tool commands.
3. **Delivery/resume journal**: durable original answer event/request/checkpoint/
   question version/requirements revision/fence; saved, queued, delivered and run
   started are separate states. Before POST `/internal/runtime/v1/pm/resume`, old
   Fleet session-run UUID must be confirmed terminal/safely stopped by trusted
   native readback. Reserve one new UUID and original resume operation key; rebind
   that exact run once. Late answers cannot target a later run or replacement fence.
4. **Workflow presentation source**: persisted PMIdentity's task, execution_ref,
   tracker_instance_ref/project_ref, task_ref/root_ref, concrete agent_ref,
   assignment_operation_key/ref/revision; authenticated Workflow90 POST
   `/internal/runtime/v1/pm/readback` with that immutable identity and optional
   original operation_key. Browser input cannot synthesize these refs. Required
   projection: contract_version, state/version/fence, session_run_id/binding_ref/
   hermes_run_ref, checkpoint's request/version/revision refs,
   resume_operation_key/session_run_id, terminal_readback, workflow_step_allowed,
   resume_delivered and original operation receipt. Strip execution_token. Fetch
   authorized step/history using its actual execution token/run identity; do not
   manufacture step rows from Tracker stage or message timestamps.
5. **Stream boundary**: pinned Base SSE offers onOpen/onEvent/onCursor but no
   disconnect/error callback. Expired-cursor reset/snapshot, full native missed
   tool history and explicit transport state require their owning contracts.
   This packet's fresh snapshot on reconnect does not implement those producers.

The consumer cannot display verified Workflow steps/checkpoints/rebind or PM
answer-delivery progress until those records exist. A question's checkpoint UUID
is a request reference, not proof of Workflow checkpoint acceptance. Current UI
keeps absence explicit and does not fabricate receipts or stage completion.

## Release Order And Remaining Acceptance

This packet adds no migration or dependency pin. Publish only its commits on the
independent branch. Do not open a giant main PR containing the integration branch's
historical runtime migrations. Integrate after the runtime owner's review, preserving
Fleet 000010/11 ->12 ->13 ->14 ->15 ->16 and producer release order from existing plans;
Tracker/Workflow/Forge release heads and exact-head CI must be verified separately.
PR47, Base PR150 and unrelated release branches are unchanged.

Remaining: compatible released admission producers, real PM tools/initial dispatch,
durable delivery/checkpoint/resume/rebind orchestration, trusted prerequisite
verifier, authorized Workflow step projection, authenticated owner/operator
browser scenarios and live restart/denial/partial-success acceptance, then
seven-agent SDLC acceptance. The independently verified consumer fixes do not close
those gaps or permit automatic dispatch.
