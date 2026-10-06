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
Documentation/evidence commit `e4ee22227112a9512210a1c98ad037e02e0a854b`
and its append-only consumer review follow-up belong to the same independent
[branch](https://github.com/FerrPOINT/fleet-control/tree/feat/chats-pm-consumer-20261006).
Integrate the two original commits followed by `0ecee7e` and the HTTP 408
follow-up at this branch's tip. Published
history is preserved; do not merge the entire historical runtime tail into main.

## Краткая таблица готовности

| Область | Реализовано и проверено | Что требуется для настоящего PM |
| --- | --- | --- |
| Chats, история и доступ | Consumer UI; 262 unit-теста и 33 браузерных случая с fixture API. 13 Rust/PG/HTTP тестов проверяют gateway, binding, private history и отзыв доступа. | Реальная задача владельца и установленный совместимый Tracker; fixture-вход не подтверждает live-права. |
| Ответ владельца | Single/multiple/text без предвыбора; точный receipt. HTTP 408 удерживает формы; явный повтор сохраняет исходные payload/key, даже после сохранения ответа. | Tracker `POST /api/v1/issues/{id}/sdlc/clarifications/{question_id}/answers`, durable answer event/outbox и доверенный runtime consumer этого события. |
| Подтверждение требований | Согласие на конкретные revision/hash; проверка owner и Backlog receipt; после timeout повторяется исходная команда. | Tracker `POST /api/v1/issues/{id}/sdlc/requirements/{revision}/confirm`, настоящий prerequisite verifier и опубликованный контракт требуемого перехода. |
| Первый PM запуск | Task send/steer закрыты без полномочий; UI не подменяет admission. | Tracker owner CAS/reservation + Workflow predispatch claim/first-step + effective Fleet config и Base delegated credential. Опубликованный PM bind требует уже работающий run и не заменяет predispatch admission. |
| Checkpoint | Опубликованный контракт прочитан; реальная запись не подтверждена consumer-тестами. | Workflow `POST /internal/runtime/v1/pm/checkpoint`: immutable PMIdentity, operation key, version/fence, текущий run/binding/Hermes ref, checkpoint/request/version/revision; runtime bearer и текущий execution token. |
| Resume и неизвестный исход | Consumer сохраняет неопределённость команд; Workflow orchestration в этом пакете отсутствует. | Доверенный Fleet `GET /internal/runtime/v1/pm/runs/{session_run_id}` подтверждает terminal старого run; Workflow POST `pm/resume` резервирует один новый UUID, runtime запускает его один раз, POST `pm/rebind` связывает его; POST `pm/readback` сверяет исходный operation key. Нужен durable runtime journal. |
| Workflow на экране | Проверены опубликованные поля, cursor и identity; шаги не выдумываются из Tracker stage. | Авторизованная Fleet projection из Workflow `POST /internal/runtime/v1/pm/readback` и настоящих step/history. Identity и токен берутся из server records; execution token не передаётся браузеру. |

Браузерные случаи используют тестовый SSO/API. Отдельный чат Codex уже работает
параллельно с runtime-чатом; вход пользователя в Fleet не требуется для этой
доработки. Live PM acceptance остаётся отдельной интеграционной проверкой.

## Implemented

- Unknown answer outcomes hold every question and stale-draft transfer in the
  chat. A retry uses the captured original question/version/revision/payload/key,
  including when another question/version is displayed. Readback refreshes run
  before the explicit original-command retry is enabled. No automatic POST retry.
  A later 4xx denial of that retry does not erase the original uncertainty. Holds
  continue during pending retries and clear only on a matching acknowledgement.
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
  A non-200 successful context preflight also returns dependency uncertainty,
  rather than passing its context body through as an acknowledged command.
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
  Failed refresh of an empty cached question list shows the source error. A stale
  draft cannot transfer into an answered, read-only or unsuccessfully refreshed
  question. These edges and lost ACK -> denied retry -> exact successful receipt
  have seven additional consumer regressions. The same uncertainty guard protects
  a free-chat prompt after a denied retry; its original payload/key stays frozen
  until an acknowledged original message arrives. Unknown steer stays held.
- HTTP 408 is also an unknown outcome for answer, confirmation and free-chat
  prompt. Three stateful regressions reproduced unlocked forms before the fix.
  The browser fixture saves the answer/confirmation before returning 408, then
  accepts only the original explicit retry with identical payload/key; refreshed
  business permissions can already deny a new answer/confirmation at that point.

## Verification And Evidence

Source locations: [Chats](../frontend/src/pages/chat-detail/index.tsx),
[consumer regressions](../frontend/src/pages/chat-detail/index.test.tsx),
[gateway](../backend/api/src/routes/task_chats.rs),
[PostgreSQL/HTTP history test](../backend/infra/tests/sdlc_foundation.rs),
[browser scenarios](../frontend/e2e/fleet-control.spec.ts).

Node 22.20.0, pnpm 10.28.1, frozen lockfile and Base
`cbb4e99230420dc2659431b1c9fb5090e5c940f0` are used. Frontend typecheck, lint,
262 Vitest tests, build, format, OpenAPI generated-client/compatibility and seven
local chat-contract comparisons pass. The seven-schema local snapshot is not
proof of parity with published Tracker114; see the producer boundary below.
The build still reports the existing production chunk-size warning (>500 KiB).
The pinned installed Base UI checker and effective theme contrast pass. The
checker script's SHA256 matches the read-only pinned Base source. The normal
`pnpm ui:check` entry point requires Base's development dependencies (not installed
in this read-only dependency checkout), so the exact installed script was used:
`node node_modules/@sdlc/ui/scripts/check-ui-contract.mjs --repo .. --require-installed`.

