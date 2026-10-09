# ADR 0018: Atomic Terminal And Pinned Recovery

## Status

Accepted source design; verification results belong in the verification ledger.
This does not grant task/PM admission or installed-runtime acceptance.

## Context

A crash after the effective native session pin but before the first stream worker
left a running record outside the pending-only recovery queue. Native status can
survive a process restart while its in-memory SSE queue does not. Previously,
assistant insertion, prompt delivery and terminal run state used separate commits;
a late database failure could expose a partial outcome and retain capacity.

## Decision

The bounded UUID-keyset recovery queue covers accepted pending, running, waiting
and stopping free-chat runs. Every HTTP read requires the original accepted
request journal, origin and credential context. Existing task/PM boundaries remain
closed. Pending acceptance pins its effective session once; an already pinned run
requires exact session equality and never creates a replacement stream worker.
Recovery of pinned runs uses authenticated bounded GET only, not POST or SSE.

Validated native terminal evidence calls one private repository transaction.
Lock order is agent, session, run, prompt, outbox, journal.
Session serialization uses `FOR NO KEY UPDATE`, compatible with the `KEY SHARE`
checks of existing progress/event foreign keys; exclusive session key locks would
invert the old run/message-first progress paths. Concurrency regressions must
exercise actual blocking and trigger writes, not only simultaneous happy paths.
The transaction checks the exact concrete agent, prompt/native run/effective session mapping and accepted
journal, then commits the optional redacted assistant, preview, prompt delivery,
run state/error/timestamps and trigger-owned durable events together. Outbox and
journal acceptance remain unchanged. Empty completion has no fabricated reply;
failed/cancelled outcomes create no assistant.

Session locking serializes assistant deduplication. Exact terminal replay performs
no writes, cursor changes or timestamp updates; conflicting state/body/identity
is rejected. A historical partial assistant can be reused only with the identical
redacted body and author identity. Capacity is released by the committed terminal
state, not by transport EOF, polling failure or lease expiry.

An SSE worker periodically reads the persisted terminal state and drops its old
stream after GET recovery wins. Delta, tool-mirror and approval insertion paths
serialize with terminal persistence and refuse late writes; nonterminal cache
updates cannot reopen a terminal run. Original-context checks precede stream attach.

## Consequences

Crash recovery can finish a known accepted and pinned free-chat run even when the
native SSE endpoint is gone. PostgreSQL fault/concurrency tests and controlled
HTTP restart tests are required. No new migration, public route, Java capability
or dependency/runtime pin is introduced.

Missing legacy journals, changed origin/credential and invalid status retain
capacity for reconciliation. The companion [ADR 0019](0019-native-original-key-recovery.md)
adds bounded unknown-key recovery; known-ID GET is separate from that horizon.
Atomic terminal persistence alone does not provide original-key lookup, replay of missed native
tool/approval events, operator recovery or OS process-tree quiescence. Native run
completion still does not complete a Tracker stage or authorize Workflow resume.

## Alternatives

- Reattach SSE after restart: rejected as the recovery proof; native event queues
  may no longer exist and duplicate consumers can repeat tool/approval effects.
- Mark a pinned running record complete after timeout: rejected; there is no
  terminal evidence.
- Keep three terminal commits with retries: rejected; partial transcript and
  capacity state remain observable after a crash.
- Add a second scheduler: rejected; recovery observes existing execution only.
