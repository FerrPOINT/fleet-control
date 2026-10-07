# ADR0034: Recovered Controller Namespace Stop

## Status

Accepted for the internal candidate. Installed rollout, actual Docker stop and
restored SDLC execution are separate acceptance gates.

## Context

Recovered custody does not grant the old supervisor's effects. Explicit stop
still needs to contain the original namespace without rewriting its launch or
repeating an uncertain kill. Native exit can be confirmed after the admitted
lease expired or after the native reply was lost.

## Decision

Add000022 with a stable original-launch stop intent and a separate once-only
dispatch envelope. Require exact current DB custody and native physical/live
owner checks before admission. Preserve the existing Base stop operation ID
equal to the launch UUID. A claimed command cannot be resent or reset; recovery
uses original namespace observation only. Validate a closed stop receipt or
namespace-exited observation against the original immutable snapshot.

Commit outcome, original launch exit, stopped runtime state and hash-only audit
atomically. Historical positive settlement is not lease renewal. An outcome
replay cannot overwrite a later generation. Keep model/control/task admission
and session/approval/configuration reconciliation independent and fail closed.

## Consequences

A crash after claim but before native acceptance can leave a running namespace
held. The candidate deliberately does not repeat the kill without proof of safe
non-acceptance. Retained history prevents destructive downgrade. The explicit
stop action does not complete a task or repair unknown Hermes acceptance.

## Alternatives

- Reuse the legacy owner path: rejected because original owner custody is fenced.
- Retry stop on a timeout: rejected because timeout does not prove non-acceptance.
- Mark runtime stopped on an HTTP error: rejected because namespace exit is unknown.
- Treat stopped namespace as stage completion: rejected because workflow receipts
  and business acceptance are separate.

See [contract](../contracts/CONTROLLER_RECOVERY_V1.md) and
[migration lineage](../MIGRATIONS.md).
