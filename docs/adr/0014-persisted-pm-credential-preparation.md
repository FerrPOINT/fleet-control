# ADR 0014: Persisted PM Credential Preparation

## Status

Accepted source design, opt-in and disabled by default. Live Base/Tracker
interoperability, native admission and runtime tool handoff are not accepted.

## Context

Tracker reserves an immutable PM assignment for the human owner's Draft. Base
delegates a short-lived child under the original parent token and operation key;
an acknowledgement can be lost after issuance. A new key or rotated parent would
create a different operation. A successful issuer response is not current Tracker
assignment authority and must not expose a root credential to Hermes.

## Decision

Use the existing owner/key creation saga after its atomic chat receipt. Persist
exact command/hash/key/TTL, normalized integration origins, canonical machine UUID
and original high-entropy parent fingerprint before Base mutation. Fresh root
introspection requires that UUID and exactly Tracker read/write. The child request
adds one exact task/assignment/execution/agent/version grant, not a wildcard.

Persist ACK metadata only (token UUID, expiry, scopes), then introspect the child
and read its permitted Tracker task context. Match original human owner and every
frozen assignment/binding field. Failed readback retains ACK; continuation replays
the original command under the original parent and repeats current checks. Secret
bearers are memory-only, nonserializable and absent from audit/public DTOs.

Additive migration 000011 extends the creation guard without editing historical
000010. Journal progress is absent -> immutable intent -> immutable ACK. SQL
guards reject malformed first writes, secret/extra fields, receipt replacement
and erase. Once-only intent/ACK audit shares the receipt transaction. Downgrade
refuses retained journals. The absent field preserves legacy operation bytes.

The deployment-owned flag defaults to false. Issuance does not claim execution
capacity, bind Workflow, renew a credential, start a run or authorize tools.
Creation still ends at awaiting admission with dispatch disabled.

## Consequences

Restart/concurrent replay cannot silently mint another child. Parent rotation,
changed origins/TTL, expiry and revoked/stale assignment require explicit recovery.
Retained journal metadata and actual child use must be distinguished from native
admission, workflow progress and successful SDLC. Real producer interoperability
is a separate acceptance gate from PG/HTTP fixtures. Renewal/revocation recovery
and runtime handoff still need dedicated persisted operations and authorization.

## Alternatives

- New key/parent on retry: rejected because unknown issuance may already exist.
- Persist raw child secret: rejected because the operation and backups would hold
  transferable runtime credentials.
- Treat issuer ACK as readiness: rejected because Base does not own Tracker
  assignment, Workflow or runtime authority.
- Rewrite historical migration 000010: rejected because applied schemas need a
  forward-compatible upgrade and retained recovery material.
