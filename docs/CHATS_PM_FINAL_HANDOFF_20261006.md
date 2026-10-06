# Chats / PM: remaining integration and acceptance

Production Chats implements dialogue, structured answers and exact-revision
confirmation. Full live PM acceptance remains open. This inspection starts at
consumer `bf9b05d2c5b213d1a8c8f9c4abac2d17c08196f5`; Base remains pinned to
`cbb4e99230420dc2659431b1c9fb5090e5c940f0`. Published sources were re-fetched
and verified against Git blobs: [Tracker114](https://github.com/FerrPOINT/task-tracker/pull/114)
at `8c80a41fae3bf1c10439ddb7e536b05bf320340d`,
[Workflow90](https://github.com/FerrPOINT/project-workflow/pull/90)
at `e4fba60f55aaefb2fa62cb2d6c151e075d7d5b37`; both remain open.

One consumer defect is fixed here: a failed context, question or requirements
refresh retained cached query success during automatic GET retries and left
answer/confirmation enabled. The controls now require successful fresh reads,
no fetch in progress and zero failed attempts. Draft/consent and readable data
remain retained; original uncertain commands keep their original payload/key.
The four regressions failed before the fix. Current checks and UI captures are in
[validation.json](assets/screens/chats-authority-20261006/validation.json).

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

Fixture results prove the consumer regressions, not this live sequence. This
packet changes no API, producer, runtime, model, credential storage or migration.
