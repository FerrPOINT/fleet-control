# API Edge Cases

Leader-specific cases below apply to preserved legacy `/sessions` routes.
The new `/chats` route excludes leader controls. Target task-bound identity and
SDLC transition gates remain described in [SDLC implementation](SDLC_IMPLEMENTATION.md).

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
- Java chat/control/config activation operations remain phase 2; the existing
  jar lifecycle remains available and must not be reported as unimplemented.
