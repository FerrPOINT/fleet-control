# ADR 0006: Session Mirror And Runtime Runs

## Status

Accepted.

## Context

Fleet Control must coordinate tasks across runtimes without depending on Hermes
internal storage. Direct writes into Hermes SQLite would couple Fleet to Hermes
implementation details and bypass runtime behavior.

## Decision

Fleet Control stores a transcript/control mirror and per-agent runtime links
instead of treating Hermes SessionDB as Fleet's writable database.

`session_messages` records user, agent and system messages visible to the
control plane. `session_agent_runs` maps one Fleet session to one or more
runtime sessions, because a leader-scoped task can involve a selected leader and
an executor.

Fleet writes to Hermes only through the runtime supervisor/adapter boundary.

## Consequences

- Runtime adapters remain the only write channel into Hermes/Java Agent.
- Fleet can audit messages, participants and handoffs consistently.
- Mirror drift must be handled by stream reconciliation and runtime run states.

## Alternatives

- Write directly into Hermes SessionDB. Rejected because it is unsafe and would
  bypass runtime-level validation.
- Store only external runtime ids. Rejected because Fleet needs search, audit,
  permissions and cross-agent session views.
