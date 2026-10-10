# API

Managed request observation adds authenticated read-only
`GET /api/v1/sessions/{session_id}/runs/{run_id}/request-observation` for the
original accepted free-chat run. Owner/read-all access and fresh original native
custody are required; task-bound sessions remain503. Handler results are no-store
and digest-only, never dispatch/admission. Existing config PUT explicitly opts
new revisions into the pinned observer or removes it with enabled=false.
See [consumer contract](contracts/MANAGED_REQUEST_OBSERVER_V1.md). OpenAPI and
generated TypeScript include the route; the frozen Linux compilation and actual
Rust OpenAPI parity gate pass. The actual managed observer core scenario also
passes; interrupted observer activation/Fleet-death, full readiness and current
release-head CI remain open, not implied by these checks or earlier CI.
Opt-in PM creation now prepares the current Tracker lease and the actual
Workflow Draft assignment through internal owner HTTP contracts. Workflow intent
is stored before POST; explicit replay keeps its original command/source/token
fingerprint. The public creation response remains awaiting admission with dispatch
disabled until native supervisor integration is complete.

The Fleet-owned Hermes plugin registers six scoped PM tools and an LLM execution
gate. Its native inventory endpoint reads the actual active run agent rather than
YAML declarations. Fleet admission/tool endpoints and live native acceptance are
still being integrated; the plugin is not enabled in the installed workspace.

Machine-only PM lease GET and claim POST are internal outbound Tracker operations,
not new Fleet/browser endpoints. They require the acknowledged delegated
credential and original reservation; claim persists its original intent first
and reconciles lost acknowledgements through keyed GET. Failure preserves
awaiting-admission rather than dispatch.
Fleet OpenAPI and public DTOs are unchanged. See
[contract](contracts/PM_EXECUTION_LEASE_READBACK_V1.md).

