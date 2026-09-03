# Database Indexes

Required index coverage:

- `users.email`, `users.system_role`.
- `agents.ordinal`, `agents.name`, `agents.kind`, `agents.product_role`,
  `agents.status`.
- `leader_executors.leader_agent_id`,
  `leader_executors.executor_agent_id` and unique pair.
- `agent_sessions.user_id`, `agent_sessions.agent_id`,
  `agent_sessions.leader_agent_id`, `agent_sessions.parent_session_id`,
  `agent_sessions.state`, `agent_sessions.task_key`.
- unique `(agent_sessions.user_id, idempotency_key)` where key is not null.
- `session_messages.session_id, created_at`.
- unique message idempotency index by session/user/key where key is not null.
- `session_agent_runs.session_id`, `session_agent_runs.agent_id`,
  `session_agent_runs.state`.
- `deployment_jobs.state`, `deployment_jobs.agent_id`,
  `deployment_jobs.created_at`.
- `audit_log.actor_user_id`, `audit_log.action`,
  `audit_log.entity_type/entity_id`, `audit_log.created_at`.
- recent `agent_events` and `agent_logs` access by time and agent.
