# PM Execution Lease Readback v1

Status: source consumer implemented; verification and release evidence belong to
[the ledger](../CHAT_CLARIFICATION_VERIFICATION.md#pm-execution-lease-readback-8-october-2026).
Producer: read-only Tracker114 `357caa7a60a717eb7b0ac72f286b793326992931`.

## Authority And Transport

`PmCredentialCoordinator::read_execution_lease` is a machine-only internal port,
not a browser route. It requires the original acknowledged credential journal,
unchanged parent fingerprint/origins/TTL/scopes, child token identity/expiry and
reservation machine subject. Fresh Base child introspection and Tracker context
verify the assigned agent/execution/version and owner; a cloned operation with a
different machine subject is rejected before HTTP. Raw credentials remain in
memory only; this observation does not persist a second lease authority.

The fixed configured Tracker origin receives authenticated
`GET /api/v1/issues/{immutable-task-id}/sdlc/pm-draft-execution-lease` with optional
original `idempotency_key`. No claim, heartbeat, Workflow mutation or Hermes
request is issued. The GET has a five-second request/body deadline, 16 KiB limit,
identity encoding, no-cache/no-store request and exact-200 JSON requirement.
Redirects, proxy, retries, encoded/oversized bodies and non-200 success are not
accepted. Auth failures stay auth failures; upstream bodies/tokens are not echoed.
Credential expiry is checked again after receipt of the body.

## Closed Evidence

Every nested object is closed. `current` and `operation` are required nullable
fields. UUIDs are canonical and non-nil; versions are positive safe integers;
timestamps round-trip as Tracker's UTC nanosecond strings. Binding, owner CAS,
assignment/execution/agent/version and holder match the original reservation.

Lease chronology requires claimed <= heartbeat <= observation, expiry exactly
heartbeat +30 seconds and initial version1 claimed=heartbeat. `unclaimed`,
`active` and `expired` must agree with the current lease and observation time.
Receipt constants remain TTL30, heartbeat10 and `dispatch_allowed=false`.

Optional operation lookup verifies the exact original command/key and canonical
SHA256 labels `claim_pm_execution_lease` or `heartbeat_pm_execution_lease`.
Its historical result must refer to the same lease/claimed time and cannot have
a newer version or heartbeat than current. Equal versions require equal content.
Claim results are version1; heartbeat results increment their original expected
version exactly once. An unsolicited operation, altered hash or foreign receipt
is rejected. A missing original operation is absence of evidence, not permission
to redispatch. A historical receipt cannot renew expiry or grant current authority.

## Creation And Remaining Execution

Credential preparation now consumes this fresh GET after storing the child ACK.
An unclaimed lease completes credential preparation. A known active lease also
requires the saved original claim and exact keyed operation evidence; foreign
active or expired claims require reconciliation and quiescence, never automatic
adoption. Failure preserves the credential ACK and awaiting-admission state,
with no runtime run.

## Original Claim Coordination

`claim_execution_lease` derives its command from the immutable reservation and
uses `fleet-pm-lease:{creation-operation-uuid}`. The existing locked creation
saga persists command/hash before POST. Fresh delegated-token introspection,
Tracker context and keyed lease readback precede the mutation. Only an unclaimed
lease without its original receipt permits POST. HTTP 201 or 200 must contain a
closed canonical claim receipt; a subsequent fresh keyed GET must agree with
that receipt and show the same lease active. Lost ACK leaves the intent pending.
Reconciliation reads its original key and persists the original acknowledgement;
it never chooses another key, adopts a different lease or renews an expired one.

Private journal progress is absent -> original claim -> immutable ACK. Migration
000023 adds assignment/owner/holder/TTL and immutable JSON guards without changing
000010/000011. Audit stores only operation/lease identifiers once. Tokens stay in
memory. The public creation response still has `dispatch_allowed=false`; this
internal coordinator is not wired to automatic creation dispatch.

Heartbeat scheduling, trusted first-step admission, loaded configuration
attestation, model dispatch, structured PM tools and checkpoint resume remain
separate work. Claim coordination adds the explicit journal migration, but no
public endpoint, SDK/image pin, installed-runtime or Java capability change.
