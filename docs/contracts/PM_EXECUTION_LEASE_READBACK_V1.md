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
Only `unclaimed` completes this prerequisite; active/expired claims require
original-key reconciliation and quiescence, never automatic adoption. Failure
preserves the credential ACK and awaiting-admission state, with no runtime run.

This is not claim/heartbeat scheduling, trusted first-step admission, loaded
configuration attestation, model dispatch, structured PM tools or checkpoint
resume. Workflow90 `11398711aa04605bc1a618622a84ae648e6de0c8` only changes an
OpenAPI description from the prior inspected head; its missing trusted owner
evidence and post-dispatch PM bind remain blockers. No public API, migration,
SDK/image pin, installed runtime or Java capability is changed.
