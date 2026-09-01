# ADR 0002: Agent Kind Contract

Decision: model runtime type as `AgentKind = hermes | java_agent` from the first
version.

Reason: Hermes is implemented first, but Java Agent must not require a later DB
or UI redesign.
