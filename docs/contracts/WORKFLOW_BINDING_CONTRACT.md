# Workflow Binding Contract

Fleet Control stores workflow bindings but does not own workflow definitions.

Fields:

- `agent_id`
- `namespace_id`
- `namespace_name`
- `workflow_id`
- `workflow_name`
- `binding_status`

Status is computed by Fleet Control from the live `project-workflow` catalog:

- `connected` — both stored ID/name pairs exactly match live namespace and workflow entries.
- `stale` — a persisted selection no longer exists or was renamed upstream; Fleet keeps the values for operator diagnosis and does not silently retarget an agent. An operator can explicitly select a valid catalog namespace/workflow pair through `PUT /api/v1/workflow-bindings/{agent_id}`.
- `unbound` — no namespace or workflow selection was stored for the agent.

The source of truth for namespace/workflow behavior is `project-workflow`.
