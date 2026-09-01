# Operations

## Provisioning

Provisioning creates database rows first, then materializes folders. Re-running
provision for the same agent is safe when the marker belongs to the same agent.

## Runtime Lifecycle

Hermes supports start, stop, restart and health. Java Agent operations are
reserved for phase 2 and return a typed validation response.

## Logs

Runtime stdout/stderr is captured into `agent_logs`. Secret-like markers are
redacted before persistence.

## Recovery

On backend restart, managed process handles are lost. The health action
reconciles status by marking an untracked Hermes process as stopped.
