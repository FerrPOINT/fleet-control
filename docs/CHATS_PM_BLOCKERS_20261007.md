# Chats / PM: remaining acceptance boundaries

The production consumer keeps an unknown answer or requirements confirmation
bound to its original human actor, service and idempotency key. A late 2xx after
logout, sign-out, another actor/service or failed fresh session authorization
cannot erase the answer draft or show confirmation as saved. Returning to the
original actor/service requires fresh reads and an explicit replay of the exact
original payload/key. The server still validates current session/project access
and the original receipt. These PM identities remain in component memory;
this change does not persist private answers or provide PM recovery after reload.
Sign-out also disposes the consumer stream immediately.

The [identity validation packet](assets/screens/chats-pm-identity-20261007/validation.json)
separates regression fixtures from the installed system. The earlier
[control reload packet](assets/screens/chats-control-reload-20261007/validation.json)
establishes tab-scoped digest-only stop/steer recovery. Its historical source
hashes remain unchanged.

## Installed system: read-only observations

The [live record](assets/screens/chats-pm-identity-20261007/live-readonly.json)
contains HTTP observations and exact installed image IDs. Existing normal SSO
authenticated the browser without entering new credentials. Both visible Fleet
agents had zero sessions for the current owner filter; Fleet `/workflows`
showed insufficient permissions. Workflow `/tasks` showed no tasks in the
current namespace. No task creation, permission change, PM dispatch or runtime
replacement was performed.

All four inspected image IDs and the Fleet public schema fingerprint match the
[previous installed-system record](assets/design/chats-pm-review/installed-readonly.json).
Fleet's public schema contains 67 paths and does not advertise the 12 required
PM/chat/control-lookup paths listed in the live record. Missing public Swagger
paths alone do **not** prove private capabilities absent. Tracker's probed schema
URL returned 404; Workflow's returned HTML, not a JSON schema. These observations
do not validate the candidate frontend or native PM execution.

## Required producer contracts and release evidence

The consumer branch now includes published runtime integration
[`03c26d2`](https://github.com/FerrPOINT/fleet-control/commit/03c26d20c51b1c5562295c0a6e5975a64da4bc01)
through an ordinary fast-forward merge. Earlier consumer commits `16b7516`,
`b466a67` and `94e889d` remain in its history. Runtime-owned recovery/stop
acceptance is documented in the integration verification ledger; this consumer
packet does not retest or extend it. Browser discovery of forgotten control
identity and native PM execution remain separate contracts.

The [current consumer release packet](CHATS_PM_RELEASE_PACKET_20261007.md)
records the new context-source fix, current producer heads, constraint-aware
schema differences, unchanged installed images and the conflicted main merge
preview. The earlier installed-system/browser observations above are historical.

| Boundary / owner                                                | Required evidence before the browser can claim completion                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Lost tab metadata / Fleet controls producer                     | An authorized read-only discovery projection across historical runs, with stable bounded pagination and original actor/session/run/agent/operation, native command ID/ACK and original key/canonical digest or an equivalent scoped reconciliation handle. Current-run list reads are limited to 100 entries and do not reconstruct forgotten identity. Fresh owner/service/project access remains mandatory; private guidance and credentials stay excluded. This is a required contract, not an existing new endpoint. |
| Draft creation / Fleet UI and runtime                           | Approve the existing isolated Draft preview before production form integration. Ordinary private-chat creation is not PM Draft creation. Recover create/continue by original operation identity; `incomplete` and `awaiting_admission` stay non-dispatching.                                                                                                                                                                                                                                                             |
| First PM model call / Tracker, Workflow, Base and Fleet runtime | Genuine predispatch owner/project authority, execution ordinal, immutable input, assignment ID/version/key, fence/lease, accepted configuration/chat/workspace receipts, Workflow claim/first phase and delegated credentials. Postdispatch binding to an already-created run cannot authorize the first call.                                                                                                                                                                                                           |
| Clarification / PM runtime, Tracker and messaging               | Actual structured publication plus durable answer delivery/consumption bound to request/question version, requirements revision, checkpoint/fence and original operation/event receipt. Saving an answer, delivering it and starting continuation are separate outcomes.                                                                                                                                                                                                                                                 |
| Requirements → Backlog / Tracker and native runtime             | Exact current revision/hash, original owner/key receipt and fresh business prerequisites. A Tracker `Backlog` ACK does not prove a Workflow step. Published `Analysis` stage and optional `expected_routing_policy_version` parity still require producer acceptance.                                                                                                                                                                                                                                                    |
| Checkpoint / resume / Fleet and Workflow runtime                | Accepted structured checkpoint, terminal old-run proof and current fence; reserve one successor run UUID using the original resume key, start once, rebind that exact run and reconcile native readback after restart/lost ACK. Unknown outcome must never cause another dispatch.                                                                                                                                                                                                                                       |
| Workflow presentation / Fleet gateway and Workflow              | Authorized browser projection from accepted immutable PM identity plus current run/binding/version/fence and server-resolved runtime credentials. Enforce fresh human/Tracker ACL and sanitize output. Native step/history/attempt cursor must come from the producer; catalog rows, Tracker stage and a question's checkpoint UUID cannot prove accepted execution.                                                                                                                                                     |
| Stream access loss / Base SDK                                   | Direct stream 401/403 status callback and cursor/restart semantics. Fresh session GET denial is covered here; transport-only denial and the separate WebKit pending-fetch navigation diagnostic remain open.                                                                                                                                                                                                                                                                                                             |
| Combined live release / runtime owner                           | Compatible installed producer/consumer heads, a real authorized PM task, native admission/publication and actual Workflow records. Then execute the [full live acceptance sequence](CHATS_PM_API_ACCEPTANCE_20261007.md). Candidate tests and unchanged installed images cannot substitute for it.                                                                                                                                                                                                                       |

The accepted ten-field PM identity is `task`, `execution_ref`,
`tracker_instance_ref`, `tracker_project_ref`, `task_ref`, `root_ref`, `agent_ref`,
`assignment_operation_key`, `assignment_ref`, `assignment_revision`.
Native reads are Workflow `POST /internal/runtime/v1/pm/readback` and
`GET /internal/runtime/history?task=...&n=...&session_run_id=...`, using runtime-role
credentials and server-only `X-Workflow-Execution-Token`. Old-run terminal proof
comes from Fleet `GET /internal/runtime/v1/pm/runs/{session_run_id}`.
Neither current native read serializer supplies `attempt_number`; do not infer it.

The [consumer handoff](CHATS_PM_FINAL_HANDOFF_20261006.md) describes those
published contracts and the six-step live sequence in detail. This follow-up
changes frontend behavior, regression tests and evidence only. It changes no
producer, SDK, OpenAPI, migration, credential storage or installed runtime.
