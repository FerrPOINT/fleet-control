# Hermes Control Outcome v1

Status: Base producer and native QA implemented; production Fleet consumer,
positive native approval and installed rollout remain unverified. This is not
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

Production Fleet does not yet send these headers or consume this extension.
Do not enable its plugin on an installed agent. Existing command/decision
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

## Evidence

[Native protocol harness](../../scripts/hermes_protocol_live/README.md) controls
scenario verifies real APIServerAdapter/AIAgent steer and interrupt, actual
HTTP reply loss, original-key readback and gateway restart, one inference and
uncertain rejected command hold. Model is deterministic loopback.
Component tests cover exact-action approval ACKs, not a positive native tool
approval through this extension. No production Fleet recovery, live central
identity, loaded-generation/OS isolation, image build/install or full SDLC
acceptance is inferred. Exact hashes and limits are in the
[verification ledger](../CHAT_CLARIFICATION_VERIFICATION.md).
