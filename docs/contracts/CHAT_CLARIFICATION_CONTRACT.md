# Chat Clarification Contract v1

Status: agreed target contract; deployment compatibility must be verified.

## Runtime Scope Decision (2026-10-10)

Hermes is consumed unchanged through its existing `/v1/runs` API. A custom
Hermes pre-model authorization hook, reserved native-run handshake or producer
patch is NOT a requirement or release dependency. The product owner explicitly
rejected that extra mechanism. Earlier proposed contracts demanding it are
superseded by this decision, not implemented capabilities.

Fleet remains responsible for ordinary user/project/session authorization,
current task assignment and workflow state before dispatch, isolated agent
configuration, one active run and durable message/control idempotency. Those
are Fleet/Tracker/Workflow responsibilities, not a second authorization layer
inside Hermes. PM tools use supported runtime integration mechanisms; they do
not require modifications to Hermes internals. Existing missing Fleet wiring
must be implemented and tested; this decision does not turn held source paths
into working runs or waive clarification/confirmation business gates.

## Source Compatibility Evidence (2026-10-10)

All seven DTOs were rechecked against clean Tracker source
`357caa7a60a717eb7b0ac72f286b793326992931` using the authentic Fleet Rust schema
`e1b17e723abf43866c4f913c9fa4fba8b201bef5e3532b4a8f6cdc32ccbcce76` from
[codegen38044630680](https://github.com/FerrPOINT/fleet-control/actions/runs/38044630680).
Comparison and all eight verifier unit cases pass. This proves source wire
compatibility, not deployed authorization, delivery or execution acceptance.

The PR47 foundation reconciled with mainc8093aa retains Base875cac2 and
matches all seven DTOs in Tracker PR114 source
`357caa7a60a717eb7b0ac72f286b793326992931`. The comparison includes nested
closed schemas and validation constraints, not just documentation-free shape.
Positive versions are bounded by JavaScript's safe integer range; the optional
confirmation routing-policy version is omitted when absent. `Analysis` is a
valid context stage. The exported producer OpenAPI SHA256 is
`7a1131a653ad06898170318b25dea9f78760e07f4f58a0d91b081800efbf074e`.
This is exact source parity, not deployed compatibility or execution admission.
Earlier evidence below applies only to its older source packet.

## Release-Specific Compatibility Evidence (2026-10-06)

The PR47 foundation release, reconciled from5240107 with main3c6b8ef and pinned
SDKcbb4e99, matches all seven clarification/context/requirements/confirmation
wire schemas in actual Tracker PR114 OpenAPI at
`8c80a41fae3bf1c10439ddb7e536b05bf320340d`. The comparison reads exact Git
blobs, recursively resolves DTO references and verifies property types,
required fields, formats, enums and compositions against generated Fleet
schemas. It does not prove HTTP authorization, complete behavioral equivalence,
deployed compatibility, namespace execution authority or runnable admission.
The newer runtime integration branch must compare its extended schemas
independently. See the [verification ledger](../CHAT_CLARIFICATION_VERIFICATION.md).

## Identity And Ownership

Tracker instance + immutable issue ID identify a task; display key is not identity.
Fleet session binds one concrete agent, Tracker project/task/root IDs and central owner subject.
Unique instance/task/agent binding never merges transcripts. Existing unbound sessions stay free.

## Creation Recovery And Directory

Fleet creation is an opt-in owner/key saga, not runtime dispatch. An unknown
creation response is reconciled through project-scoped owner/key GET; only an
authoritative `404` means no operation. A saved operation continues via strict
`POST .../operations/{id}/continue {}` with original input and stable remote
keys; it does not accept edits or generate a second chat/run. Historical reads
remain available after rollout disablement, but continuation mutations do not.
Both require verified human identity and fresh explicit Tracker project access.

Project choices use Tracker contract v1 `/api/v1/sdlc/project-directory`,
with exact fields `contract_version`, `tracker_instance_id`, `projects`
(canonical ID/key/name only) and required nullable `next_cursor`. Fleet requests
the default 50-row UUID-keyset page and verifies instance, order, bounds and
cursor before applying its rollout allowlist. An empty filtered page can still
have a next cursor; it is not the end, a total or a readiness receipt.

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

Target contract: answer saved, resume queued, resume delivered and run started are separate states. Tracker
outbox and Fleet inbox use stable event IDs. Per-session durable event cursor is Fleet-owned.
Cross-service command replay uses canonical payload hash and persisted result.

Creation is a resumable Draft/binding/dispatch saga. PM starts only after links are saved.
Owner-only creation is wired to PM dispatch when `pm.dispatch.enabled` is true
and dispatch checks pass. Creation responses distinguish `awaiting_admission`,
`awaiting_runtime_acceptance` and `runtime_accepted`; none proves business completion.
The initial assignment adapter targets Workflow PR90 source
`163a4ace7e06a4770958122ae4c56a426db758ef`, using
`POST /internal/runtime/v1/pm/assign` and its closed `PMDraftAssignment`:
the ten execution identity fields plus original `owner_version`, `input_snapshot_ref`,
`input_sha256` and the probed `runtime_compatibility`. Role/mode are Workflow-owned;
later-stage work/queue/workspace/decomposition refs remain null. Assignment and bind
readback verify the exact Draft projection (`stage_key=draft`, `pm_draft_input`,
`lease_generation=1`); that generation is metadata, not proof of a live lease.
Assignment capabilities require exactly `assign`, `bind`, `resume`, `rebind`,
`readback`; the runtime credential retains `checkpoint`, `readback`. Old persisted
dispatch intents are not rewritten or resent with the new assignment payload.
This source alignment and its synthetic fixtures are not executable qualification
against deployed Workflow/Tracker/Hermes. No idle-owner prompt contract is added.
Fleet persists the request before calling Tracker,
derives stable per-operation command keys, and always reads authoritative Draft
and reservation operations before retrying writes. The original title/description
must match the immutable Tracker input snapshot/hash; mutable issue edits are not
input provenance. Reservation operation hash is canonical JSON
`{"operation":"reserve_pm_draft","payload":command}`. Every nullable readback
field must be present, UUID refs canonical, and historical reservation result
equal to the fresh current assignment/owner CAS. The atomic chat primitive creates
no prompt or run. Creation receipts do not bypass Workflow/Hermes admission.
Before any Tracker creation/reservation/chat continuation, Fleet now reads
`GET /api/pm/namespace-ownership/{namespace_id}` at its fixed configured Workflow
origin with a dedicated machine read PAT. It checks the closed version-1 DTO,
canonical namespace/UUIDs, exact Tracker instance/project, pinned authority issuer
and original provisioner. No cache, redirect, retry, proxy-env or human/catalog
credential fallback is allowed; the body is bounded to 16 KiB and five seconds.
The guard runs even on creation replay and never treats a previously successful
GET or the mapping's `created_at` as authority for execution. It does not persist
an admission receipt or dispatch a run. Owner-only progress GET remains read-only.

Before dispatch, Fleet revalidates Tracker reservation/owner, original input and
task context, namespace authority, concrete-agent configuration, runtime capabilities
and Workflow assignment. It persists run/dispatch custody before Hermes submission.
After acceptance readback pins the effective session, Fleet verifies Workflow
assignment bind and PM bind, obtains first-step instructions and sends ordinary
runtime guidance. Native first-step evidence is not a predispatch prerequisite;
no Hermes pre-model hook or reserved-run handshake is required.

Wait captures an execution checkpoint. Resume requires terminal readback of the old
run; a Stop ACK alone does not release capacity. Saved-answer delivery and continuation
confirmation remain separate. Continuation verifies the original checkpoint,
identity/fence and Workflow rebind. Unknown runtime acceptance requires readback;
no blind redispatch or EOF-as-success.

Fleet implements the machine-only Workflow callback
`GET /internal/runtime/v1/pm/runs/{session_run_id}`. The flat response is the
Workflow `RuntimeObservation` identity plus observation/binding/run/status,
dispatch key, checkpoint and fence. Fleet run UUIDs are globally unique;
Hermes run references are agent-local. A dedicated callback token is not an agent
credential. Reservations commit before dispatch, acknowledgement pins the
effective Hermes session ID, and every callback probes the actual runtime.
Callback, PM event/readback recovery and saved-answer continuation are wired in
source. Their implementation does not establish deployed compatibility or successful
end-to-end execution; executable qualification remains separate.

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

### Scoped Credential Issuance

The Fleet server-side `infra::pm_credentials` client implements the Base #126
delegation wire protocol. Its PM command derives exactly Tracker read/write and
`task-tracker:sdlc:pm:<task>:<assignment>:<execution>:<agent>:<version>` scopes
from the persisted assignment identity, matching Tracker's `PmAssignment.scope`.
Base's policy and Tracker's current assignment checks remain authoritative;
possession of this credential cannot substitute for ownership or workflow proof.

Before issuance, the coordinator must persist the command/key and parent
credential identity. An unknown response reuses that exact command under the
same parent, never a new operation key or a rotated parent. The HTTP client
does not retry automatically. Expired replay is rejected, not renewed implicitly.
Successful acknowledgement must include `no-store`, the exact scopes, a non-nil
token ID and a live bounded expiry. Its body is size-limited and redirects are
disabled. Neither issuer nor delegated credential debug output contains a secret;
the credential cannot be serialized. It authenticates only enumerated PM GET/POST
operations under the canonical assigned task's SDLC path at the configured
Tracker origin: context/questions/requirements and revision/diff reads, original
input/lease reads, question/revision publication, question cancellation
and lease claim/heartbeat. Legacy APIs, another task, arbitrary suffixes, owner
answers/confirmation, assignment/binding writes and verifier evidence are denied
before HTTP. The task restriction is derived server-side and is not an additional
field in the Base delegation request. The receiving gateway must disable redirects and
apply its own bounded response handling.

Client confinement is not a restriction on bearer possession. Tracker must also
reject PM assignment-scoped credentials on its legacy router and validate current
assignment authority for allowed task-scoped reads and writes. Broad service
read/write scopes do not confer legacy authority on the PM child. Runtime handoff
remains disabled until this server boundary is independently accepted.

The credential release connects this client to the owner creation coordinator
behind `pm.credentials.enabled=false` by default. Persisted issuance intent
precedes Base mutation; immutable acknowledgement stores only token ID, expiry
and scopes. Replays freshly introspect both credentials and verify the frozen
Tracker context. Exact command/parent identity pinning prevents a changed key,
parent or assignment from silently issuing a replacement child.

No runtime-tool handoff, lease claim, first-step authority, revocation workflow
or model dispatch is connected by this preparation. Actual Base/Tracker live
acceptance remains pending. No root PAT is placed in a runtime env or tool
argument. See the [candidate scope and gates](../plans/2026-10-09-pm-credentials-release.md).

## Interface

### Bounded Event Metadata

Tracker's opt-in event projection is `metadata_v1`, `contract_version=1`.
The opt-in authenticated poller requests a maximum 100 events and 262144
serialized bytes. A full requirements document is never an event resource.
The envelope contains required `after`, `next_after` canonical decimal strings,
`has_more`, and `events`. Each event contains decimal `sequence`, canonical
non-nil `event_id`/`task_id`, `event_type`, a stable nanosecond UTC `created_at`,
`metadata_sha256` and `payload`. Payload contains instance/project/root/owner,
stage, required nullable `current_requirement_revision`, and the typed resource:

- `task.created`: required nullable input snapshot reference/hash.
- `task.bound`: empty object.
- `pm.assigned`: assignment/execution/agent/version fence.
- `clarification.published` / `clarification.cancelled`: question/version,
  request/checkpoint, requirement revision, exact state and fence.
- `clarification.answered`: answer/question/version, request/checkpoint,
  requirement revision and the historical question's fence, never answer text.
- `requirements.published`: requirement revision and content hash.
- `requirements.evidence_recorded`: revision/content hash and hashed check ID.
- `requirements.confirmed`: confirmation reference, revision/content hash.

The digest is SHA256 of sorted-key compact UTF-8 JSON
`{"contract_version":1,"projection":"metadata_v1","event":<event without metadata_sha256>}`.
Arrays retain order and text is not normalized. This is a transport consistency
check, not a signature or substitute for authenticated Tracker authorization.
The page returns a contiguous task-event prefix, allowing global sequence gaps.
Unknown/corrupt events and an unrepresentable first event block progress;
clients never skip them or substitute an empty successful page.

Fleet verifies the exact bound identity and source digest before a transaction.
The transaction pins projection/version, including empty pages, and persists
only the safe summary, immutable receipt and durable invalidation. Legacy full
events retain their existing hashes and cannot share a metadata cursor without
an explicit migration. Persisted metadata does not mean PM delivery or business
completion. Actual producer-byte snapshots are verified independently. Live
authenticated cross-service polling remains a release acceptance prerequisite.

Polling is disabled by default. A dedicated server-only PAT must introspect at
the fixed Base origin as the configured canonical subject with exactly
`task-tracker:read`; write, wildcard and duplicate scopes are rejected. Every
cycle reads current Tracker project access, and every task fetch checks its ACL.
Only active Fleet owners' immutable bindings in that project set are scanned,
in keyset pages of 100 with at most two concurrent fetches. Each binding advances
at most one bounded source page per cycle. Errors never advance its source cursor
or skip an event. Persisted cursors, not an in-memory scan position, drive replay.
HTTP redirects/retries are disabled and bodies are bounded before decoding.
This worker never issues a PM credential, prompt, approval or business transition.

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
