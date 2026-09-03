# Operations

## Provisioning

Provisioning creates database rows first, then materializes folders. Re-running
provision for the same agent is safe when the marker belongs to the same agent.

## Runtime Lifecycle

Hermes supports start, stop, restart and health. Java Agent operations are
reserved for phase 2 and return a typed validation response.

## Agent File Purge

Default agent delete archives the agent and leaves files intact. Physical purge
is a separate operator action:

1. Archive the agent.
2. Open the agent workspace tab.
3. Review the storage report totals, marker status and retention hint.
4. Type the exact `agentN` name into the purge confirmation field.
5. Run file purge.

The backend recomputes `agents_root/agentN`, rejects symlinks, requires a
matching `.fleet-agent.json` marker, removes only that folder and writes both an
event and an audit entry. Purge does not remove database sessions, logs or
audit history.

The storage report is read-only. It scans `runtime`, `config`, `workspace` and
`logs`, counts files, directories and symlinks without following symlink
targets, and reports whether the folder is currently purge-eligible.

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
