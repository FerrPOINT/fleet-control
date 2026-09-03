# Workflow

Fleet Control stores workflow bindings, not workflow definitions.

Source of truth:

- `project-workflow` owns namespaces, workflow definitions and supervisor
  policy.
- Fleet stores `namespace_id`, `workflow_id`, binding status and local display
  metadata.

Agent behavior:

- Developer, tester and IT lead behavior comes from profile, prompt/SOUL, skills
  and workflow binding.
- Each agent can have a different namespace and workflow.
- Leaders can manage executors across configured bindings, subject to backend
  validation.

Session behavior:

- Session namespace defaults from the primary agent.
- Delegated child sessions inherit the leader from the parent and may target the
  executor namespace.
