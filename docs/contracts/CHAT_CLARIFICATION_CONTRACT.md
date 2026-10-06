# Chat Clarification Contract v1

Status: agreed target contract; deployment compatibility must be verified.

## Identity And Ownership

### Producer Release Compatibility: 5 October 2026

Read-only remote recheck on6 October preserves the same exact heads below:
Tracker PR114 remains open/conflicting with main; Workflow PR90 remains open
on master. Neither publication is an installed predispatch-admission receipt.
The newer foundational Fleet PR47 has seven-schema parity with Tracker114;
the later integration extension below still does not. Do not confuse the two
Fleet source packets or enable routing from a foundational CI result.

Fresh fetched Tracker PR114 head `8c80a41fae3bf1c10439ddb7e536b05bf320340d`
does not contain the local unpublished Analysis/routing/reservation extension.
Exact Git-blob OpenAPI comparison fails three of the seven accepted Fleet
schemas: `TrackerTaskContext`, `TrackerConfirmation`, `ConfirmRequirementsRequest`.
Its confirmation command has only content_hash/idempotency_key and rejects
unknown fields; the optional routing opt-in below must not be sent to that build.
The existing legacy omission remains compatible behavior, not full-schema parity.
Do not overwrite the extension snapshot or claim deployment readiness from a
passing comparison against the divergent local Tracker checkout.

Workflow PR90 head `e4fba60f55aaefb2fa62cb2d6c151e075d7d5b37` likewise excludes
the local unpublished Base admission/binding modules. Its PM minimal handshake
still finalizes bind from an already-running Fleet callback. It is not
predispatch first-step authority. Runtime/config/file health cannot replace the
missing producer admission. Both repositories remain read-only here; actual
compatible release heads and live acceptance are required before opt-in.

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

Answer saved, resume queued, resume delivered and run started are separate states. Tracker
outbox and Fleet inbox use stable event IDs. Per-session durable event cursor is Fleet-owned.
Cross-service command replay uses canonical payload hash and persisted result.

Creation is a resumable Draft/binding/dispatch saga. PM starts only after links are saved.
The implemented owner-only creation slice stops at `awaiting_admission` with
`dispatch_allowed=false`. Fleet persists the request before calling Tracker,
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

Full predispatch admission still requires Tracker current owner CAS and execution
lease, Fleet actual effective config/chat/workspace receipts, Workflow execution
claim and catalog/native first-step evidence, plus Base scoped credentials.
Immutable namespace mapping is only one necessary prerequisite. Any future
prepare/admit protocol must revalidate these producer receipts under fencing and
recover unknown CAS outcomes through exact readback, without minting another
execution/ordinal. Existing Workflow PM bind verifies a running callback after
dispatch; it cannot be reused as a circular predispatch proof.
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

The free-chat adapter now atomically persists a verified 202 ACK before GET:
accepted run ID, prompt delivery and outbox result commit together. Pending
accepted runs can recover effective-session readback after restart through
authenticated GET only. The requested alias is replaced once by the actual
status document's session ID; concurrent identical pins have one stream-start
winner. Task-bound and PM execution records are excluded, so this recovery does
not bypass admission or authorize tools. An unknown run ID is never POSTed again.
The additive private request/key/origin/default-profile fingerprint/horizon
journal now reserves the concrete free-chat run atomically and consumes one
submission permit before HTTP. ACK and journal progress commit together.
Known-ID recovery requires its accepted journal and original context; legacy
records without that proof remain held without HTTP. Unknown acceptance remains
pending with an error and no repeated POST unless its original intent contains
the verified opt-in recovery extension facts described below. This is not task
admission. The unextended native baseline
has no non-dispatch HTTP key lookup/store epoch; durable=true and a retention
deadline cannot prove SQLite continuity. Recovery after the pin-to-worker
gap now uses original-context GET only for pinned active free chats. Validated
terminal run/prompt/optional assistant and durable events commit atomically;
empty output creates no synthetic response. Old stream writes cannot reopen the
terminal packet. Task/PM runs remain excluded. Authentic Fleet/native runtime
acceptance and full admission remain gates; see
[ADR 0018](../adr/0018-atomic-terminal-pinned-recovery.md).

