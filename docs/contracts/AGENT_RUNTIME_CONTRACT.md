# Agent Runtime Contract

## Durable Runtime Controls Unit13

Free-chat steer/stop require authenticated human/owner authority, an immutable
request key and one committed submission permit. Unknown effects hold the run;
readback never replays a mutation. ACK, terminal runtime evidence, OS process
stop and SDLC completion are distinct. Task/PM bindings do not grant control
admission. See the [source candidate](../plans/2026-10-09-runtime-controls-release.md).

## Hermes Journal Release Unit12

The journal release is free-chat only. Task-chat and PM bindings do not grant
submission authority. Unknown POST results retain capacity and cannot be retried;
recovery of an accepted run uses its original journal and GET-only readback.
Wire acceptance and completed runtime output do not prove SDLC stage completion,
model admission or deployment readiness.

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
