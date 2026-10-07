# Chats / PM: API readiness and live acceptance

This frontend follow-up contains the production Chats fixes from
[`80e8b2c`](https://github.com/FerrPOINT/fleet-control/commit/80e8b2cac3c3cb63c3f5271a06c2b6eeff338156).
The separate PR branch starts at the published runtime integration
`d50c6947bdaf60dea73b096a71b1f09d69220392`; only frontend tests/UI and
documentation differ. At `c93a83f`, its entire frontend tree was identical
to `80e8b2c`; the current follow-up additionally fixes catalog-read freshness.
No runtime, Base controller, producer or OpenAPI changes are proposed here.

The 309 unit cases and 63 three-engine browser cases in the
[validation packet](assets/screens/chats-session-recovery-20261007/validation.json)
exercise fixture APIs. The packet's source hashes describe its original
consumer commit; the integration branch also preserves the parent's existing
handoff paragraph. These checks do not establish live PM acceptance.

The Workflow catalog follow-up holds the namespace selector and ordinary
binding rebind button while a cached catalog GET is pending or retrying after 503. The prior implementation checked only final query error state, so a
cached-success retry could still submit a stale selection. Two regressions
failed before the fix. A fresh successful GET restores the selected namespace
without an automatic PUT. This is ordinary agent/workflow binding;
it does not implement native PM checkpoint/rebind or a real step projection.
Its source hashes, unit/browser results and reviewed captures are in the
[catalog freshness packet](assets/screens/workflow-catalog-freshness-20261007/validation.json).

## Current producer source

Read-only inspection uses published [Tracker #114](https://github.com/FerrPOINT/task-tracker/pull/114)
at `8c80a41fae3bf1c10439ddb7e536b05bf320340d` and
[Workflow #90](https://github.com/FerrPOINT/project-workflow/pull/90)
at `e4fba60f55aaefb2fa62cb2d6c151e075d7d5b37`. Both PRs remain open.
Relevant source: Tracker
[routes](https://github.com/FerrPOINT/task-tracker/blob/8c80a41fae3bf1c10439ddb7e536b05bf320340d/backend/api/src/routes/sdlc.rs)
and [DTOs](https://github.com/FerrPOINT/task-tracker/blob/8c80a41fae3bf1c10439ddb7e536b05bf320340d/backend/domain/src/sdlc.rs);
Workflow [PM API](https://github.com/FerrPOINT/project-workflow/blob/e4fba60f55aaefb2fa62cb2d6c151e075d7d5b37/project_workflow/interfaces/ui/routes/pm_api.py),
[snapshot](https://github.com/FerrPOINT/project-workflow/blob/e4fba60f55aaefb2fa62cb2d6c151e075d7d5b37/project_workflow/application/pm_execution.py)
and [history](https://github.com/FerrPOINT/project-workflow/blob/e4fba60f55aaefb2fa62cb2d6c151e075d7d5b37/project_workflow/interfaces/ui/routes/runtime_api.py).

## Available endpoints and missing integration

In this table, Fleet public paths use `/api/v1/sessions/{session_id}` as `S`;
Tracker paths use `/api/v1/issues/{task_id}/sdlc` as `T`;
Workflow PM paths use `/internal/runtime/v1/pm` as `W`.

| Area / owner                                     | Published endpoint and fields                                                                                                                                                                                                                                               | Remaining live acceptance / contract                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Structured questions / PM runtime, Tracker       | Tracker `POST T/clarifications`; trusted `fence`, `request_id`, `question_id`, optional expected question version, `requirement_revision`, `checkpoint_id`, question content/options and original key. Fleet `GET S/clarifications` renders the actual question DTO.        | Bind the real PM tool's author/assignment to the trusted run. Publish once, then read the same request/question/version/checkpoint from Fleet. Reject stale fence or another assignment; preserve the original result after lost ACK. The UI does not manufacture a question or infer one from a message.                                                                                                                                                                 |
| Answer persistence / Fleet, Tracker              | Fleet `POST S/clarifications/{question_id}/answers` forwards `expected_question_version`, `requirement_revision`, `selected_option_ids`, `text`, `comment`, `idempotency_key` to Tracker `POST T/clarifications/{question_id}/answers`.                                     | A real owner answers the current version. Prove persisted exact receipt, 403 access loss, 409 version conflict and lost-ACK replay with the original payload/key. Saving is implemented; fixture success does not prove native PM delivery.                                                                                                                                                                                                                               |
| Answer delivery / Tracker messaging, PM runtime  | Tracker `GET T/events` exposes the durable event feed. Fleet inbox/gateway projection is part of the foundation.                                                                                                                                                            | Connect the native consumer and reconcile queued, delivered and consumed outcomes by event/operation identity, question version, revision, checkpoint and fence. Prove restart and duplicate delivery cannot start another continuation. No browser endpoint currently exposes a trusted native consumption receipt; define its sanitized DTO before showing delivery/continuation as complete.                                                                           |
| Requirements / PM runtime, Tracker               | Tracker `POST T/requirements` accepts trusted fence, optional `expected_requirement_revision`, document and original key; Fleet `GET S/requirements` reads published revisions.                                                                                             | Publish a real requirements revision after the delivered answer. Verify revision/hash and conflict behavior; do not treat a saved answer as a published revision.                                                                                                                                                                                                                                                                                                         |
| Exact confirmation / owner, Tracker verifier     | Fleet `POST S/requirements/{revision}/confirm`, Tracker `POST T/requirements/{revision}/confirm`; body `content_hash`, `idempotency_key`. The URL fixes the reviewed revision.                                                                                              | Fresh owner/project rights and actual business prerequisites must permit the transition. Prove current revision/hash, consent, original lost-ACK receipt and a real Backlog result. Backlog alone does not establish a Workflow step. Published schema parity still has the documented Analysis/routing-policy differences.                                                                                                                                               |
| Checkpoint / PM runtime, Workflow                | `POST W/checkpoint` binds immutable PM identity, operation key, version/fence, current session-run/binding/Hermes scope and `checkpoint_ref`, `clarification_request_ref`, `clarification_version`, `requirements_revision`.                                                | Establish a real accepted checkpoint from the trusted tool, not browser-selected refs. Reject stale credentials/fence and reconcile the original operation after lost ACK.                                                                                                                                                                                                                                                                                                |
| Resume / Fleet, Workflow runtime                 | Fleet `GET /internal/runtime/v1/pm/runs/{session_run_id}` supplies terminal old-run proof. Workflow `POST W/resume`, `POST W/rebind` and `POST W/readback` reserve/reconcile the successor and exact original operation.                                                    | Prove one reserved successor UUID, one native start, exact new binding and native receipt across restart/lost ACK. An unknown result cannot authorize another dispatch. This frontend PR adds no orchestration.                                                                                                                                                                                                                                                           |
| Real workflow steps / Fleet projection, Workflow | Workflow `POST W/readback` returns native state/version/fence, run/binding refs, checkpoint/resume information, `workflow_step_allowed`, `resume_delivered` and operation result. `GET /internal/runtime/history?task=...&n=...&session_run_id=...` returns native records. | Publish an authorized Fleet browser projection; no endpoint path/DTO exists in the current consumer contract. Resolve identity and runtime/execution credentials from server records, enforce fresh session/Tracker ACL and sanitize output. Supply a read-only attempt cursor: current snapshot/history serializers lack `attempt_number`. Render only actual phase/verdict/transitions/cycle/attempt records; never invent steps from Tracker stage or catalog entries. |

The immutable PM identity has ten fields: `task`, `execution_ref`,
`tracker_instance_ref`, `tracker_project_ref`, `task_ref`, `root_ref`,
`agent_ref`, `assignment_operation_key`, `assignment_ref`,
`assignment_revision`. Native Workflow reads require runtime-role credentials
and the current server-only `X-Workflow-Execution-Token`; these credentials
must not become browser fields. First dispatch additionally needs genuine
predispatch authority; the postdispatch PM bind is insufficient.

Exact Workflow command DTO fields from the published source:

- `PMCommand`: the ten identity fields, `operation_key`, `expected_version`,
  `expected_fence`, `binding_ref`, `hermes_run_ref`, `session_run_id`.
- `PMCheckpoint`: those fields plus `checkpoint_ref`,
  `clarification_request_ref`, `clarification_version`, `requirements_revision`.
- `PMResume`: the checkpoint fields plus `answer_event_ref`,
  `new_session_run_id`.
- `PMRebind`: the command fields plus `checkpoint_ref`, `resume_operation_key`,
  `new_binding_ref`, `new_hermes_run_ref`, `new_session_run_id`.
- `PMReadback`: the identity fields and optional `operation_key`; current state
  and a historical exact operation result must be distinguished.

## Combined live sequence

1. Create one real Draft/chat and recover the same identity after lost ACK;
   incomplete/admission-waiting states make zero model calls. PM Draft UI design
   remains pending as recorded in the [proposal](design/PM_DRAFT_CREATION_PREVIEW.md).
2. Prove genuine first-step admission and one native model dispatch. Publish
   real questions through the trusted PM tool and read their structured DTOs.
3. Save a real owner answer with conflict/access/lost-ACK checks. Trace its
   durable event through native consumption and one continuation.
4. Publish the resulting requirements and confirm the exact reviewed
   revision/hash; obtain a real prerequisite-checked Backlog receipt.
5. Accept a checkpoint, prove the old run terminal, then reserve/start/rebind
   one successor across restart/lost ACK. Read the native continuation result.
6. Read actual authorized Workflow history/projection in the browser and test
   denied/stale scopes. Re-run the combined browser gate, including the separate
   unresolved Base WebKit pending-fetch navigation case.

The existing Chats fixes are reviewable independently of this sequence.
`main` currently lacks their foundation; [Fleet #47](https://github.com/FerrPOINT/fleet-control/pull/47)
is still Draft and reports conflicts with `main`. A standalone PR containing
only this follow-up therefore needs either the integration branch as its
explicitly approved target or the foundation to land first. Neither choice
establishes live PM readiness or authorizes a new PM Draft design.
