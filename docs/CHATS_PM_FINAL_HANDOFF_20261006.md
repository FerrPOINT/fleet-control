# Chats / PM: remaining integration and acceptance

Production Chats implements dialogue, structured answers and exact-revision
confirmation. Full live PM acceptance remains open. This inspection starts at
consumer `14982e15e9597e3bf7d6a9490381e0b99f7f120f`, already integrated in
published Fleet `abcf8224b47051bec1d9e48fab2781ffcdef4db0`. Fleet's frontend
and browser API snapshot are identical at those two commits. Base remains pinned
to `cbb4e99230420dc2659431b1c9fb5090e5c940f0`. Producer head checks on
7 October 2026 match the previously verified source blobs:
[Tracker114](https://github.com/FerrPOINT/task-tracker/pull/114)
at `8c80a41fae3bf1c10439ddb7e536b05bf320340d`,
[Workflow90](https://github.com/FerrPOINT/project-workflow/pull/90)
at `e4fba60f55aaefb2fa62cb2d6c151e075d7d5b37`; both remain open.

The earlier [answer/confirmation packet](assets/screens/chats-authority-20261006/validation.json)
holds commands during failed context/document GET retries. Integrated `717aaba` closes
the same cached-success window for prompt/steer controls and the active run's
command journal: pending or failed journal reads hold steer/stop, and a fresh
unresolved receipt keeps that hold. A successful current controls read is required
for message submission. Drafts and captured original retry payload/keys survive.
Four regressions failed before that fix; its source hashes, local checks
and fixture captures are in
[command freshness validation](assets/screens/chats-command-freshness-20261006/validation.json).

Integrated `14982e1` fixes a separate question-version display defect: the retained
selected answer previously used labels from the new version, or fell back to
an option UUID after replacement. Drafts now retain their original labels.
Explicit transfer still filters out removed option IDs and captures the reviewed
version's labels, so another version change preserves the correct intermediate
draft. No transfer or answer POST occurs automatically. Two regressions failed
before the fix; renamed/replaced options, repeated version changes, original
commands and captures are recorded in
[draft label validation](assets/screens/chats-draft-labels-20261007/validation.json).
The two new browser cases passed in all three engines. The full fixture run had
56 passes and one legacy Chromium `Page.captureScreenshot` protocol failure;
two focused repeats of that unchanged case passed. The record preserves both
the original failure and the rechecks.

The parent integration subsequently merged this consumer normally as
`d8a8c0b1263a087b8ef5e8c3a6c331997f3229a5`. Its fresh combined gate passes all66
selected cases across Chromium/Firefox/WebKit with zero retries/skips/flaky
results. Six draft-label captures match fresh Chromium images byte-for-byte;
292 unit cases and the frontend/document gates pass. This separate result does
not replace the earlier failed run or prove the live acceptance below. See
[parent evidence](CHAT_CLARIFICATION_VERIFICATION.md#integrated-clarification-draft-labels-7-october-2026).

The [PM Draft proposal](design/PM_DRAFT_CREATION_PREVIEW.md) was published in
`96bb6d0`, with a separate clickable entry, 11 component cases, 12 isolated browser
cases and 20 reviewed captures. Its previously sent design question remains
pending. Parallel frontend authorization is not explicit approval of that new
creation/recovery flow. Existing approved Chats fixes continue independently.

The current follow-up refreshes `GET /api/v1/sessions/{id}` on SSE open and
non-delta events. A cached successful session during a pending/failed GET no
longer grants command or approval authority. A definitive 401/403/404 session
denial closes the SSE connection and hides the workspace. The explicit access
check only repeats GET; a successful result restores the retained answer draft
and consent without submitting a command. Prompt, answer and exact confirmation
tests cover the cached-success retry window.

Known steer recovery now follows the original command ID on its original run,
including after the active run changes. A fresh
`GET /api/v1/sessions/{id}/runs/{run_id}/controls` must match command ID, session,
run, agent, actor and operation. Only `acknowledged`, or `terminal_observed`
with a nonempty native acknowledgement, releases the composer; no second POST
occurs. Wrong-scope and unconfirmed terminal receipts retain the hold. The
reading-position indicator now reacts to visible content changes, so reconnect
and a delta for an inactive run do not announce new messages.

The [session and receipt validation](assets/screens/chats-session-recovery-20261007/validation.json)
records 309 unit cases in 31 files, including 71 ChatDetail cases; the browser
results and reviewed captures are recorded with the exact source hashes. These
are existing-API fixture checks, separate from live PM acceptance.

The original-key recovery contract is now published; the SSE SDK boundary remains open:

| Owner                                       | Existing contract and required acceptance                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fleet runtime / public control API          | Published `f1891aade512a20f395d83c450999fb654a4f505` adds read-only `GET /api/v1/sessions/{session_id}/runs/{run_id}/controls/lookup`, one `Idempotency-Key` header and closed `operation` / `payload_sha256` query. The consumer retains the original run/key/input/actor/agent after a lost or invalid reply; it uses exact Rust trim and compact UTF-8 JSON digest semantics. Only fresh scoped ACK releases the hold; 404/409, unresolved/wrong-scope receipts and terminal state without ACK remain held. No second POST occurs. The later reload follow-up persists bounded metadata in sessionStorage before POST, without guidance or credentials. Actual document reload recovers by digest with fresh original actor/service access. Tab/OS restart or missing metadata still needs a producer discovery contract. See the current lookup packet below. |
| Base authenticated SSE SDK / Fleet consumer | The current SDK reports `onOpen` and events, but a direct stream HTTP 401/403 has no consumer status callback. The session GET denial path above is covered; transport-only denial remains open. Define an authorization/connection-status callback and cursor/restart semantics, then prove access revocation hides the workspace, stops reconnects, preserves drafts and only resumes after fresh session authorization. The separate pending-fetch WebKit navigation diagnostic remains open.                                                                                                                                                                                                                                                                                                                                                                  |

| Remaining contract / owner                                          | Concrete missing integration or fields                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| PM Draft entry point / Fleet UI                                     | `pm-drafts.ts` has create, original-key lookup, operation readback, continue and project-directory clients, but no production TSX caller. Ordinary private-chat creation does not create a PM Draft. Wire the approved creation/recovery flow to `operation_id`, project/agent/task/session IDs, `state`, `next_step`, `dispatch_allowed`; HTTP 202 `incomplete` / `awaiting_admission` remains non-dispatching.                                                                                                                                                                                                                                                                                                       |
| First-step admission / Tracker, Workflow, Base, Fleet runtime       | Non-circular predispatch proof must bind owner/project ACL, Tracker execution ordinal, immutable input, assignment ID/version/key, fence/lease, accepted configuration/chat/workspace receipts, Workflow claim and native first phase, plus delegated credentials. Postdispatch PM bind includes an already-created run and cannot authorize the first model POST. Genuine issuer identities must satisfy both producers' validators.                                                                                                                                                                                                                                                                                  |
| PM publication and answer delivery / PM runtime, Tracker, messaging | Structured publication must derive author/assignment/endpoint from trusted run authority. Durable answer delivery needs request/question version, requirements revision, checkpoint/fence and event/operation receipts with outbox consumption/reconciliation. Saved answer, queued event, delivered event and started continuation are distinct outcomes.                                                                                                                                                                                                                                                                                                                                                             |
| Exact confirmation / Tracker and native runtime                     | Retain expected revision/content hash, owner and idempotency receipt; refreshed permissions and business prerequisites gate confirmation. Tracker `Backlog` acknowledgement alone does not prove Workflow publication or a model step. Published parity remains open for Fleet's added `Analysis` stage enums and optional `expected_routing_policy_version`; local snapshot checks do not close that drift.                                                                                                                                                                                                                                                                                                           |
| Authorized projection / Fleet runtime and Workflow                  | Resolve immutable chat + accepted ten-field PM identity + current session-run/binding/Hermes refs/version/fence to protected runtime-role bearer and execution-token handles. Define persistence, rotation and stale/waiting/resume-pending scope semantics; enforce fresh session/operator/Tracker ACL and sanitize output. Workflow readback has checkpoint, resume refs, terminal readback, `workflow_step_allowed`, `resume_delivered` and exact operation result; native history has phase/verdict/transitions/cycle. Neither read serializer has `attempt_number`; a native read-only attempt cursor is needed. Browser-selected refs, catalog/PAT fallback and inferred step rows cannot supply this authority. |
| Checkpoint / resume / Fleet and Workflow runtime                    | Validate the accepted structured checkpoint, terminal old-run proof from Fleet runtime readback and current fence before reserving one new run UUID with the original resume key. Start once, rebind the exact new run and reconcile fresh native readback/operation receipt after lost acknowledgement or restart; an unknown result must not cause another dispatch.                                                                                                                                                                                                                                                                                                                                                 |

The exact PM identity is `task`, `execution_ref`, `tracker_instance_ref`,
`tracker_project_ref`, `task_ref`, `root_ref`, `agent_ref`,
`assignment_operation_key`, `assignment_ref`, `assignment_revision`.
Native reads are `POST /internal/runtime/v1/pm/readback` and
`GET /internal/runtime/history?task=...&n=...&session_run_id=...`, using the
current server-only `X-Workflow-Execution-Token` with runtime-role credentials.
Terminal proof comes from `GET /internal/runtime/v1/pm/runs/{session_run_id}`
on Fleet. Existing durable-field limitations are documented in the
[previous contract audit](CHATS_PM_CONSUMER_HANDOFF_20261006.md).

Acceptance sequence on the combined release head:

1. Owner creates one real Draft/chat; lose the create acknowledgement and recover
   by the original key. Restart and verify the same task/session/assignment,
   feature/rollout gates, denied operator/project access and zero model calls
   while creation is incomplete or awaiting admission.
2. Accept genuine predispatch authority and the first Workflow phase; prove one
   authorized first model dispatch and its native identity/binding/readback.
3. Publish real structured questions. Exercise explicit selection/text, stale
   question/version conflict and access loss; preserve drafts and block stale
   commands. Lose an answer acknowledgement and reconcile the original receipt.
4. Follow the durable answer event through queued/delivered/consumed receipts.
   Publish the resulting requirements separately; confirm the exact current
   revision/hash and verify owner/business gates and the original retry receipt.
5. Reach a real accepted checkpoint and terminal old run. Exercise restart/lost
   resume acknowledgement, one reserved successor run, exact rebind and native
   continuation receipt. Reject stale fence/assignment/credential scopes.
6. Read actual authorized Workflow projection/history from the browser; check
   owner/operator denials and supported stale/waiting/resume states. Re-run the
   combined browser gate, including the outstanding Base WebKit navigation case.

The earlier [original-key lookup packet](assets/screens/chats-control-key-lookup-20261007/validation.json) records 352 unit cases in 32 files, including 105 ChatDetail and seven API cases; all 72 fixture browser cases pass across Chromium, Firefox and WebKit. Twelve responsive captures cover held and acknowledged steer/stop. Initial malformed/incomplete 2xx replies, cached ACK during pending/failed GET and active-run changes retain the original identity and command hold. Stop ACK retains an unrelated composer draft and does not claim run termination.

The subsequent [tab reload packet](assets/screens/chats-control-reload-20261007/validation.json) proves actual document reload with original key/digest and actor/session/run/agent retained, without a second POST or persisted private text. Auth/service changes and failed or unconfirmed readback keep the hold; only fresh scoped native ACK removes metadata. Logout suspends authority in memory before central navigation and clears auth on pagehide, preserving the unknown command. Public events and the bounded current-run journal cannot reconstruct a missing original key/digest; tab destruction, OS restart and lost storage remain a producer discovery gap.

The reload follow-up passes 401 unit cases in 33 files and 78 browser cases
across three engines with zero retries; 12 responsive captures were reviewed.
Its ledger preserves the earlier 76-pass/two-failure full run and unfiltered
WebKit transport errors. Healthy reload tests now use long-lived HTTP SSE and
assert original-key/digest lookup; the producer/Base gaps remain open.

Fixture results prove the consumer regressions, not this live sequence. This
packet changes no API, producer, runtime, model, credential storage or migration.

The subsequent [PM identity follow-up](CHATS_PM_BLOCKERS_20261007.md) prevents a
late answer/confirmation ACK after logout or an actor/service/access change
from clearing the original draft or displaying a saved confirmation. It keeps
the original in-memory payload/key for explicit replay after fresh access returns.
The [identity packet](assets/screens/chats-pm-identity-20261007/validation.json)
records regression checks and the separate authenticated read-only live audit.
No real PM task was available; the native live acceptance sequence remains open.

The [7 October release packet](CHATS_PM_RELEASE_PACKET_20261007.md) preserves
those commits through the `54a4e14` and `03c26d2` integration fast-forwards and fixes unavailable
task context being displayed as absent requirements/assignment/binding. It
records current published producer constraints and eight main merge conflicts;
source/browser fixture evidence remains separate from native live acceptance.
