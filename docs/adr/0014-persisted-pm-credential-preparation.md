# Persisted PM Credential Preparation

## Status

Accepted design; isolated implementation candidate. Linux/PostgreSQL and live
Base/Tracker acceptance remain pending. Model admission is not provided here.

## Context

Base may accept delegation before Fleet receives or saves its acknowledgement.
Repeating with a new key or rotated parent can mint a second secret. A restarted
Fleet must distinguish immutable issuance intent from an acknowledged child
without storing either bearer credential in its database.

## Decision

Extend the existing owner/key-unique PM creation journal with immutable intent
and acknowledgement. Persist intent before Base mutation and reuse the original
command/key/parent identity after uncertainty. Freshly introspect parent/child
and verify Tracker assignment context on every continuation. Persist token ID,
expiry and scopes, never a bearer. Reject expired or changed issuance identity;
do not automatically remint. Gate preparation behind disabled-by-default server
configuration. Add one additive migration to both supported database lineages;
refuse downgrade while any credential journal remains.

## Consequences

Creation can recover issuance independently of later Tracker availability, with
metadata-only audit. Operator reconciliation is necessary for parent rotation or
expired acknowledged children. Credential preparation is not execution admission:
lease/fencing, Workflow first step, runtime handoff and live end-to-end acceptance
remain separate requirements.

## Alternatives

- Memory-only issuance loses recovery identity after a crash.
- Storing bearer secrets in Fleet expands its database compromise surface.
- New keys or implicit parent rotation on retry can duplicate external effects.
- Treating delegated scopes as admission bypasses Workflow and owner receipts.
