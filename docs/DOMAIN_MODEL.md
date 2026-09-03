# Domain Model

Core terms:

- Agent: a technical runtime instance with isolated folders and process state.
- Agent kind: runtime implementation, `hermes` or `java_agent`.
- Product role: `leader` or `executor`.
- Profile: prompt/skill/workflow preset such as `developer`, `tester`,
  `it_lead` or `custom`.
- Leader: an agent that can coordinate selected executors.
- Executor: an agent that runs delegated or direct work.
- Session: a user-owned chat/task with one primary agent.
- Selected leader: optional leader for a session, at most one.
- Delegation: child executor session created from a leader-scoped parent.
- Runtime run: link between a Fleet session and a runtime-local session.

Rules:

- Runtime kind never encodes business profile.
- Human-created executor sessions are private by default.
- Leader-created executor sessions inherit the leader and parent session.
- Fleet mirrors transcript metadata but runtime adapters deliver messages.
