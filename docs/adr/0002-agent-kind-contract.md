# ADR 0002: Agent Kind Contract

## Status

Accepted.

## Context

Hermes is the first fully runnable runtime, but Java Agent is already part of
the product direction. Delaying Java Agent modeling would force later changes to
database constraints, API payloads, UI create forms and runtime contracts.

## Decision

Model runtime type as `AgentKind = hermes | java_agent` from the first version.

## Consequences

- Runtime behavior stays behind adapter contracts.
- Java Agent can be visible in capability matrices and create screens before it
  is runnable.
- Product roles such as leader and executor remain separate from runtime kind.

## Alternatives

- Ship Hermes-only database fields. Rejected because it would create migration
  churn and ambiguous UI terminology later.
- Model developer/tester/lead as runtime types. Rejected because those are
  prompt, skill and workflow profiles, not process implementations.
