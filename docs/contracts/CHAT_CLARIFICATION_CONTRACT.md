# Chat Clarification Contract v1

Status: agreed target contract; deployment compatibility must be verified.

## Identity And Ownership

Tracker instance + immutable issue ID identify a task; display key is not identity.
Fleet session binds one concrete agent, Tracker project/task/root IDs and central owner subject.
Unique instance/task/agent binding never merges transcripts. Existing unbound sessions stay free.

Tracker checks explicit project membership/ownership and central subject for owner commands,
without environment-driven admin bypass. Machine credentials are scoped to assignment, agent,
task and execution; human payload cannot choose machine authorship. Operators may inspect
authorized sessions but cannot answer or confirm as the owner.

## Question And Answer

A question has id, request_id, version, requirement_revision, requirement reference, task/root,
assignment/execution/checkpoint, text, rationale, required, mode (single/multiple/text), option
IDs/labels/consequences, optional recommendation, and state (open/answered/superseded/cancelled).

Answer command carries question_id, expected question version and requirements revision,
selected option IDs, optional comment/custom text and idempotency key. Text mode/custom selection
requires nonempty text. Single permits exactly one option, multiple at least one. Recommendation
is never a submitted default. Unknown/duplicate option IDs are rejected.

Only an open question at the current version can be answered. Identical key/payload returns the
same durable answer; changed payload returns 409. Author/time are derived server-side. Conflict
preserves client draft. Machine-issued questions are accepted only for current PM assignment.

## Requirements And Confirmation

Immutable revision contains goal, scope/exclusions, scenarios, acceptance criteria, constraints,
dependencies, assumptions and content hash. PM produces the final document after answers.
Closing questions is not publication. Confirmation targets exact revision/hash and requires
owner, current revision, no mandatory open questions and verified technical prerequisites.
Failure leaves document and answers intact and does not advance the task. Ordinary Tracker
updates/transitions cannot bypass this gate. Changed requirements need a new confirmation.

## Delivery And Resume

Answer saved, resume queued, resume delivered and run started are separate states. Tracker
outbox and Fleet inbox use stable event IDs. Per-session durable event cursor is Fleet-owned.
Cross-service command replay uses canonical payload hash and persisted result.

Creation is a resumable Draft/binding/dispatch saga. PM starts only after links are saved.
Wait captures execution checkpoint. Old run must be terminal or safely stopped before new run;
resume preserves execution, verifies Workflow rebind and rejects stale request/fencing/version.
Unknown runtime acceptance requires readback; no blind redispatch or EOF-as-success.

Fleet implements the machine-only Workflow callback
`GET /internal/runtime/v1/pm/runs/{session_run_id}`. The flat response is the
Workflow `RuntimeObservation` identity plus observation/binding/run/status,
dispatch key, checkpoint and fence. Fleet run UUIDs are globally unique;
Hermes run references are agent-local. A dedicated callback token is not an agent
credential. Reservations commit before dispatch, acknowledgement pins the
effective Hermes session ID, and every callback probes the actual runtime.
This callback is a prerequisite, not proof that the dispatch/resume saga is wired.

### Source-Checked Hermes Dispatch Prerequisites

The read-only Hermes baseline `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3`
implements durable run idempotency in
`gateway/platforms/api_server_run_idempotency.py` and
`gateway/platforms/api_server_runs.py`. This source review is not live acceptance.

- Admission uses `Idempotency-Key`. The advertised
  `features.runs_idempotency` includes `supported`, `durable` and
  `retention_seconds`. Supported alone is insufficient: the store can fall back
  to process memory, and retained terminal records can eventually be pruned.
- The key is scoped to runtime authentication/profile identity. Before dispatch,
  Fleet must persist the exact request body/hash, operation key, runtime identity
  fingerprint and recovery horizon. Credential/profile changes or an expired
  horizon prohibit automatic replay. Unknown acceptance must never get a new key.
- A `202` acknowledgement contains `run_id`, `status` and `replayed`, not the
  effective session ID. Fleet must read the authenticated run status to pin that
  effective session; it must not invent it from the requested alias. The pinned
  run/session mapping remains immutable even after runtime compression/rotation.
- In this baseline, the trusted run context's `HERMES_SESSION_KEY` is the raw
  `run_id` (`_RunLaunch.approval_session_key`), not a chat UUID. A PM plugin must
  bind tools through this context plus its runtime credential. Tool arguments
  cannot select owner, agent, assignment, execution, endpoints or credentials.
- The asynchronous run can begin before Fleet finishes acknowledgement/readback
  and Workflow bind. Pending binding is a typed retryable wait, not authorization
  to publish questions or a reason to start another run. The gateway must require
  the verified first workflow step before any business mutation.

These are dispatch implementation and live-test requirements, not claims that
Fleet's coordinator, external PM plugin or safe replay is already connected.

### PM Draft Provenance

Draft execution needs a PM-specific typed assignment contract. Tracker must own
the persisted execution ordinal, owner-issued assignment/CAS and exact immutable
input snapshot/hash. Fleet supplies real task-chat/runtime/config identities;
Workflow validates its actual project/catalog mapping and versioned PM bundle.
Chat UUIDs and display task keys cannot generate substitute execution identities.
Delivery queue, decomposition and CI deployment fields that genuinely do not
apply to PM Draft must use explicit typed absence in that variant, not fabricated
receipt strings. Required inputs and runtime workspace evidence remain real.
The general delivery assignment contract must not be weakened to admit Draft.

## Interface

Tracker: /api/v1/issues/{id}/sdlc context, questions/answers, requirements revisions/detail/diff,
confirmation; machine commands publish questions and revisions for an assigned PM only.
Fleet: /api/v1/sessions/{id}/task-context and protected clarification/requirements gateway.
Workflow: versioned bind/checkpoint/rebind/readback bound to task and execution identity.

Errors: unauthorized/forbidden, missing resource, stale version or key-payload conflict,
validation error, dependency unavailable and uncertain acceptance are distinct. Do not convert
unavailable data into an empty successful response. Secrets never appear in URLs or evidence.

## UI And Evidence

Dialogue/clarification/requirements tabs share task context, not agent transcripts. List is
server-scoped. Full document and diff precede exact confirmation. Drafts stay in memory and
survive tab switches; conflict must not silently overwrite them. Approvals, business questions
and infrastructure failures have separate actions. Preview evidence is not live acceptance.
