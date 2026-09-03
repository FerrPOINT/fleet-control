# Operations

## Provisioning

Provisioning creates database rows first, then materializes folders. Re-running
provision for the same agent is safe when the marker belongs to the same agent.

## Runtime Lifecycle

Hermes supports start, stop, restart and health. Java Agent operations are
reserved for phase 2 and return a typed validation response.

## Deployments

Provision and runtime update work is represented by deployment jobs. Operators
use `/deployments?tab=jobs` to create, inspect and cancel jobs.

## Settings

Runtime roots, runtime sources, port ranges, integrations and auth settings are
managed in `/settings`. Secret-like values must be returned redacted.

## Logs

Runtime stdout/stderr is captured into `agent_logs`. Secret-like markers are
redacted before persistence.

`/logs` has process logs, events and audit tabs. Use audit for role changes,
settings changes, skill/config edits, runtime actions, handoff and delegation.

## Recovery

On backend restart, managed process handles are lost. The health action
reconciles status by marking an untracked Hermes process as stopped.

Use idempotency keys when retrying session/message create calls. If the previous
payload differs, the API returns `409` and the operator should create a new
intent instead of replaying the old key.
