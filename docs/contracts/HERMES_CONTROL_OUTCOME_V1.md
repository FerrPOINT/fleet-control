# Hermes Control Outcome v1

Status: Base producer, stop/steer supervisor dispatch/GET recovery and Fleet
OS-death recovery verified. Original approval journal/sender/GET recovery implemented;
native once/deny/lost-ACK GET recovery and separate two-SIGKILL/three-Fleet-process
approval recovery verified. Combined extensions and installed rollout remain gates. This is not
task admission, safe OS stop, Workflow completion or a replacement for
[human control authorization](HERMES_RUN_CONTROL_V1.md).

## Producer

Explicit `fleet-hermes-controls` plugin uses the supported native platform hook
and middleware before router freeze. No second listener and no Hermes source
or SessionDB edits. Source revision is bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3;
loaded native API modules are hash-checked. Default-profile bearer only; room
grants, profile routes, session keys and empty/manual auth are denied.

The producer owns `HERMES_HOME/fleet_controls.db`: exclusive Unix0600 creation,
parent fsync, SQLite FULL synchronization, canonical path/device/inode/schema
and immutable persistent epoch. Capacity100000; no expiry/purge/backfill/reset.
Moving/restoring/replacing a store requires reconciliation. This storage is not
an OS sandbox or defense against privileged/agent filesystem writers.

Before one native handler, persist `(scope, command UUID, native run ID,
operation, raw request SHA256)`. Only an exact successful native ACK may update
that reservation once. Commit before the HTTP response. No guidance, secrets,
transcript, full terminal run or tool result is stored in the ledger.

## Wire

`GET /fleet/v1/controls/capabilities` returns object
`fleet.hermes.controls.capabilities`, contract_version1, store_id,
scope_fingerprint, profile=default, native_source_revision, single_send=true,
non_dispatch_lookup=true, operations=[steer,stop,approval] and exact GET lookup.

Native POST `/v1/runs/{run_id}/{operation}` requires original store epoch in
`X-Fleet-Control-Store-Id` and canonical command UUID `Idempotency-Key`.
Steer: closed `{input: nonempty string}`. Stop: empty bytes.
Approval: closed `{request_id, choice: once|deny, resolve_all:false}`.
Body64KiB/5s including chunked transport; handler10s; exact JSON HTTP200 ACK
64KiB, normalized stored ACK16KiB. Unexpected/negative replies are uncertain,
not proof of non-effect. Always/session/bulk/aliases are forbidden.

Duplicate POST returns409 without running the handler, even with a stored ACK.
Changed identity under the same scope/key also409. Capability failure, missing
epoch, unknown commit and handler failure never authorize another invocation.

`GET /fleet/v1/controls/lookup` requires exactly one each: store_id, command_id,
run_id, operation, request_sha256. Found200 identifies the same original context,
with state=uncertain or acknowledged+exact normalized ACK. Missing404, conflict409
and unavailable503 do not authorize dispatch or release of held capacity.
All responses use no-store; lookup contains no effect and never extends a permit.

ACK meanings: steer queued guidance; stop requested stopping; approval resolved
one exact decision. No receipt proves model consumption, terminal state, tool
completion, descendant termination, task acceptance or success of a stage.

## Required Fleet Consumer Packet

Fleet has a strict Rust original-context wire-consumer in
`backend/infra/src/runtime/control_outcome_wire.rs`. Preparation validates a
closed version1 capability packet, default-profile credential scope, canonical
store epoch and pinned native source. It serializes the exact action once,
including empty bytes for stop, and returns an opaque original context without
raw token or Debug implementation. Deserialization alone grants no authority.

Lookup revalidates origin/credential/body hash and the saved facts before GET;
it sends exactly the original five query fields, never a request body or POST.
HTTP200/JSON/identity encoding, ten-second and64KiB budgets are mandatory.
Closed response/ACK types reject duplicate, unknown and missing fields and
verify command/run/operation/epoch/scope/hash plus exact approval ID/choice.
The only positive outcomes are guidance queued, stopping requested or one
decision resolved. Uncertain has no dispatch meaning. No current capability
snapshot or terminal run replaces the original witness.

With `FLEET_CONTROL_FLEET__HERMES_CONTROL_OUTCOME_ENABLED=true`, the supervisor
prepares this context after its existing human/accepted free-chat/native
preflight, persists context and single-use claim through additive000015, then
sends the saved bytes/UUID/store headers once. Strict closed native HTTP200 ACK
commits the witnessed outcome atomically. An unknown HTTP or database ACK leaves
the submitted/uncertain receipt recoverable, without a second POST. Capability
failure before claim rejects the command; it never falls back to legacy sending.

