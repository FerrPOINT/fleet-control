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
