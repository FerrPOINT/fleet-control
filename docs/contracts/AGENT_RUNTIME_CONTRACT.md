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

The shared process-log repository returns the exact persisted redacted row from
`INSERT ... RETURNING`, not a latest-log lookup. Concurrent streams do not change
that acknowledgement. See [Logging Standards](../LOGGING_STANDARDS.md); this is
not a durable generation cursor or Docker collection receipt.

All adapters use the common agent layout:

```text
agentN/runtime
agentN/config
agentN/workspace
agentN/logs
```
