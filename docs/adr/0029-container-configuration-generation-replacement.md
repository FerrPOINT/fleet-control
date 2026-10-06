# Container Configuration Generation Replacement

## Status

Implemented candidate; 31 focused Linux/PostgreSQL cases and strict Clippy pass.
Full exact-source workspace verification passes 523 cases with 29 explicit
opt-in ignores; real Docker acceptance remains pending. This does not authorize
installed enablement.

## Context

The native activation journal already drains assignments, snapshots prior files,
reads back applied files and retains a recovery hold on unknown outcomes.
Docker cannot use a retained native child or restart an exited namespace. An
unknown preparation may exist before a runtime launch row; rewriting its files
or changing its generation would violate immutable custody.

## Decision

- Validate configuration phase/revision/hash under the same agent/runtime/head
  locks before the pre-create fence, and again before start. Regular creation
  is forbidden while draining; activation requires the claimed desired revision;
  rollback binds the still-effective prior revision.
- Before activation, validate isolated paths and marker ownership. An active
  container must be owned and observed by its original controller. A pending
  pre-create fence or unpublished preparation document prevents file mutation.
- Prepare the private activation journal before stopping the old namespace.
  Only a confirmed original namespace exit permits file writes and a fresh
  generation. Start the candidate with phase `activation`; effective revision
  advances only after readiness and successful activation persistence.
- A stopped, reconciled agent may apply and read back its files without starting
  a runtime. Its effective configuration is not proof of runtime or SDLC readiness.
- If candidate readiness fails, first confirm its namespace exit, restore/read
  back prior files and start another fresh generation with phase `rollback`.
  Preserve the failed revision and previous effective revision.
- Unknown prepare/start/stop, foreign controller or missing custody retains
  drain/journal/candidate files. Do not invent exit, overwrite a potentially
  executing candidate, prepare rollback or fall back to native.

## Consequences

No new migration or Base wire-protocol change is required. Generation history,
prepared receipts and private journals remain immutable. Interrupted activation
still needs explicit reconciliation; this decision does not implement takeover
or loss recovery. Real named-volume Hermes/model evidence remains required in
addition to the fake-Base supervisor/PostgreSQL regressions.

## Alternatives

Restarting the same container loses the namespace-exit proof and is rejected.
Hot-editing files under a running agent mixes desired/effective configuration.
Treating HTTP health alone as loaded-config proof bypasses custody and is
rejected. Automatically rolling back unknown delivery can create concurrent
effects and is rejected.
