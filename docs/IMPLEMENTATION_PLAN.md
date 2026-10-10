# Implementation Plan

## Current Scope

Deliver the approved runtime/execution scope, not just the already implemented
parts. Leaders are deferred; Java lifecycle is retained, but automatic Java SDLC
requires separately verified chat/control capabilities. Hermes remains unchanged.

The executable work order and acceptance boundaries are maintained in
[REMAINING_DELIVERY_WORK](REMAINING_DELIVERY_WORK.md). Do not maintain a second
copy of its workstream table here.

## Sources Of Truth

- [SDLC implementation](SDLC_IMPLEMENTATION.md): product scope and service ownership.
- [Chat clarification plan](CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md) and
  [contract](contracts/CHAT_CLARIFICATION_CONTRACT.md): owner flow and wire rules.
- [Current state](CURRENT_STATE.md): exact source and verified check evidence.
- [Gap register](GAP_REGISTER.md): limitations and required exit evidence.
- [Quality gate](QUALITY_GATE.md): required checks; source presence is not acceptance.

## Delivery Rules

Keep one reviewed candidate per qualification cycle and preserve Git history.
Use existing runtime supervisors, command journals, adapters and shared Base
utilities. Do not add another scheduler, host-controller service or custom
Hermes authorization hook. Tracker and Workflow are read-only dependencies.

Release only after the relevant backend, migration, frontend, browser and native
checks pass on the final inputs, and the real owner PM flow is accepted.
Fixtures, healthy processes and accepted runs do not prove completion of SDLC.

The former phase checklist is preserved in
[Git history](https://github.com/FerrPOINT/fleet-control/blob/ce4153f453e730dad1e315131d65ca030243264c/docs/IMPLEMENTATION_PLAN.md).
Its historical `done` labels are not current qualification evidence.
