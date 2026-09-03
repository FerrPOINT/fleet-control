# API Edge Cases

Required edge-case behavior:

- `POST /sessions` without `user_id` always owns the session by the current
  authenticated user.
- Direct executor session defaults to `visibility=private` and
  `leader_agent_id=null`.
- Direct leader session sets `primary_agent_id` and `leader_agent_id` to the
  same leader.
- Selecting a leader for an executor session requires an existing
  `leader_executors` binding.
- Selecting a leader on a session with active runs should be confirmed by the UI
  and rechecked by the backend.
- A private session is invisible to leader-scoped reads.
- A leader can write only where `agent_sessions.leader_agent_id` is that leader.
- Delegation requires a leader-scoped parent session and a managed executor.
- Runtime unavailable errors must leave the Fleet mirror consistent and mark the
  related run as failed or waiting.
- Java Agent operations return typed `not_implemented` until phase 2.
