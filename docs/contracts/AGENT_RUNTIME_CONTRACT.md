# Agent Runtime Contract

Every runtime adapter must provide:

- runtime kind
- capability metadata
- provisioning behavior
- command preview
- start, stop, restart and health behavior
- session metadata mapping
- log capture policy
- secret redaction policy

All adapters use the common agent layout:

```text
agentN/runtime
agentN/config
agentN/workspace
agentN/logs
```

## Configuration Foundation Candidate

Package preparation freezes an exact Base Git package and Workflow-owned mapping
in an existing draft. Validate/activate/readiness reverify that mapping; supervisor
preflight reads the owner before changing files/runtime. An owner failure before
mutation preserves the old head/files and releases drain; uncertain rollback
retains drain. Config/readiness observations are not native admission or business
completion. See [Workflow binding](SDLC_WORKFLOW_BINDING_V1.md) and
[release scope](../plans/2026-10-09-runtime-config-release.md).
