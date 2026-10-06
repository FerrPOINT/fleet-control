# ADR 0030: Generation-Bound Container Endpoint

## Status

Implemented in the integration candidate; actual controlled-model Docker chat
acceptance passes. Fresh Linux workspace checks pass526 tests, fmt/check/strict
Clippy, with30 explicit opt-in ignores; separate migrations pass19 cases without
ignores. Populated endpoint downgrade refusal and ordered release/CI still require
their own evidence.
Not enabled in installed services and not evidence of complete SDLC readiness.

## Context

The first real Rust-supervisor/Docker/Hermes chat gate started two UID 999 gateways
but rejected its first dispatch. Both the Rust journal admission and the original
database trigger assumed `http://127.0.0.1:<agent-port>`. A separate Docker
namespace has a Base-verified bridge address instead. Permitting any supplied
private address would not establish which runtime receives a prompt.

## Decision

Add the immutable, internal `runtime_launch_endpoints` journal in additive
migration `000019`. The original Base endpoint guard first verifies custody,
registration, network and running namespace. The owning Rust supervisor then
records its canonical origin against the exact durable launch binding and PID.
The transaction locks the agent and original launch. Identical replay succeeds;
changed origin, PID, controller/binding or a closed launch fails.

This API is an internal repository operation, not a user-facing endpoint setter.
The database rejects non-container custody, unstarted/foreign launches, wrong
ports and non-private IPv4 addresses. Loopback remains accepted for focused
boundary fixtures; production Base still has to prove the actual original
namespace endpoint. Hostnames, credentials in URLs, paths, query strings and
alternate URL spellings are rejected by Rust's canonical origin check.

Hermes journal preparation and submission require the exact sealed origin and
the current launch ID in the private `fleet_launch` capability. The immutable
request, credential fingerprint, once-only submission permit and terminal/ACK
guards remain unchanged. The supervisor also rechecks original Base custody and
origin before the actual request. Native/legacy free chats retain the original
localhost-only rule and cannot use the container path as a fallback.

Endpoint history survives stop and replacement; a new namespace receives a new
launch and endpoint row. No transcript, task ownership or runtime SQLite is
rewritten. Update, delete and truncate are prohibited. Downgrade refuses retained
endpoint history or non-localhost dispatch journals rather than erasing evidence.

## Consequences

Actual container chats no longer need a fake localhost origin. Database origin
authority and runtime custody checks agree on the same generation. A missing or
changed endpoint holds dispatch; it never authorizes a fresh unknown submission.
No public API/client schema or deployment flag changes are needed.

Additional database writes/readbacks occur during container endpoint resolution.
The primary key is the launch ID, and dispatch joins the already indexed current
agent launch. Actual controller restart/takeover, private-journal loss, logs,
Docker rollback and task/PM admission remain separate acceptance requirements.

## Alternatives

- Allow arbitrary private IPs in the journal: rejected; address shape is not
  authority for an original runtime.
- Publish every agent port on the host and keep localhost: rejected; that weakens
  isolation and does not match the selected per-agent private Docker networks.
- Store origin only in frontend or mutable runtime status: rejected; replay and
  control require immutable generation-bound identity.
- Rewrite the already published localhost migration: rejected; existing ledgers
  require a separate additive release migration.
