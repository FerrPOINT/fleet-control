# Private Container Control v1

Status: private client implemented in the integration candidate; supervisor
routing, authoritative Docker launch binding and live Fleet/Hermes acceptance
remain incomplete. Existing native start/stop is not silently relabelled Docker.

## Ownership

Fleet owns agent identities, authorization, configuration and runtime lifecycle.
Base owns the shared Compose boundary and private control utility. There is no
new user-facing service, HTTP controller or business scheduler. One agent uses
one container in the existing sdlc1/sdlc2 project. QA uses an owned temporary
Compose project. The Docker socket is never available inside agent containers.

## Transport

`infra::runtime::container_control::ContainerControl` invokes isolated Python
with a fixed bootstrap and an operator-selected Base root. Before every call it
checks SHA256 of runtime_boundary.py, runtime_bootstrap.py and runtime_control.py.
The bootstrap skips package initialization and does not inherit PYTHONPATH or
model/provider secrets. Docker context is explicit; conflicting host/TLS env
is passed to Base's rejecting guard, not silently used as a fallback endpoint.

Only structured stdin is used, never a caller-provided shell command. Request
and both output streams are bounded to64 KiB, overall deadline60 seconds. A
timeout/unknown exit yields reconciliation-required, without resend. Native
stderr/inspect/env are never returned through AppError. The common subprocess
is private backend code, not an agent tool or publicly configurable command.

## Commands And Receipts

Protocol version1 carries Base boundary policy version1 or2. Common fields are
action/context/policy/absolute compose and journal paths. Register additionally
binds original full container ID and operation UUID. Start/observe carry the
original registration. Stop adds its original operation UUID and separate
private stop journal. Base response envelope is closed and action-matching;
typed registration/snapshot/receipts reject unknown fields and unknown states.

Never-started readback is `registered/never_started/null snapshot`, not a start
ACK. `held/unavailable/null snapshot` remains unresolved, never a permit to adopt
a fresh PID. Successful ACK/exit uses the original container/resource/generation,
Engine identity, sealed expected running inventory and network hash. A positive
stop requires `observed/namespace_exited`; HTTP failure or EOF cannot replace it.

Register does not start a model. Before start, Fleet must commit the original
registration plus immutable agent/config/launch binding in its database. The
client alone does not enforce this transaction or provide Tracker/Workflow
admission. Its public Rust methods are controller-internal, not public HTTP API.

## Remaining Wiring And Acceptance

Required next integration: guarded single-agent Compose rendering/create,
private mount/credential isolation, explicit trusted Fleet bridge access,
pre-exec DB binding, start readiness, original stop/health/logs reconciliation,
and receipt-based configuration drain/activation/rollback. Java control remains
phase2. No public OpenAPI, database migration, SDK pin or installed image changes
are implied by the client.

Base native protocol QA verifies two synthetic authenticated HTTP containers,
not this Rust client or actual Hermes. Fleet client unit/build evidence is
recorded in [verification](../CHAT_CLARIFICATION_VERIFICATION.md): Rust1.88 fmt,
workspace all-target check/strict Clippy and six client units pass. Neither
substitutes for real container supervisor, PM,
seven-agent SDLC or deployed application acceptance.
