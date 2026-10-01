# ADR 0010: Separate Agent Chats And SDLC Ownership

## Status

Accepted for the Fleet foundation. Cross-service task bindings and automatic
SDLC remain unimplemented; see [SDLC implementation](../SDLC_IMPLEMENTATION.md).

## Context

A shared transcript across agents obscures authorship, runtime identity and
attempt history. Product roles also cannot represent both runtime technology
and the seven delivery specializations. Existing leader data must survive the
change without becoming a prerequisite for the new chat experience.

## Decision

- Keep runtime kind, legacy product role and `sdlc_role` independent.
- Group `/chats` by concrete agent, with that agent's sessions inside.
- Preserve `/sessions` and legacy leader history. Do not expose leader controls
  in the new Chats route.
- Keep free private chats separate from future task-bound chats. They cannot
  advance business stages.
- The target task-bound identity is immutable Tracker task ID plus agent ID.
  Rework continues that chat; agent replacement creates another chat.
- Tracker owns requirements, assignments, scheduling and transitions. Fleet
  owns configuration, chats and runs. Workflow owns step execution contracts;
  CI/CD owns build and deployment evidence.

The unique task binding and machine assignment protocol are future work, not
capabilities implied by the current UI. SDLC readiness stays blocked until
these contracts are verified.

## Consequences

- Transcripts do not move between agents.
- Legacy data and routes remain readable.
- Seven specializations are selectable without introducing virtual agents.
- Fleet does not introduce a competing business scheduler.
- Current chat screenshots prove navigation, not a full task lifecycle.

## Alternatives

- One task chat shared by all agents: rejected because it mixes runtime history.
- Stage-first navigation: rejected because stages change while agent identity
  and chat ownership must remain stable.
- Remove legacy leaders: rejected because existing data must be retained.