The separate background worker scans at most100 UUID-keyset records per page,
advancing past invalid contexts and resetting only after an empty page. Between
pages it waits five seconds; each HTTP read is bounded to ten seconds/64KiB.
It checks the current accepted free-chat dispatch, agent/run/native pins and
original credential/origin, then uses only saved-context GET. It adopts no fresh
capability or store epoch, does not restore actor mutation rights, and cannot
backfill old submitted commands. Disabled flag starts no outcome worker. Approval
wire preparation/lookup exists, but this supervisor sender excludes approval.
Its separate journal is now implemented through additive000016; the approval
HTTP handler and worker remain legacy/unconnected in this packet.

Original approval reservation fixes `outcome_required=true` at INSERT. Claim
atomically persists immutable context and consumes `submission_claimed` once;
it requires the same owner/operator authority, active accepted free chat, exact
request/choice and closed source-pinned context. Legacy uncertain decisions
cannot be upgraded. Deferred guards bind claim/context and ACK/delivered/audit.
Completion records only an already verified prior effect; pending becomes
approved/denied, while cancelled requests and terminal runs are not reopened.
Unknown claimed decisions cannot become failed, receive another permit or be
deleted. Sender selection and native outcome verification must be connected and
accepted before this journal can recover actual HTTP approval decisions.

The stop/steer outcome row preserves exact bytes/capabilities, never adopts a
fresh epoch and has no dispatch-reset or delete operation. Claim rechecks the
actor and accepted free-chat journal before consuming the permit. Context,
operation and semantic payload must match the original command. ACK commits
revalidate accepted dispatch/native pins and exact saved context. The internal
completion method records an already witnessed effect; it does not authorize
a new effect after actor revocation. Public mutations still require current
human authorization. Deferred DB guards bind required context and ACK/receipt
atomically, including audit failure rollback. Positive ACK after a terminal
mirror preserves terminal history and adds the independent outcome fact only.

Do not enable its plugin on an installed agent before release/native/combined
acceptance. Existing command/decision
history cannot be given an epoch or witness after dispatch.

Consumer implementation must preserve the current authoritative human/owner
and accepted free-chat context gates, and independently verify producer
version/source/profile/scope and epoch before claiming a submitted permit.
Persist original command UUID, exact serialized body/hash, origin, credential
fingerprint and capabilities transactionally before HTTP. Stable body bytes,
not reconstructed semantic JSON, determine the producer fingerprint.

On lost HTTP/DB ACK or restart, authenticate original-context GET only. Check
every lookup field and ACK operation/native run/request/choice; commit the
existing receipt, audit and durable events atomically. Never re-send submitted
controls or issue a replacement key on missing/uncertain outcome. Unknown
approval must remain uncertain even if the native run has completed.

The producer does not share a transaction with native in-memory effects:
crash after effect but before durable ACK cannot be recovered as acknowledged.
That permanent hold requires explicit future operator reconciliation, not a
negative-lookup retry. Producer storage loss/rotation is likewise not absence
of prior effects. Fenced task/machine controls remain a separate admission.

These journal/consumer interactions remain explicit rollout requirements:

1. Stop/steer supervisor context/claim before POST and GET-only worker are
   implemented. Approval context persistence is additive000016; its exact-byte
   sender and bounded GET worker use the same default-off flag and a separate
   decision lifecycle. A producer
   epoch observed after submission is not a historical witness. The public
   actor idempotency key remains separate from the producer command UUID.
2. Additive000015 allows witnessed uncertain-to-acknowledged and separately
   preserves terminal mirrors as `terminal_observed` plus projected ACK. Applied
   migration000013 is unchanged. Verify the supervisor/native races without
   clearing the observed terminal fact or reopening the native run.
3. An approval can become cancelled by terminal mirroring before a lost decision
   ACK is read back. Persist witnessed decision delivery independently of that
   request lifecycle, without manufacturing another pending question, changing
   the choice or calling the decision endpoint again.
4. Before enabling the worker, test row-lock order, concurrent normal ACK and
   GET recovery, audit/event rollback, actor revocation, stale context, missing
   store and actual Fleet/gateway restarts. A lookup error remains a hold, not
   a new dispatch path. Old intents stay outside the extension.

## Evidence

[Native protocol harness](../../scripts/hermes_protocol_live/README.md) controls
scenario verifies real APIServerAdapter/AIAgent steer and interrupt, actual
HTTP reply loss, original-key readback and gateway restart, one inference and
uncertain rejected command hold. Model is deterministic loopback.
Component tests alone do not prove native approval. Separate managed native
`approval-outcomes` and `approval-restart` cover real decisions and original GET
settlement, including two Fleet SIGKILLs. The `control-outcomes` scenario
now passes actual Fleet sender/worker, transport loss and gateway PID restart
with one native POST per command and preserved terminal history. The model is
loopback and agents are disposable. Separate `control-restart` verifies two
Fleet SIGKILLs for controls. Combined extensions and installed enablement are not certified. No live central
identity, loaded-generation/OS isolation, accepted image install or full SDLC
acceptance is inferred. Exact hashes and limits are in the
[verification ledger](../CHAT_CLARIFICATION_VERIFICATION.md).
