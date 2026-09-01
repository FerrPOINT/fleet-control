# Session Handoff Contract

Handoff moves a task session to a target agent.

Rules:

- target agent must exist
- session `agent_id` becomes the target agent id
- session state becomes `handoff_requested`
- namespace follows the target agent namespace
- transcript synchronization is runtime-specific and belongs to adapter work
