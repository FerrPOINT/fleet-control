# ADR 0016: Hermes Original Request Journal

## Status

Accepted source design for free-chat dispatch. Verification is recorded separately;
this decision does not enable automatic SDLC or unknown-key POST recovery.

## Context

Pinned Hermes scopes idempotency by authentication/profile identity. Its 202
can be lost after execution starts. Credential rotation can accept the same key
as a different run. Durable capabilities and the 86400-second retention do not
prove store continuity: a new empty SQLite advertises durable storage too. The
native HTTP contract has GET by known run ID, not a non-dispatch lookup by key.

## Decision

Atomically reserve the concrete free-chat run and immutable original request
journal before POST. Save exact bytes/hash, original UUID key, loopback origin,
default-profile credential fingerprint, verified bounded wire facts and a DB-clock
horizon with a 60-second retention margin. Do not store raw runtime tokens or
serialize private intent DTOs into public responses/logs/audit.

Consume a one-winner durable submission permit before network IO. Send the saved
bytes without rebuilding prompt/model/options. A verified 202 commits native
ID, prompt delivery, outbox and journal progress together. Unknown submission
holds capacity and pending delivery with an error; it never obtains another
permit. A prepared intent is distinct from one that may have been submitted.
Classify unacknowledged errors under the same message lock used by submission:
a late error from a stale prepared snapshot cannot overwrite pending delivery
after another sender commits submitted, nor prevent its subsequent native ACK.

Known-ID session pin recovery requires accepted journal and matching original
origin/credential context before GET. Historical ACKs without that proof stay
readable and unresolved; migration does not invent credentials retrospectively.
Task/PM runs remain excluded and require their own fenced admission authority.

## Consequences

Concurrent send attempts cannot create a second POST through this path. Restart
preserves original request evidence, but automatic prepared-intent recovery and
unknown-key reconciliation remain separate work. Positive owner-supported
non-dispatch lookup is needed to recover an unknown original native ID; negative
lookup, 404, missing storage or expired retention never authorizes resend.

Migration 000012 is additive; guards prohibit identity/history rewrite and
downgrade with journal rows. No public API field is added. Runtime completion is
still not process-tree quiescence, Workflow receipt or Tracker stage success.

## Alternatives

- Rebuild request from current agent/config: rejected; input and scope can change.
- Retry the saved key whenever durable=true: rejected; store loss can duplicate execution.
- Check replayed=false after POST: rejected; the new execution has already started.
- Backfill legacy fingerprints with current credentials: rejected; no original proof.
- Clear capacity on timeout/404: rejected; neither proves the old run stopped.
