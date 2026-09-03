# Session Handoff Contract

Handoff moves a task session to a target primary agent.

Rules:

- target agent must exist
- session `agent_id` is retained as the storage column and becomes the target
  primary agent id
- API responses also expose `primary_agent_id` as the explicit public name
- session state becomes `handoff_requested`
- namespace follows the target agent namespace
- if the session has a selected leader and the target is an executor, that
  leader must manage the target executor
- primary participant is replaced with the target agent
- a new `session_agent_runs` row records the target runtime handoff
- transcript synchronization is runtime-specific and belongs to the runtime
  adapter boundary