Original preparation recovery adds only the private Base control action
`reconcile_preparation`; its fields match `prepare` and require original retained
identity, not a browser-supplied receipt. Missing DB/native custody returns the
existing reconciliation failure; no public force-adopt/reset endpoint is added.
Fleet routes, DTOs and generated OpenAPI remain unchanged. See the
[private contract](contracts/CONTAINER_CONTROL_V1.md#original-preparation-readback).

Pinned Base package preparation seals native skill discovery in the existing
configuration snapshot; missing/edited policy fails existing validation and
effective readback. No endpoint or OpenAPI schema changes. Configuration health
still cannot grant SDLC admission. See [policy](RUNTIME.md#sdlc-skill-discovery-policy).

Signed-v3 configuration recovery is an internal, default-off reconciliation
path. It adds no public endpoint, DTO, OpenAPI route or force-release operation;
its rollback claim and secret-bearing journal must never be exposed through
runtime/settings responses. See [ADR0035](adr/0035-signed-configuration-recovery.md).

Controller recovery epochs are internal repository operations, not new public
agent/runtime actions. Requests, witness hashes and lease state are not browser
DTOs. Any outstanding recovery fences existing queue/permit/endpoint/lifecycle
effects with reconciliation errors. No public force-unlock, owner reset or
takeover API is enabled; OpenAPI remains unchanged by this storage candidate.
See [the private contract](contracts/CONTROLLER_RECOVERY_V1.md).
The000021 trusted supervisor entry and private delivery/outcome repository ports
are not public routes. Raw original command, native witness, private journal and
receipt body must never be exposed through agent DTOs, logs or browser streams.

Original container dotenv bytes/hash remain exclusively inside the private
creation intent. Existing public routes, DTOs and generated OpenAPI are unchanged;
environment drift produces existing reconciliation errors, not raw secret output.
No public log ingestion endpoint is enabled by this input-custody change.

## Container Endpoint Custody

There is no public endpoint setter. The internal runtime supervisor resolves the
original namespace through Base and seals its origin against its durable launch
generation/PID before container dispatch. Private `fleet_launch` facts and this
immutable record must match. Existing message routes and public DTOs are unchanged;
native free-chat origins remain localhost-only. Candidate acceptance is tracked
in [ADR 0030](adr/0030-generation-bound-container-endpoint.md).

The owner clarification/confirmation gateway accepts only exact-200 receipts
matching its original task/question/version/revision/content and owner. An
unrelated success or nil receipt identity returns dependency uncertainty (503),
never an acknowledgement. Read-only questions/revisions use immutable binding
and fresh project access after reassignment; writes retain current-agent checks.
Successful non-200 Tracker context preflight responses also become 503, so a
context body cannot be mistaken for an answer or confirmation receipt.
No DTO/path change: [consumer handoff](CHATS_PM_CONSUMER_HANDOFF_20261006.md).

Hermes container control is an operator-configured candidate behind existing
runtime action routes, not a new public controller API. No registration, policy,
Compose path, source pin or prepared-generation document is exposed publicly.
Named-volume mapping and mapping-file/hash provenance are likewise private:
the backend resolves them from its explicit controller, never from a browser
path/volume selection. Boundary policy3 does not add a public DTO or route;
OpenAPI is unchanged. A stale/missing mapping returns existing reconciliation
errors and never silently adopts a replacement or retries an uncertain effect.
For an original container launch, runtime `pid` is the Docker-daemon init PID
from its durable ACK, not a process Fleet may signal on the backend host.
Start/health require original running namespace evidence and Hermes readiness;
stop requires original namespace exit. Unknown start fails closed without retry
or native fallback. Automatic generation preparation is internal to start
when private provisioning config is present: saved original intent, Base create/
register receipt, then DB binding before process start. Additive000018 now commits
a private pre-create DB fence before the intent file and Base prepare; lost
controller files return reconciliation, not permission to create a replacement.
No new public endpoint or DTO is added. No public field selects
an image, Docker policy, process credential or operation key. Docker config
activation remains unavailable. Restart prepares another generation only after
original namespace exit and closed DB history; an unknown launch cannot advance
its preparation key. This is not controller takeover or installed acceptance; see
[Container Control](contracts/CONTAINER_CONTROL_V1.md).

Managed health observation by a non-owning controller can return degraded
without changing shared agent/runtime/launch state. The health audit records
both observed `status` and fresh `persisted_status`; a nonpersisted observation
does not enter the HTTP health-transition alert path. Public response schemas
and generated OpenAPI remain unchanged by this custody/liveness fix.

Runtime actions now use the internal [pre-spawn launch journal](contracts/RUNTIME_LAUNCH_JOURNAL_V1.md).
There is no public launch-claim/ACK/reset endpoint or new API schema. Unresolved
original launches can return existing unavailable/conflict responses instead of
starting a replacement or reporting a successful stop/config activation. Backend
ownership is authoritative; HTTP health does not resolve an unknown generation.

Additive000016 introduces private original approval reservation/claim/readback/
completion. No Fleet route or OpenAPI schema is added. The existing exact-request
decision endpoint selects original mode when the default-off control-outcome flag
is enabled. It sends saved action bytes once and leaves transport/DB uncertainty
held for original-context GET recovery. A replay reads the existing decision;
failed preparation never falls back to legacy POST. Legacy decisions cannot gain
original context retrospectively. Public DTOs exclude context, action bytes,
mode/claim flags and credential/store fingerprints. Witnessed delivery is not
tool completion or a business-stage receipt.

The internal [control outcome journal](contracts/HERMES_CONTROL_OUTCOME_V1.md)
adds no Fleet HTTP route or OpenAPI DTO. The default-false supervisor consumer
connects this journal to exact-byte POST and original-context GET recovery;
public receipts never expose the
private original context. A `terminal_observed` receipt can independently
carry a witnessed acknowledgement without changing the terminal observation.
`accepted` reflects that known ACK, never task completion or safe process stop.
Unknown HTTP/DB ACK remains submitted/uncertain. Same-key public replay reads
the authorized receipt and never posts again; a worker may later commit the
independent ACK fact. Capability rejection does not use a legacy fallback.

Hermes free-chat stop/steer now require current pinned identity, original accepted
journal/context, fresh exact capabilities and native status readback before POST.
Pending/terminal/stale/legacy context does not authorize control; task controls
remain fail-closed until admission is integrated. ACK requires bounded HTTP200
JSON and the original native run. A stop ACK means only interrupt requested;
steer ACK cannot reset a concurrent waiting/stopping/terminal state. Invalid or
unknown acceptance returns a durable receipt without retry or capacity release.
Preflight validation failures remain errors. See
[consumer profile](contracts/HERMES_RUN_CONTROL_V1.md).

## Durable Free-Chat Controls

Steer and stop require an authenticated `Idempotency-Key` header (valid reference,
1..128 characters, exactly one header value). The same actor/key, run, operation and normalized payload
replays the existing command; a changed payload/identity is `409`. Actor identity
comes from authentication, not JSON. Both mutations additionally require verified
human-session proof; a sessionless principal receives `403` before session/run
lookup, even with an admin role and a valid key. This does not introduce an agent
or assignment machine-control API. A single unresolved command holds each run.
Reservation and a single-use submitted permit precede the native POST; submitted
or uncertain commands cannot be sent again after restart, including with a new key.

`RuntimeRunControlResponse.command` contains a redacted `RuntimeControlReceipt`.
`accepted=true` means only a validated native acknowledgement persisted atomically
with audit/event and, for stop, nonterminal stopping state. A timeout, invalid ACK
or unknown outcome is `accepted=false`; do not display success or clear guidance.
An ACK-persistence failure can return an error while the journal remains submitted.

- `GET /sessions/{session_id}/runs/{run_id}/controls`: latest 100 receipts, newest first.
- `GET /sessions/{session_id}/runs/{run_id}/controls/{command_id}`: exact receipt.
- `GET /sessions/{session_id}/runs/{run_id}/controls/lookup?operation=steer|stop&payload_sha256=<hash>`:
  original-key readback when the POST reply/command ID was lost. The original
  `Idempotency-Key` stays in the header; actor is derived from the current verified
  human session. Only that actor's key is searched, even for an admin/operator.
  Current session/project authorization is mandatory. The indexed read is not
  restricted to the newest100 commands or to an active run.

Lookup hash is lowercase SHA256 of compact UTF8 JSON with sorted keys:
`{"input":<normalized-steer-text-or-null>,"operation":"steer-or-stop"}`.
Steer uses the existing Rust `str::trim` semantics before native dispatch; preserve
the captured semantic input, not a newly edited draft. Unicode is not ASCII-expanded
or otherwise normalized; stop input is `null`. Unknown query fields/operation,
missing/duplicate key and noncanonical digest are rejected. Same scope/key with
changed payload/operation returns409; no own scoped receipt returns404. Foreign
actor keys never return somebody else's receipt, even with read-all permission.

Lookup performs no native HTTP, dispatch/claim/reconciliation, journal mutation,
audit/event insertion or run-state update. It preserves uncertain/submitted/
reserved states and works for an original terminal run. A404 is not permission
to resend: the initial POST may still arrive or commit later. Release a consumer
hold only from an exact original receipt with validated native acknowledgement;
`terminal_observed` without ACK is not proof that guidance was accepted.

The required header is an explicit incompatible security migration for legacy
unkeyed controls, not an ordinary additive API change. See
[versioning and client migration](API_VERSIONING.md#explicit-runtime-security-migrations).

All receipt routes require current session/project read access and exact session/run identity.
Receipts expose IDs, actor, operation, state, ACK and timestamps, not command input,
key, native credential/context or upstream response. Durable `runtime_control_changed`
session events notify consumers to read the authorized receipt; events are not ACKs.
`terminal_observed` means an independently committed terminal mirror was found,
not that the control was accepted or that a task succeeded. Task-bound generic
controls remain blocked pending fenced admission; Java chat/control remains phase 2.

The internal Hermes stream consumer now enforces the
[bounded event profile](contracts/HERMES_EVENT_STREAM_V1.md). Every JSON data
event must name the original accepted run. Stream retirement retains capacity
and never grants retry, cancellation, config activation or task admission.
Public Fleet SSE routes/DTOs and generated OpenAPI remain unchanged; this does
not implement expired-cursor snapshots or missed native approval replay.

Prepared Hermes restart delivery is internal, not a public retry endpoint. Only
the original unconsumed journal permit can submit once; submitted/legacy/task
records cannot use it. Public OpenAPI/client and authorization do not change.
See [ADR 0020](adr/0020-prepared-dispatch-restart-recovery.md).

Hermes original-key recovery adds no public Fleet API route or DTO. Private
adapter [recovery v1](contracts/HERMES_RECOVERY_V1.md) freezes verified facts
before original POST and uses authenticated non-dispatch native lookup. A found
ID is not completion, readiness or a permission to dispatch task/PM work. Unknown
or legacy context stays held; browser payload cannot supply a lookup witness.

`AgentConfigurationSnapshot.renderer_version` is an additive, server-selected
response field: omitted legacy/1 or native Hermes renderer 2. New Hermes drafts
use 2; existing snapshots are not rewritten. Config-edit requests are unchanged.
Validation refuses unsupported versions and malformed v2 native platform objects.
V2 accepts API settings only under `platforms.api_server`; root and gateway API
aliases are rejected, including null. Gateway/platform maps, blocks and optional
extra fields must be objects. These restrictions do not reinterpret legacy v1.
Optional `extra` on direct gateway object sections also must be an object,
including dynamic plugin sections; unrelated scalar settings stay unchanged.
Derived listener/credentials are private rendered values, not public config JSON.

Runtime stop/restart can return `503` for untracked active processes or unconfirmed
kill/wait. No stopped state or cleared PID is then acknowledged. Success confirms
only tracked parent-process exit, not descendant quiescence or task completion.
Start also rejects untracked active replacement with `503`; public start/stop/
restart recheck drain after the supervisor lock and return `409` during activation.
These responses are generated from Rust operation annotations in OpenAPI.
Physical purge propagates stop failure before filesystem deletion or success
event/audit writes; `503` retains the folder, while drain returns `409`.

Hermes config activation now persists a protected local journal before runtime
and file changes and retains it on unknown outcomes. No new endpoint or response
field is added; backups, resolved secret bytes and paths are never API data.
The asynchronous result cannot promote effective revision until Linux directory
barriers for apply/rollback succeed. An unconfirmed filesystem outcome retains
drain and the private journal; accepting the activation request is not completion.
Public operator crash reconciliation remains unavailable. See
[operations](OPERATIONS.md#sdlc-foundation-recovery) before attempting recovery.

Hermes free-chat dispatch now requires the verified durable `/v1/runs` protocol
and a private pre-POST journal/single-send permit. There is no public journal
endpoint or new OpenAPI response field. An unknown ACK retains pending delivery
with an error and the agent's run capacity; it is not definitive rejection or
permission to retry. Accepted-but-unpinned recovery requires original origin and
credential/profile context; legacy records without it are not probed automatically.
History remains readable. No operator "retry unknown" API is available; see
[the data model](DATA_MODEL.md#hermes-dispatch-journal).

## Exact Runtime Approval Decisions

- `GET /api/v1/sessions/{session_id}/approvals` lists redacted requests visible to the owner or an operator/admin.
- `GET /api/v1/sessions/{session_id}/approvals/{approval_id}/decision` reads the durable decision; `404` means no command has been reserved.
- `POST /api/v1/sessions/{session_id}/approvals/{approval_id}/decision` accepts only `choice: once | deny` and a required `idempotency_key`. A verified human session is required; machine PATs cannot self-approve. Operator/admin approval is a runtime permission, never requirements confirmation.
- Reservation and audit commit before HTTP. An identical replay returns the same decision without another runtime call; a changed actor, key or choice returns `409`. Hermes must acknowledge the exact run/request/choice and exactly one resolution before `delivered` is stored.
- `uncertain` is not success or rejection. It survives process restart and blocks a new command for that request. Do not retry dispatch when the runtime outcome is unknown. Readback is safe; operator reconciliation still needs independent evidence.
- The legacy run-wide `/runs/{run_id}/approval` route returns `409`: broad `always`, session grants and `resolve_all` are not available in Fleet. Existing transcript routes remain supported.
- The runtime adapter also rejects the retired run-wide approval method; internal
  callers cannot bypass the exact-request decision flow.
- Exact decisions additionally require the original accepted free-chat dispatch
  journal, current agent/origin and original derived-credential fingerprint.
  Fresh authenticated capabilities must expose `run_approval_response`,
  `approval_events` and the exact POST endpoint. Native GET must match the pinned
  run/session and its current `waiting_for_approval` event/request ID before POST.
  Legacy unjournaled and task-bound decisions fail closed; PM reservation/context
  alone is not control admission. Historical receipt reads/replays remain available.
- Require exact HTTP200, JSON MIME, identity encoding and at most64KiB for the
  exact ACK. Reservation is already durable before preflight; a preflight failure
  conservatively retains `uncertain` without a POST, not a successful decision.
  Restoring capabilities or repeating the key cannot dispatch that command later.

These runtime requests are separate from Tracker clarification answers and exact-revision requirements confirmation.

The change from legacy run-wide `200` to unconditional `409` is an intentional
security-breaking migration, not backwards-compatible approval behavior. Clients
must list the exact requests and submit an idempotent decision for one request;
they must not fall back to the legacy route. The product compatibility wrapper
exempts only that former `200` response, checks the exact replacement refusal and
rejects any restored success response. Base still checks the route, request,
other responses, component schemas and every other API operation. Regression
tests prove neighboring removals and changes still fail. This exception disappears
once the baseline includes the retired route; the security guard still rejects
restored numeric/wildcard success responses and a missing or changed refusal.
It is not a general drift bypass.

## October SDLC Foundation

### Pinned Base Package Draft

`POST /api/v1/agents/{agent_id}/config/base-package` is an operator/admin-only
operation with no client-controlled source path or revision. Fleet reads regular
Git blobs at Base commit `4b9b4c9297a13fb28a6ba2039af2f7cb719f2f58` from the
configured operator-owned cache. It verifies schema, all seven roles, fourteen
skills, hashes and exact inventory before preparing the concrete agent's allowlist
and SOUL in a new configuration draft. Other enabled skills are disabled in that
draft; installed files, active runs and the effective revision are unchanged.

Before preparing the draft, Fleet also reads the persisted Workflow v3 binding
with a dedicated Base service-read PAT. Numeric namespace/workflow IDs stay
separate from namespace names, workflow keys and declared profiles; the mapping
is frozen in the configuration. An absent/denied/unavailable owner readback is
`503`; mismatched persisted IDs or a changed mapping is `409`. A declared profile
that differs from the Base package is `422`. No legacy catalog-token fallback
or automatic candidate installation is permitted. See the
[binding contract](contracts/SDLC_WORKFLOW_BINDING_V1.md).

The response is `AgentConfigRevision`, not installation/readiness evidence.
Missing cache, invalid package, non-Hermes runtime or mismatched role/namespace
fails with `422`. A changed desired revision, concurrent identity change or drain
returns `409`; unauthorized users receive `403`. No private source text or Git
stderr is included in errors or audit. Audit contains the revision number only.
The protected draft response contains its SOUL/skills just like existing revisions.

Validation and activation of a package-marked revision recheck the immutable
snapshot against the pinned Git source: a client-editable proof field alone is
not trusted. Fresh Workflow readback is required for validation, activation,
supervisor apply and package configuration observation; drift blocks the action.
Activation also checks role/namespace/workflow IDs under the agent row lock.
Existing legacy revisions without package metadata retain their behavior.
This endpoint does not attest physical/bundled skill inventory or enable dispatch.

`sdlc_role` is independent from `kind` and `product_role`. Configuration `PUT`
now saves a draft, not effective runtime files. New operator/admin routes:
`GET /agents/{id}/config/revisions`, `POST /agents/{id}/config/revisions/{revision}/validate`,
`POST .../activate`, and `GET /agents/{id}/readiness` (all under `/api/v1`).
History remains limited to the latest 100 revisions, but readiness reads the
effective head directly and validate/activate load the exact agent/revision.
An older non-desired revision may validate; activation still returns 409 unless
it is the current desired validated revision. A revision on another agent is 404.
Workflow rebind and actual role/namespace/workflow identity changes return 409
while configuration is draining, a runtime run is unresolved or accepted prompt
dispatch is pending/dispatching/uncertain. Unchanged identity fields do not turn
a metadata-only update into an identity mutation.
Human message requests cannot set agent authors/runtime IDs or non-prompt kinds.
Pending prompts use transactional outbox; acceptance-unknown dispatch is not retried.
Session SSE supports `Last-Event-ID` or `cursor`, and rechecks ownership/role/expiry.
Delta events contain a redacted `text` snapshot, not a fragment that could expose
a credential split across frames. See [current scope](SDLC_IMPLEMENTATION.md).
Runtime mirror completion now requires the exact accepted Hermes run and native
success flags; nested terminal names, partial results and invalid EOF status
responses leave the run unresolved. No public route or DTO shape changes for
this guard. A completed runtime mirror does not authorize a Tracker transition;
see the [Hermes adapter contract](contracts/HERMES_ADAPTER_CONTRACT.md).
Known pinned free-chat runs recover through original-context status GET without
resubmitting or opening a replacement SSE consumer. Terminal state, delivery,
optional assistant and durable invalidations commit together; exact replay does
not advance the cursor. Empty output is not replaced by a synthetic reply.
No public endpoint/schema changes are introduced; task/PM admission is unchanged.

Base path: `/api/v1`.

## PM Chat Gateway

- `POST /projects/{project_id}/pm-drafts`: opt-in verified human owner creation;
  accepts `agent_id`, `title`, `description`, `idempotency_key`. The operation is
  persisted before Tracker HTTP, recovers Draft/reservation through authoritative
  readback, and atomically creates a private task-bound PM chat. `202` means
  `awaiting_admission`, with `dispatch_allowed=false`, not an active PM run.
  Reuse the exact request/key after an interrupted response; changed payload is
  `409`. Human credentials are request-local and never saved for unattended retries.
  Each POST, including replay, first verifies the concrete agent's namespace via
  fresh Workflow ownership readback with a separate server-only machine PAT.
  Foreign project mapping is `409`; unavailable/denied/invalid ownership is `503`.
  This is not full admission and does not enable dispatch. See [ENV](ENV.md).
- `GET /pm-drafts/operations/{operation_id}`: owner-only, fresh human/project
  access; returns historical creation state, IDs and no original input or machine
  credentials. It is not current workflow or admission authority and never
  starts a run. Reading completed history remains possible with creation disabled.
- `GET /projects/{project_id}/pm-drafts/operation?idempotency_key=...`: recover
  the operation after an unknown creation response, when its UUID never reached
  the browser. The exact owner/key lookup uses fresh project authorization;
  another owner, project or Tracker instance cannot expose its metadata. Missing
  operation is `404`, invalid key is `422`, invalid query shape is `400`.
- `POST /pm-drafts/operations/{operation_id}/continue`: strict empty JSON object
  `{}` (arrays, null, fields and nonobjects are `422`); continue the persisted
  original operation without accepting replacement input, agent or command key.
  Rechecks human identity, owner, current project access, rollout and namespace.
  Returns `202` with existing incomplete/awaiting-admission state, never a run.
- `GET /pm-drafts/projects?after={canonicalUuid}`: verified human project choices
  from Tracker's strict `/api/v1/sdlc/project-directory`, with no legacy directory
  fallback. Returns `enabled`, `tracker_instance_id`, `projects` (ID/key/name)
  and required nullable `next_cursor`. Tracker page size is 50; Fleet filters
  the page by rollout allowlist but preserves the original cursor, including an
  empty filtered page. There is no total, readiness claim or default project.
  Disabled creation returns `enabled=false` and an empty directory without an
  upstream read. Creation and recovery still perform their own access checks.

- `POST /sessions/{id}/task-binding`: explicit owner binding to assigned concrete PM.
- `GET /sessions/{id}/task-context`: verified binding and Tracker context; unbound chat
  returns null context, dependency failure is not an empty successful SDLC response.
  Reads survive PM reassignment with fresh Tracker project access. If the current
  assignment is absent or belongs to another agent, the gateway returns the bound
  context with `can_answer=false` and `can_confirm=false`; write routes still check
  the current concrete agent. Tracker task-wide permission flags never authorize
  a command from a historical chat.
- `GET /sessions/{id}/chat-controls`: authoritative ownership/capability/dispatch gates.
- `GET /sessions/{id}/history?before={messageUuid}&limit=50`: latest-first pages, each
  page returned in server allocation order, independent of host timestamps; maximum
  100, UUID cursor scoped to session. Internal ordering is not an SSE replay cursor.
- `GET /sessions/{id}/clarifications`, `POST .../{questionId}/answers`.
- `GET /sessions/{id}/requirements`, `POST .../{revision}/confirm`.

Tracker context/confirmation may return `Analysis` as well as Draft,
Clarification and Backlog. Confirmation optionally forwards
`expected_routing_policy_version` (1..9007199254740991) for explicit routing
snapshot opt-in; omission/null preserves legacy wire and never enrolls a task.
The owner/project/exact-revision checks remain authoritative in Tracker.
Analysis intent/reservation metadata is read-only context, not permission to
send a prompt or bypass assignment admission. See
[chat contract](contracts/CHAT_CLARIFICATION_CONTRACT.md#bounded-event-metadata).

Answer and confirmation forward the verified original bearer to configured Tracker,
which revalidates human session, project membership and exact owner. Local legacy tokens
cannot authorize these commands. Operator read-all is not proxy consent. Payload conflict
and upstream status are retained; unknown network outcome requires same-key readback/replay.
Questions/revisions remain Tracker-owned JSON envelopes documented by the cross-service
contract, not a second Fleet database. OpenAPI generates the Fleet routes and response DTOs;
the gateway rejects malformed successful responses and versions unsafe for JavaScript.
`pnpm chat:contract` verifies seven wire shapes against the accepted Tracker v1 snapshot.
The separate PM Draft boundary checks exact captured Tracker reservation/readback
bytes, required nulls, canonical UUIDs and the canonical command envelope hash.
Use `node scripts/verify-chat-contract.mjs --tracker <tracker-openapi.json>` to check the
actual sibling build before rollout. Wire checks cover field names, required fields,
types/nullability, UUID/date formats, enums and recursive validation constraints,
including additional-property policy, numeric/string/collection bounds and reference
siblings. Only documentation annotations are ignored. External, missing and recursive
schema references fail closed; field names are never treated as documentation.
Recording a producer snapshot first verifies every compared Fleet DTO; an incompatible
producer cannot overwrite the accepted snapshot. Requirements revision responses reject
unknown fields and revisions outside `1..9007199254740991`. Other semantic gates have
separate backend tests. Source parity alone is not deployed compatibility, predispatch
admission, first Workflow step or permission to invoke a model.

Auth:

При настроенном `FLEET_CONTROL_AUTH__CENTRAL_JWKS_URI` UI использует Central
Auth Authorization Code + PKCE, а backend проверяет центральный Bearer token и
активность сессии. Локальные register/login/refresh и изменение роли закрыты;
ошибка Central Auth не переключает приложение на локальный пароль.

- `POST /auth/register`
- `POST /auth/login`
- `POST /auth/refresh`
- `POST /auth/logout`
- `GET /users/me`
- `GET /users/me/permissions`
- `GET /users`
- `PATCH /users/{user_id}/role`

В платформенном режиме access tokens выпускает Central Auth. Локальный профиль
создаётся строго по `sub`, каталог `/users` синхронизируется с Central Auth, а
личные API-токены управляются в Admin Panel. Local HMAC JWTs остаются только для
явного legacy-режима без центральной конфигурации.

`display_name` локального профиля обновляется из актуальной центральной
проверки JWT или личного токена. `sub`, локальный ID и исторические авторы
сохраняются; старое имя в JWT и email prefix не являются источником имени.
При отсутствии проверенного имени запрос отклоняется без создания/изменения
профиля. Повторная проверка неизменённого имени не обновляет `updated_at`.

RBAC:

В центральном режиме все активные люди получают одинаковые пользовательские
права Fleet Control; экраны назначения локальных ролей скрыты. Роли ниже
применяются только к legacy-режиму. Runtime/service credentials остаются
отдельной машинной границей.

- `admin`: all users, settings, RBAC, sessions and runtime actions.
- `operator`: agents, leaders, executors, runtime, config, skills, deployments,
  logs and all sessions.
- `user`: own sessions/messages and read-only agent directory.

Fleet:

- `GET /dashboard`
- `GET /agent-directory`
- `GET /agents`
- `POST /agents`
- `GET /agents/storage-review` returns a fleet-wide managed-folder retention
  review with total bytes, archived bytes, purge-ready agents and marker/path
  issues.
- `GET/PATCH/DELETE /agents/{agent_id}`
- `GET /agents/{agent_id}/storage` returns the managed folder storage report,
  marker status and purge eligibility.
- `POST /agents/{agent_id}/purge-files` physically removes the managed
  `agents_root/agentN` folder after archive, exact name confirmation and marker
  validation. The agent remains archived after file removal.
- `POST /agents/{agent_id}/provision`
- `POST /agents/{agent_id}/start`
- `POST /agents/{agent_id}/stop`
- `POST /agents/{agent_id}/restart`
- `POST /agents/{agent_id}/health`
- `GET/PUT /agents/{agent_id}/config`
- `GET /agents/{agent_id}/skills`
- `PUT /agents/{agent_id}/skills/{skill_name}`
- `GET /leaders`
- `GET/PUT /leaders/{leader_agent_id}/executors`
- `GET /executors`

Sessions and workflow:

Единые правила чатов и source-review расхождения описаны в [CHAT.md](CHAT.md).
Там отдельно отмечены текущий SSE wire format, synchronous dispatch, ограничения
agent authorship и различия central/legacy permissions; наличие route не
подтверждает выполнение всех целевых гарантий.

- `GET /sessions?agent_id={agent_id}&leader_agent_id={leader_id}&user_id={id1,id2}`
  lists sessions by primary agent, selected leader and user filter.
- Omitting `user_id` returns only the current user's sessions.
- `user_id=all` returns all users only for admin/operator; normal users are
  forbidden from expanding beyond themselves.
- `POST /sessions` creates a session owned by the authenticated user. Use
  `primary_agent_id`; legacy `agent_id` is still accepted.
- `POST /sessions` is idempotent by `idempotency_key`; replay returns the
  original session, while the same key with a different payload returns `409`.
- `GET /sessions/{session_id}`
- `GET/POST /sessions/{session_id}/messages`
- `POST /sessions/{session_id}/messages` is idempotent by request key and avoids
  duplicate runtime dispatch on replay.
- `GET /sessions/{session_id}/stream`
- `GET /sessions/{session_id}/participants`
- `PUT /sessions/{session_id}/leader`
- `POST /sessions/{session_id}/handoff`
- `POST /sessions/{session_id}/delegations`
- `GET /sessions/{session_id}/runs`
- `POST /sessions/{session_id}/runs/{run_id}/steer`
- `POST /sessions/{session_id}/runs/{run_id}/stop`
- `POST /sessions/{session_id}/runs/{run_id}/approval` is retired and returns
  `409` without dispatch; use the exact-request decision endpoints above.
- `GET /workflow-bindings`
- `GET /workflow-catalog` reads the live Project Workflow catalog via its
  read-only `/internal/runtime/catalog` bridge. Configure
  `FLEET_CONTROL_FLEET__PROJECT_WORKFLOW_URL` and
  `FLEET_CONTROL_FLEET__PROJECT_WORKFLOW_CATALOG_TOKEN` on Fleet; the latter
  must match Workflow's `PROJECT_WORKFLOW_FLEET_CATALOG_TOKEN` and is never
  returned to the browser. Background binding refresh uses the same bridge.
- `PUT /workflow-bindings/{agent_id}` requires an operator and an exact
  `{namespace_id, workflow_id}` pair from that live catalog. Fleet rejects a
  workflow outside the selected namespace, atomically updates the agent and its
  binding, then records an audit entry. Stale bindings are never retargeted by
  the background reconciler.

Session defaults:

- Direct executor chat: `leader_agent_id = null`, `visibility = private`.
- Direct leader chat: primary agent and selected leader are the same leader.
- Child executor session from a leader chat inherits `leader_agent_id` and gets
  `parent_session_id`.
- Backend validates that selected leaders manage the target executor through
  `leader_executors`.

Runtime:

- `GET /runtime-templates`
- Hermes chat dispatch uses the runtime adapter and `/v1/runs`; Fleet sends
  `session_id=fleet:{session_id}:{agent_id}` and stores the mirror transcript.
- `GET/POST /deployments/jobs`
- `GET /deployments/jobs/{job_id}`
- `POST /deployments/jobs/{job_id}/cancel`
- `GET /logs`
  returns persisted redacted process records. Internal writes acknowledge the
  exact inserted row, even if another stream has already written a newer row;
  public response fields and ordering are unchanged.
- `GET /events` as SSE
- `GET /events/recent`
- `GET /audit-log`

Settings:

- `GET /settings/runtime` — эффективные startup-пути, источники и команды.
- `GET /settings/ports` — эффективные startup-порты backend и агентов;
  frontend port остаётся справочным внутренним значением deployment.
- `GET /settings/integrations` — эффективное подключение Project Workflow из
  startup-конфигурации.
- `GET /settings/auth` — эффективная legacy auth/cookie policy; в платформенном
  режиме human auth принадлежит Central Auth.
- Legacy `PUT` для этих четырёх endpoint сохранён для совместимости как
  idempotent no-op: точное совпадение с effective snapshot возвращает `200`,
  а попытка изменить значение — `409 Conflict`. Runtime-конфигурация через
  эти legacy endpoint не изменяется.
- `GET /settings/managed` — применённый процессом управляемый snapshot и номер
  активной версии. До первого применения `active_version` равен `null`, а
  snapshot собирается из deployment/env-конфигурации.
- `POST /settings/managed/preview` — нормализовать и проверить полный snapshot,
  вернуть field-level diff и признак обязательного restart без записи в БД.
- `POST /settings/managed/apply` — атомарно создать новую активную версию.
  Требует `expected_active_version` и `confirm_restart=true`; после успешного
  ответа процесс завершается с кодом `75`, чтобы supervisor выполнил restart.
- `GET /settings/managed/versions?limit=20` — неизменяемая история версий.
- `POST /settings/managed/versions/{version}/rollback` — создать новую активную
  версию из выбранного snapshot; история не переписывается.

Managed snapshot включает runtime roots/commands, диапазон портов агентов,
несекретные CI/CD и Project Workflow integration settings, auth/cookie policy и
retention thresholds. Database URL, JWT/runtime/API tokens, backend bind port,
frontend port и host/container port mappings остаются deployment-owned. Apply и
rollback записываются в `audit_log` в одной PostgreSQL-транзакции с переключением
активной версии; stale `expected_active_version` получает `409 Conflict`.

- `POST /deployments/jobs/bulk` — bulk runtime updates/rollback (Phase 3): один job на агента из `agent_ids` (≤100), archived/unknown пропускаются и считаются в `skipped`; `rollback: true` допустим только для `runtime_update` (помечает jobs и добавляет `detail.rollback`).

`POST /deployments/jobs` также принимает отдельные продуктовые операции Service Pulse:

| `job_kind` | Обязательные поля | Результат |
|---|---|---|
| `product_deploy` | `environment: "demo"`, точный 40-символьный `commit_sha`, UUID `idempotency_key`, `title` | Forge deployment для commit из защищённого `main` |
| `product_rollback` | `environment: "demo"`, UUID успешного `previous_release_id`, UUID `idempotency_key`, `title` | Отдельный Forge rollback deployment |

Для этих видов `agent_id`, `runtime_kind` и произвольный `detail` не допускаются. Повтор с тем же ключом и тем же содержимым возвращает исходный job, изменение параметров даёт `409`. `detail` ответа содержит связанный Forge deployment/pipeline ID и `health_verified`; `completed` возможен только после успеха pipeline и самостоятельной HTTP-проверки Pulse API/UI. Ошибка Forge, отмена, 30-минутный таймаут или провал health завершают job как `failed` с `last_error`. Переходы и ключ идемпотентности хранятся в PostgreSQL и восстанавливаются после рестарта. Для локального стенда задаются `FLEET_CONTROL_FLEET__PULSE_HEALTH_URL` и `FLEET_CONTROL_FLEET__PULSE_UI_URL`.
- `POST /settings/retention/review` — запустить проход stale-folder review сейчас (operator, audited): возвращает `stale_agent_ids` archived-агентов старше `fleet.retention.stale_archived_days`, порог и время прохода

Управляемая версия накладывается на deployment/env baseline при следующем
старте процесса. Секреты и сетевое подключение контейнера из baseline всегда
сохраняются; пользователи и платформенный вход управляются Central Auth.

The frontend build regenerates TypeScript types from `openapi/openapi.json`.
The OpenAPI JSON is regenerated from Rust source before release. Native Windows
regeneration requires MSVC `link.exe`; WSL/Linux generation is supported.

## PM Runtime Readback

`GET /api/v1/agents/{agent_id}/readiness` remains operator/admin-only.
An active/effective database revision does not imply that its runtime files
are intact. Fresh read-only filesystem verification adds
`effective_configuration_readback_failed` when the snapshot, marker, isolated
workspace or skills cannot be verified. Underlying paths, resolved credentials
and file hashes are not exposed. `effective_revision` still reports the database
head, not a successful runtime observation. Workflow admission remains a separate
blocker; this endpoint cannot authorize PM dispatch.
For a pinned Base snapshot, readback also revalidates Git provenance and the
role allowlist, then scans HOME skills (maximum 4096 entries / 16 levels).
Only the pinned package's materialized `<name>/SKILL.md` files are allowed.
Unlisted/nested/case-aliased instructions, support/scripts/assets/templates,
hidden files, links, special files and missing allowed skills fail closed without
deleting them. Unix hard-linked canonical instructions are rejected as well.
Legacy non-package snapshots preserve the previous managed-only verification.
`.bundled_manifest` never supplies proof.
Project/external/plugin discovery and runtime-loaded settings are not attested. The separate
`runtime_skill_inventory_not_verified` blocker prevents treating intact managed
files as proof of complete skill inventory/native provenance.

### SDLC Configuration Observation

`GET /internal/runtime/v1/agents/{agent_id}/configuration` is an opt-in machine
read, outside browser/local admin authentication. Base freshly introspects a
dedicated PAT at the configured fixed origin. Its canonical subject must equal
the registered configuration reader, its scopes must be exactly
`fleet-control:read` (no wildcard, duplicates or write scope), and the concrete
agent UUID must be in Fleet's deployment-owned reader allowlist. This uses Base's
existing service read/write PAT issuance, not an unissuable compound grant.
Email is never identity authority.
No PAT is stored by this read, and no shadow user is created.

The version-1 observation exposes only agent/role, effective revision, pinned
public package proof and frozen Workflow mapping metadata, observation UUID/time, managed-file verification
and blockers. It never returns prompt/skill content, env values, filesystem paths
or credentials. The effective database head is re-read after verification;
concurrent change/drain returns `409`. Effective head and exact pinned revision
are read directly, independently of the latest-100 history window.
Missing package/effective revision also
returns `409`; invalid/disabled dependency configuration or failed file/provenance
checks return `503`; missing/invalid/revoked PAT `401`, wrong subject/scopes `403`.

`runtime_ready` is deliberately `false`: this is a read-only observation, not a
lease, config activation, assignment acceptance/ACK, admission token or run
receipt. Tracker must not dispatch using its observation UUID. Native effective
settings/inventory and the frozen assignment/workflow protocol remain gates.

`GET /internal/runtime/v1/pm/runs/{session_run_id}` is a machine-only callback
for Project Workflow, outside browser authentication. It requires the dedicated
`FLEET_CONTROL_PM__READBACK_TOKEN`; an unset/short/reused credential fails closed.
The response is the flat Workflow `RuntimeObservation`, without a Fleet envelope.
It contains immutable Tracker/assignment/execution identity, Fleet run UUID,
binding, dispatch key, fence, checkpoint and a fresh observation UUID/status.

Fleet probes the authenticated Hermes `/v1/runs/{runtime_run_id}` only at the
original origin sealed in its private reservation. New reservations require the
original launch/controller IDs and derived-credential fingerprint; no secret or
additional field is added to the public flat callback. Historical unbound records
remain readable, but cannot obtain runtime proof by guessing the current listener.
The supervisor verifies custody before/after HTTP and again after the observation
transaction acquires its blocking locks. This last check rejects a retained child
that exits while the transaction waits. An identical terminal replay performs a
fresh read but does not rewrite timestamps, visible run or session events.
Runtime mapping mismatch, unknown status, unreachable runtime and malformed or
oversized replies return `503`; contradiction of stored terminal proof returns
`409`. Cached Fleet run state, EOF and a human-provided status are not proof.
The endpoint does not create assignments, dispatch a prompt or resume Workflow.


## Fleet alerts (monitoring, Phase 3)

- `GET /api/v1/fleet-alerts?state=open|acknowledged|resolved`: Operator+;
  canonical persisted kinds are `agent_down`, `agent_recovered`,
  `agent_restart_loop`, and `heartbeat_stale`. Heartbeat warnings apply only to
  `running` agents with a valid, nonfuture health timestamp older than ten
  minutes. Acknowledgement does not resolve an incident. Concurrent creation
  uses an agent-row transaction lock and returns the existing open/acknowledged
  incident. A fresh heartbeat resolves that incident even without a status
  transition, while leaving unrelated down/loop alerts unchanged. Missing or
  future health timestamps and nonrunning statuses do not prove recovery.
  Explicit failed/stopped-to-running/ready health recovery resolves active
  down/loop/heartbeat incidents. Resolution and its redacted audit commit
  together. Legacy `agent_heartbeat_stale` is a display alias only, not a
  supported database kind.
- `POST /api/v1/fleet-alerts/{alert_id}/acknowledge` — Operator+; ack только для `open`-алертов; аудит `fleet_alert.acknowledge`.
- Переходы пишутся в `fleet_alerts` (миграция 6) из start/stop/health операций без блокировки ответа.

## Общая база

Подключение версий, границы контрактов и проверки описаны в [BASE_INTEGRATION](BASE_INTEGRATION.md).

## Current Native Approval Recovery

No new public route is added. Existing approval listing and durable session
events include an exact pending request restored by the background known-ID GET
worker. It uses fresh native capabilities and the original accepted free-chat
context; recovery never sends a prompt, decision or another SSE request. The
existing owner decision endpoint remains exact once/deny only. Resolved requests
are not reopened. Native status contains only the current waiting request;
historical replay and unknown decision outcome lookup remain unimplemented.
See [runtime recovery boundaries](RUNTIME.md#current-approval-snapshot-recovery).

## Opt-in native PM v1 owner coordination

`pm.workflow.enabled` uses the server-only Workflow assignment credential to
prepare an initial Draft from the actual Tracker reservation. The existing
Hermes dispatch journal and outbox submit the original intake once. Native
`/internal/runtime/v1/pm/agents/{agent_id}/admit` admits only the real accepted
run, effective configuration, physical package and original active lease.
The managed native credential identifies the agent; duplicate Authorization
headers, an unknown run or unsupported inventory are denied.

The fixed `/tools/{operation}` route exposes context, question, requirements,
workflow, skills and checkpoint. Tracker mutation fences and Workflow execution
fields are supplied by Fleet. The model cannot select their authority or obtain
service credentials. Workflow reports include an explicit observed phase/status
cursor; skills require the current phase allowlist and verified Base content.
A checkpoint takes only an operation key and structured question UUID; Fleet
reads its checkpoint/request/revision references from Tracker and journals the
exact command before forwarding it. A confirmed wait cooperatively interrupts
the original conversation. Native terminal readback additionally requires its
real finalizer, rather than treating `/stop` cancellation as executor completion.

Lease renewal retains the original lease UUID and uses its version as CAS.
Concurrent renewal and a lost HTTP reply reconcile the same derived operation
key. Expired, foreign or unavailable ownership stops the owned original runtime;
an unconfirmed stop is not terminal proof. Assignment or heartbeat alone never
authorizes a provider call. Human answer/resume delivery and final live PM
acceptance remain unfinished; this candidate must not be marked merge-ready.