Rust 1.88.0 `cargo fmt --all --check`, locked/offline workspace check with all
targets, API clippy with warnings denied, six task-chat API tests and seven
focused PostgreSQL/HTTP tests pass on the final backend bytes. Tests cover binding
immutability/concurrency, private history/idempotency, and read access after
assignment replacement with denial after project access revocation.
The HTTP regression also injects a valid context body with HTTP 202 and requires
503 for both structured read routes, exercising the shared command proxy guard.
[Sanitized backend validation](assets/design/chats-pm-consumer/backend-validation.json)
records commands, source hashes, test groups and exact Compose cleanup. The owned
QA project has no remaining containers or networks; installed runtime is unchanged.
Workspace Docker audit found no violations on the two reachable endpoints but
was incomplete because `sdlc1-runner` was unavailable; it is not a global audit pass.

Chromium/Firefox/WebKit pass 33 fixture browser cases across the full
`fleet-control.spec.ts` and `chats-directory.spec.ts`: PM questions,
exact confirmation, keyboard tabs/drawer/Escape, three viewports, history order,
legacy first prompt and held delivery after reload. The PM case also checks
effect-applied/lost-ACK HTTP 408 recovery for answer and confirmation. Screenshots for dialogue,
single/multiple/text (no preselection), requirements, confirmation and diff use
375x812, 1920x1080 and 2560x1440. Their source/hash manifest and preview files are
kept in [consumer evidence](assets/design/chats-pm-consumer/manifest.json).
These are production components with fixture API, **liveAcceptance=false**.
Historical fixture manifests also verify successfully (135 general screenshots,
nine chat-controller and three runtime-control images); these hash/dimension
checks are not fresh live captures. The new 21-image manifest is tied to the
current consumer source and the successful 33-case run.
[Final frontend validation](assets/design/chats-pm-consumer/frontend-validation.json)
records source/log hashes and exact commands. WebKit emits fixture teardown proxy
warnings against the absent mock upstream at `127.0.0.1:3456`; all 33 cases pass.
This is browser component acceptance and does not attest that upstream service.
The HTTP 408 follow-up runs on its own strict-port preview at
`http://localhost:24173`, stopped in the runner's finally block. Its preliminary
shared-preview disappearance and 127.0.0.1/localhost fixture SSO origin mismatch
are recorded separately from the final result. No runtime login is required for
these fixture tests.

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

Tracker114's four exact-head CI jobs and Workflow90's two jobs are successful.
Tracker114 remains conflicting with main; Workflow90 is mergeable. They are open
source candidates, not released/installed evidence. The
[wire comparison](assets/design/chats-pm-consumer/published-contract-review.json)
and [exact GitHub blob/SHA256 identities](assets/design/chats-pm-consumer/published-contract-sources.json)
record the current seven-schema audit. Four schemas match exactly. Three differ:

