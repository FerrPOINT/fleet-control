# Workflow Binding Contract

Fleet Control stores workflow bindings but does not own workflow definitions.

Fields:

- `agent_id`
- `namespace_id`
- `namespace_name`
- `workflow_id`
- `workflow_name`
- `binding_status`

The source of truth for namespace/workflow behavior is `project-workflow`.
