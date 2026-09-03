# ADR 0004: Leaders And Executors

## Status

Accepted.

## Context

Leaders and executors are both technical runtimes, but the product needs
different navigation, permissions, prompts and team assignment behavior.
Runtime kind must not be overloaded with business role.

## Decision

Keep all managed runtimes in `agents`, and split product behavior with
`agents.product_role = leader | executor`.

`AgentKind` remains only the runtime implementation (`hermes` or `java_agent`).
Profiles such as `developer`, `tester` and `it_lead` configure prompts, skills
and workflow bindings.

Leaders manage executors through `leader_executors`. Backend validation rejects
leader-to-leader, executor-to-executor and self-management bindings.

## Consequences

- `/agents` can remain a technical inventory.
- `/leaders` and `/executors` can present product workflows without duplicating
  runtime storage.
- Team membership becomes auditable and enforceable in session APIs.

## Alternatives

- Separate leader and executor tables. Rejected because runtime lifecycle,
  skills, config and folders would be duplicated.
- Treat leaders as users. Rejected because leaders have prompts, skills, logs
  and runtime processes like other agents.
