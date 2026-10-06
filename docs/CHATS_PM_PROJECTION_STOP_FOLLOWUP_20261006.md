# Chats: projection boundary and stop permission follow-up

Consumer baseline: `e33cb4824e0f07416958e0a0a21d83bffa760a9b`.
Base remains pinned to `cbb4e99230420dc2659431b1c9fb5090e5c940f0`.
This packet changes only the existing Chats controller, its tests and evidence.
It introduces no endpoint, execution authority, producer change, runtime change,
journal or migration. WebKit navigation reproduction remains with the Base owner.

## Rechecked published producer fields

Fresh GitHub reads on 6 October 2026 confirm that Workflow PR90 remains at
`e4fba60f55aaefb2fa62cb2d6c151e075d7d5b37` and Tracker PR114 remains at
`8c80a41fae3bf1c10439ddb7e536b05bf320340d`. Both are open. Four contract/source
artifacts and five Workflow implementation files were fetched again at those
exact commits and verified against their Git blob identities.

| Source                                               | Actual fields                                                                                                                                        | Safe consumer meaning                                                                                                                 |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Fleet `SessionTaskContext`                           | Nullable immutable `binding`, nullable Tracker `tracker`                                                                                             | Existing chat binding and the fresh authorized Tracker context                                                                        |
| Tracker `SdlcContext` / Fleet `TrackerTaskContext`   | `contract_version`, instance/project/task/root/owner, `stage`, nullable `requirement_revision`, `waiting_reason`, assignment and command permissions | Tracker business stage, document revision, waiting text and current assignment; no Workflow steps or native attempt cursor            |
| Tracker `PmAssignment` / Fleet `TrackerPmAssignment` | `assignment_id`, `execution_id`, `agent_id`, `version`, `machine_subject`                                                                            | Current Tracker assignment identity; assignment version is not an attempt number, step version or Workflow fence                      |
| Tracker questions                                    | Question/request IDs, assignment/execution/agent/version, `checkpoint_id`, question version and requirements revision                                | Actual published question references; `checkpoint_id` alone is not accepted Workflow checkpoint evidence                              |
| Fleet session runs/history/control receipts          | Actual Fleet run IDs, native run reference/status/error/model metadata, messages and control acknowledgements                                        | Fleet launches, transcript and saved command states; their count/order does not identify Workflow attempt numbers or completed phases |

Those source fields already support the existing production context and history.
There is no `steps`, `attempt_number`, phase history or accepted checkpoint object
in task-context. This packet does not fill those gaps with inferred rows.

Workflow has real records, but they are not present in that browser contract:

- PM `POST /internal/runtime/v1/pm/readback` returns its persisted identity,
  active/waiting/resume-pending state, version/fence, current session-run/binding/
  Hermes references, structured checkpoint, resume operation/run, terminal
  readback, `workflow_step_allowed`, `resume_delivered` and optional exact original
  operation result. Its execution token must remain server-only. No phase or
  attempt-number field is included in `PMExecutionSnapshot`.
- `GET /internal/runtime/history` returns actual `phase_code`, verdict, worker
  report, supervisor message, retryable flag, next/rollback phase, creation time,
  workflow/mode IDs/key and cycle number. PM-associated rows additionally carry
  execution ref, session-run ID, version/fence, binding/Hermes references and
  assignment ref/revision. This serializer does **not** expose `attempt_number`.
  It requires a PM runtime-role bearer, current matching execution token and exact
  current Fleet session-run UUID; assignment/catalog credentials cannot read it.
- The native assignment/bind response includes `attempt_number`, current phase
  and technical execution/workspace refs. These are responses to runtime assignment
  commands, not a read-only task-context endpoint. Calling assign, bind or step to
  populate a UI is outside this consumer scope. Returning an old assignment receipt
  as current attempt state would also require an explicit persisted readback contract.
- Fleet's internal `PmRuntimeObservation.checkpoint_ref` is the trusted callback
  wire shape for Workflow, not an owner-facing authorized Workflow readback. It
  cannot replace checkpoint acceptance or supply missing native history authority.

The exact missing producer/runtime contract remains a server resolver for the
authorized immutable chat's accepted PM identity/current run/fence and protected
credential handles for its runtime-role bearer and matching execution token.
It must preserve current binding/Hermes refs, define rotation and stale/waiting/
resume-pending read semantics, perform fresh session and Tracker project ACL
checks, and return sanitized actual records. A read-only native attempt cursor
is also needed if attempts are to be shown; neither the current history serializer
nor PM snapshot provides that number. The browser cannot choose those authority
refs, use catalog/PAT credentials as substitutes, or expose execution tokens.
See the [earlier durable-field audit](CHATS_PM_CONSUMER_HANDOFF_20261006.md)
for existing Fleet reservation fields and the missing protected handles.

## Concrete consumer defect fixed

Chats already blocked send/steer after a failed controls refresh, but its stop
button used cached `can_stop` and only checked whether another runtime command
was pending. After a successful read followed by HTTP/network failure, stopping
remained enabled. A cached true flag could also enable the button for another
local owner or without an active run ID. Backend authorization still remained
the final authority; the defect was the consumer permitting a stale command attempt.

The existing stop button and its click handler now require the current chat owner,
a successful controls query with no refresh in progress and zero failed read attempts,
`can_stop`, an explicit active run ID, no pending stop and no unresolved runtime
control. They preserve existing idempotency keys by run.
After a successful retry, the command uses the newly returned run ID; it cannot
stop the previous cached run. No new stop or dispatch mechanism is introduced.

Four unit regressions failed before their corresponding fix. They cover failed refresh and
recovery with a different run, another owner, missing run ID, and a read retry still
pending after its first failure. React Query retains cached success during its
503 retry backoff; this now holds stop immediately, before the final query error.
The existing unknown-control hold remains covered. The browser fixture drives the controller's
real controls refresh, returns 503 while retaining cached controls, captures the
disabled state at 375/1920/2560 pixels, and verifies one explicit stop addressed
to the refreshed run with its idempotency key. It uses fixture APIs only.

Fresh counts, source hashes, producer Git blobs and screenshots are recorded in
[validation.json](assets/screens/chats-stop-20261006/validation.json).
Previous packets remain historical. This consumer fix does not close the producer
projection gap or the earlier combined WebKit 35/36 acceptance finding.
