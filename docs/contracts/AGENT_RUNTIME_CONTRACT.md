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

SDLC configuration observation is a separate, read-only owner interface;
see [the API contract](../API.md#sdlc-configuration-observation).
Fresh Base PAT authorization and concrete-agent allowlisting do not authorize
dispatch. Managed-file verification, loaded-runtime admission and terminal
execution evidence are distinct. The current observation is not runtime-ready.
Effective heads and exact agent/revision lookups are independent of paged history.
Pinned package reads have fixed IO/time bounds and no HEAD/network fallback;
neither a successful read nor an older validated draft authorizes activation or
assignment dispatch.

Base-marked configuration revisions also freeze a separate
[Workflow binding](SDLC_WORKFLOW_BINDING_V1.md). Fresh owner ID/name/profile
verification precedes supervisor side effects. An unavailable preflight fails
the revision without retaining drain; an unknown apply/rollback keeps drain.

Terminal runtime evidence must identify the accepted run; EOF, nested child
completion and a requested stop are not completion. The implemented Hermes
adapter validates exact terminal names, native success flags and bounded status
readback before ending a run. Runtime completion is separate from process-tree
quiescence and owner-authorized business transitions. See the
[Hermes wire contract](HERMES_ADAPTER_CONTRACT.md); this does not grant phase-2
Java chat/control capabilities.