The Base opt-in native extension and Fleet consumer now implement
[Recovery v1](HERMES_RECOVERY_V1.md). A witness commits in the original native
reservation transaction before inference, with immutable scope/key/fingerprint/
run and store incarnation. Before initial dispatch Fleet freezes the closed
capability facts and sends the original epoch header. Unknown acceptance may
perform a bounded non-dispatch lookup with those original facts/bytes/key and a
live DB-clock horizon; mapping acceptance rechecks the horizon atomically.
Only a positive exact witness restores the original ID. It still requires native
GET for effective session/terminal proof. Missing/conflict/reset/rotation/expiry
never authorize replacement, a new key or capacity release. Legacy intents are
not backfilled, and task/PM records remain excluded. Component/native gates have
passed; managed Fleet/native acceptance and explicit config rollout have not.
See [ADR 0019](../adr/0019-native-original-key-recovery.md) and
[current evidence](../CHAT_CLARIFICATION_VERIFICATION.md#original-key-non-dispatch-recovery-4-october-2026).

Prepared-intent restart delivery now has a separate bounded free-chat worker.
It verifies fresh health/protocol and immutable original context before the
existing atomic single-submission claim. Only an unconsumed prepared intent may
make its initial original POST; a prepared uncertain outbox changes only in that
same permit transaction. Submitted unknown acceptance is never reset or replayed.
This does not admit task/PM runs, renew a horizon or reconstruct prompts from
current state. Component tests are not managed native/Fleet acceptance. See
[ADR 0020](../adr/0020-prepared-dispatch-restart-recovery.md).

The independent native protocol gate now verifies the pinned API/AIAgent/SQLite
mechanism with a local model: actual dropped 202, concurrent same-key replay,
terminal and in-flight process crash, parsed SSE/status/transcript identity and
credential rotation. Rotation proves that a new credential can accept the old
key as a new run; consumer identity guards cannot be omitted. This does not
implement Fleet's unknown-acceptance journal or prove managed gateway lifecycle,
native tool/approval behavior, retention expiry, multiplex/compression or PM
admission. See the [native evidence](../CHAT_CLARIFICATION_VERIFICATION.md#native-hermes-protocol-acceptance-4-october-2026).

### Current PM Producer Boundary (4 October 2026)

Read-only source inspection: Tracker `af6ed1ee26f6d26534a0dd1526e3b4d168962160`
and Workflow `2d794612cea6dccb4f2806c664512a05b562fed1` expose different
prerequisites, not one runnable admission.
Tracker supplies task-scoped `/pm-draft-execution-lease` POST/GET and POST
`/pm-draft-execution-lease/heartbeat` under the assigned task's SDLC prefix.
The delegated PM child is permitted to use those operations and `/context`,
not owner reservation readback. Commands bind expected owner version, assignment/
execution/agent/version fence and stable idempotency key; heartbeat also binds
lease ID/version. Exact replay returns the original receipt, not a renewed TTL.
The receipt has TTL 30 seconds, heartbeat 10 seconds and `dispatch_allowed=false`.
An expired prior lease cannot currently be released/reacquired through this API.

Tracker still keeps this execution reserved and rejects PM business mutations;
the lease alone cannot promote it. Workflow's current PM bind requires an already
running Fleet observation and finalized binding, so it cannot authorize the first
model POST. Its PM enrollment accepts catalog v2, not the new v3 Base profile.
Neither namespace ownership nor the execution step route supplies a substitute
predispatch first-step receipt. Required producer work is an explicit Tracker
admission transition and non-circular Workflow predispatch authority for the exact
frozen Fleet config/chat/workspace and assignment fence. Producer owners implement
these in their own repositories; Fleet remains fail-closed until integration.

Do not automatically claim a short-lived PM lease during Draft creation while
those gates are unavailable: expiry would strand the reserved execution. Fleet
lease consumption/restart reconciliation is still implementation work, and any
positive lease test must remain distinct from model dispatch or SDLC completion.

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
disabled, as are environment proxy inheritance and automatic retries. The expiry
ceiling is receipt time + requested TTL + five seconds of clock skew: Base starts
TTL after blocking database locks, not at Fleet's request start. An already
expired acknowledgement or excessive remaining lifetime is still rejected.
Neither issuer nor delegated credential debug output contains a secret;
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

PM creation/continuation now optionally connects this client after the immutable
chat receipt commits. The deployment flag defaults to disabled. Before POST the
owner operation stores the exact command/hash/key/TTL, normalized Base/Tracker
origins, canonical machine subject and a SHA256 fingerprint of the original
high-entropy parent bearer. A changed parent, origin or TTL conflicts before HTTP.
Fresh parent introspection requires that subject and exactly Tracker read/write.
The ACK stores only child token ID, scopes and expiry; the secret stays in memory.
Child introspection and `GET .../sdlc/context` then verify exact assignment and
original human owner. That GET is permitted; PM children cannot read the
owner reservation operation or legacy/global APIs. Failed readback retains ACK;
recovery replays the original Base command and repeats fresh authorization.
Expired receipts are not implicitly renewed. Intent/ACK audit commits once,
without bearer, parent fingerprint or upstream body in audit/public responses.

Additive Fleet migration 000011 permits absent journal -> immutable intent ->
immutable ACK, preserving historical 000010 and old operation bytes. Down refuses
any retained journal. This is preparation, not lease claim or admission: creation
still returns `awaiting_admission`, `dispatch_allowed=false`. Scoped PG/HTTP
fixtures do not prove real Base-issued child interoperability with real Tracker.
Runtime tool handoff, durable revocation/recovery administration, subsequent-run
credential renewal policy and actual Base/Tracker acceptance remain release
prerequisites. No root PAT is placed in runtime env or tool arguments.

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
- `analysis.intent_created`: immutable Tracker intent, exact confirmed revision,
  initial Analyst/business routing and cycle/attempt zero. Queued is not running.
- `analysis.assignment_reserved`: assignment/execution/intent/snapshot/agent refs,
  canonical `SDLC-<ordinal>` workflow task reference, fence and assignment hash.
  This compact metadata is not the full assignment envelope or dispatch authority.

The Analysis extension requires a compatible Tracker/Fleet build before project
opt-in; older consumers reject unknown events instead of skipping them. The
source decoder supports `Analysis` context and validates new resource identities,
stage/revision/routing shape and hashes. Tracker instance identity is opaque and
must match the bound instance, not a local UUID. New event summaries explicitly
separate prepared assignment from runtime admission. Existing metadata cursors
and digests are unchanged; actual source fixtures remain distinct from live proof.

Exact-revision confirmation accepts optional `expected_routing_policy_version`:
an explicit positive JS-safe integer requests Tracker's frozen routing snapshot.
Omitted/null remains legacy serialization without that field and does not opt in
the task. Only Tracker's owner/project/revision gates authorize publication; a
routing snapshot alone never means native runtime readiness. No automatic version
selection or routing editor is introduced by this wire extension.

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
