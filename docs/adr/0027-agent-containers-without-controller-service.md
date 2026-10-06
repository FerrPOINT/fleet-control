# Agent Containers Without A Controller Service

## Status

Accepted architecture. Opt-in supervisor consumer implemented in the integration
candidate; automatic provisioning and live Docker/Hermes acceptance incomplete.

## Context

Separate Hermes homes/workspaces in one backend process environment do not
provide an agent process/filesystem boundary. The product needs one concrete
agent per container, while local infrastructure must remain in the existing
`sdlc1`/`sdlc2` Compose projects. A new user-facing host-controller service would
add deployment and authentication work without a current product requirement.
The accepted legacy runtime and its data must not be silently replaced.

## Decision

Fleet owns agent lifecycle and configuration. Each concrete agent runs in its
own container with its own runtime/config/workspace/log mounts and credentials.
Base provides shared private Compose lifecycle utilities, called by Fleet
through a fixed source-pinned subprocess protocol, not a new network service.
Agent containers receive neither Docker socket nor controller storage.

Fleet records original registration, generation, effective config, controller,
paths and source/context pins before start. Only original durable ACK/namespace
readback authorizes runtime identity and stop completion. Unknown outcomes hold
instead of replaying start, adopting another container/PID or using native HTTP.
Docker init PID is diagnostic metadata, never a backend-host signal target.
Hermes protocol and human/machine authorization remain independent checks.

Enablement is explicit until creation, trusted controller reachability, logs,
generation replacement, drain/activation/rollback and loaded config are verified.
Legacy native data/Java lifecycle remain readable and unchanged. This decision
does not grant task assignment, workflow first-step or SDLC completion authority.

## Consequences

No new product section/service is required. Existing Runtime actions will manage
containers. Controller-private generations are a temporary integration input,
not the final agent creation experience. Process-native acceptance does not
certify Docker behavior. Changes must preserve accepted mounts/images/data and
be tested in labelled disposable Compose projects with exact cleanup.

## Alternatives

- Native subprocesses: retain compatibility, but not the target agent boundary.
- Separate HTTP host controller: deferred; not required for the current local
  deployment, and no implicit extra service is introduced.
- Docker socket in agents: rejected; an agent must not manage host infrastructure
  or other agents through the Docker API.
