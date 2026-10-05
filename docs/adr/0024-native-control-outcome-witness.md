# ADR 0024: Native Control Outcome Witness

## Status

Producer/native protocol, Fleet GET wire and internal stop/steer journal
implemented and component-tested; opt-in stop/steer supervisor connected and
verified against actual managed Hermes with a loopback model. Positive approval
outcomes, Fleet OS-restart controls, combined plugins and installed rollout
remain pending; automatic task controls remain denied.

## Context

A lost steer/stop/approval reply may follow a real side effect. Terminal run
status cannot prove whether guidance or a particular decision was accepted.
The existing Fleet ledger correctly holds uncertainty but native Hermes does
not provide original-command readback. Fleet must not edit runtime databases.

## Decision

Use an explicit Base-owned native platform plugin. It reserves immutable
scope/key/run/operation/raw-body hash before one handler and stores only a
validated native ACK before returning HTTP. Separate authenticated GET observes
the original epoch and command without dispatch. Producer storage has private
file ownership, SQLite durability, continuity checks and immutable history.
Missing/uncertain replies never grant retry. No backfill for legacy commands.
Fleet preserves original producer context transactionally with a single-use
claim through additive000015. The independent outcome row keeps witnessed ACK
separate from an already observed terminal run, without modifying applied000013
or reopening the run. ACK, receipt, applicable stopping state, audit and durable
event are atomic; legacy submitted commands cannot acquire context later.
Supervisor dispatch uses exact saved bytes/epoch/UUID only after claim; a
separate bounded UUID-keyset GET worker commits original-context witnesses.
The default-false flag starts no worker when disabled and never backfills old
submitted commands. Approval decision journaling remains pending.

## Consequences

Transport loss after durable ACK becomes recoverable without duplicate effects.
Crash between effect and ACK remains a permanent unknown hold because native
in-memory effects and SQLite do not share a transaction. Operator reconciliation,
store restore/rotation and OS containment remain mandatory separate work.
ACK is not tool completion, safe process stop or success of an SDLC stage.
The plugin stays disabled on accepted installations until ordered release,
exact-head CI and installed compatibility are verified. Local native acceptance
does not replace those release gates.

## Alternatives

- Blind POST replay: duplicate guidance/decisions, rejected.
- Derive success from terminal state or absent pending approval: indirect,
  rejected.
- Fleet writes Hermes SQLite: ownership/security violation, rejected.
- Patch native Hermes handlers: prototype is read-only; use the supported
  platform hook instead.

See [wire contract](../contracts/HERMES_CONTROL_OUTCOME_V1.md).