| Fleet schema/path | Fleet consumer snapshot | Published Tracker114 |
| --- | --- | --- |
| TrackerTaskContext.stage | Draft / Clarification / Backlog / Analysis | Draft / Clarification / Backlog |
| TrackerConfirmation.stage | Same extended enum | No Analysis |
| ConfirmRequirementsRequest.expected_routing_policy_version | Optional nullable int64 | Field absent |

These are additive Fleet extensions; they do not demonstrate a failure to decode
published legacy receipts. This UI sends only the legacy command; omitted/null
routing is skipped by Fleet's DTO serializer. The exact equality gate
`verify-chat-contract.mjs --tracker <published-openapi>` correctly rejects drift.
Do not overwrite the accepted snapshot to make it green or treat the local
seven-schema pass as published parity. Enabling Analysis/routing requires its
own compatible released producer contracts.

Tracker114 copies answer text/comment exactly and treats selected options as a set;
the gateway receipt comparisons follow that source behavior. UI sends only legacy
confirmation content_hash/idempotency_key. It does not select a routing policy.
The existing integration contract snapshot differs from three published Tracker
schemas; passing its local verifier must not be presented as deployed parity.
Neither open PR, source capability nor runtime health supplies installed admission.

The published Workflow implementation also accepts a broader PMIdentity than
Fleet's runtime validator. Workflow treats task and most refs as opaque bounded
strings; Fleet requires `SDLC-<positive canonical ordinal>` and canonical non-nil
UUIDs for execution_ref, tracker_project_ref, task_ref, root_ref, agent_ref and
assignment_ref. Workflow's inspected assign/bind paths accept supplied refs and
compare them to persisted assignment/namespace records; PM bind does not replace
them with generated opaque identifiers. This is conditional compatibility on a
shared subset, not proof that every published Workflow identity works in Fleet.
Its illustrative `PM-1`/`execution:one` values cannot be passed to Fleet as-is.
The server adapter must use genuine owner-issued refs satisfying both validators
and the namespace's task-key contract; browser normalization or invented UUIDs
would break identity. Runtime/producer owners must verify that provisioning.
[Identity review](assets/design/chats-pm-consumer/published-identity-review.json)
records the five exact published implementation blobs and Fleet validator hash.
No runtime validator is changed by this consumer packet.

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
   tracker_instance_ref, tracker_project_ref, task_ref, root_ref, concrete agent_ref,
   assignment_operation_key, assignment_ref, assignment_revision; authenticated Workflow90 POST
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

The historical published integration target observed for the `0ecee7e` review is
`8295fa8da84593d271a867c8cc692a2f4e03f77a`; it descends from this packet's baseline
and preserves the same Base pin. Its scoped overlap is `docs/CURRENT_STATE.md`:
both branches add introductions. Keep the runtime owner's new introductory sections
and this consumer introduction, preserving the shared historical body. Apply the
original code commit, original documentation commit, `0ecee7e`, then the HTTP 408 follow-up;
rerun integrated checks after resolving that documentation overlap. Use a temporary
Git index for the dry run; the actual integration checkout must remain untouched.
The [completed dry-run evidence](assets/design/chats-pm-consumer/integration-review.json)
records one initial documentation conflict and no remaining conflicts after this
resolution; all source changes apply. It does not claim an integrated build or
live acceptance. The exact target may advance during the runtime task, so repeat
the scoped application/check on its actual head before integration.
The runtime chat now owns that integration and its fresh checks; the earlier
dry-run artifact is preserved as historical evidence, not updated to claim its
current head or acceptance of this follow-up.

No independent-branch CI run exists: Fleet CI triggers only pushes/PRs for main.
This source packet has local consumer validation; merge readiness still requires
an appropriately scoped integration PR and successful exact-head CI after the
runtime owner applies it. Creating a main PR directly from this baseline would
include unrelated historical runtime work and is deliberately excluded.

Remaining: compatible released admission producers, real PM tools/initial dispatch,
durable delivery/checkpoint/resume/rebind orchestration, trusted prerequisite
verifier, authorized Workflow step projection, authenticated owner/operator
browser scenarios and live restart/denial/partial-success acceptance, then
seven-agent SDLC acceptance. The independently verified consumer fixes do not close
those gaps or permit automatic dispatch.
