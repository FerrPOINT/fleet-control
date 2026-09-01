# Data Model

Tables:

- `users`: operator accounts and refresh token hash.
- `runtime_templates`: runtime kind metadata and capabilities.
- `agents`: sequential agent identity, type, role, status, ports and paths.
- `agent_runtime`: desired state, pid, health, command and env preview.
- `agent_configs`: config JSON, SOUL.md text and redacted env JSON.
- `agent_skills`: per-agent skill selection and optional edited content.
- `agent_sessions`: task chats owned by an agent.
- `workflow_bindings`: per-agent namespace/workflow link.
- `agent_events`: audit-friendly event stream for UI invalidation.
- `agent_logs`: bounded process/runtime log records.
- `audit_log`: future operator action audit.

Important constraints:

- `agents.ordinal` and `agents.name` are unique.
- `agent_skills` is unique by `(agent_id, name)`.
- `workflow_bindings` is unique by `agent_id`.
- runtime kind, role, status, desired state, skill state and session state are
  checked text values.

Indexes cover agent status filters, session lists, task-key lookup, workflow
namespace lookup and recent events/logs.
