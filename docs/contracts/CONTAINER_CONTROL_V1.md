# Private Container Control v1

Status: private client and opt-in supervisor routing are implemented in the
integration candidate. Automated Compose preparation, installed enablement and
live Fleet/Hermes acceptance remain incomplete. Native execution is not
silently relabelled Docker.

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
private stop journal. Endpoint is a read-only version-2 command: it requires
an original running ACK, the same bridge/endpoint ID, an RFC1918 IPv4 address
and a second matching readback. It never resolves an arbitrary caller URL.
Base response envelope is closed and action-matching;
typed registration/snapshot/receipts reject unknown fields and unknown states.

Never-started readback is `registered/never_started/null snapshot`, not a start
ACK. `held/unavailable/null snapshot` remains unresolved, never a permit to adopt
a fresh PID. Successful ACK/exit uses the original container/resource/generation,
Engine identity, sealed expected running inventory and network hash. A positive
stop requires `observed/namespace_exited`; HTTP failure or EOF cannot replace it.

Register does not start a model. Before start, Fleet must commit the original
registration plus immutable agent/config/launch binding in its database. The
client alone does not enforce this transaction or provide Tracker/Workflow
admission. The supervisor now commits this binding before invoking start.
Its Rust methods are controller-internal, not public HTTP API.

## Supervisor Binding

Operator configuration `fleet.container_control` selects Docker without native
fallback. The controller currently reads an operator-prepared mode0600 file
`<controller_root>/<agent_uuid>.container-prepared.json`, with closed fields:
agent_id, paths, api_port, configuration_revision, configuration_sha256, container.
Container fields are registration, policy, compose, journal, stop_journal,
source_sha256 (three Base file hashes) and context. No public API accepts this
document. Controller storage is an existing mode0700 directory outside agent
storage; links, overlapping roots, foreign ownership and relative paths fail.

The registration resource equals the immutable agent UUID; generation equals
the Fleet launch UUID. Compose project is sdlc1/sdlc2 or an owned QA project.
Exactly four bind mounts correspond to this agent's paths: /runtime read-only,
/config, /workspace and /logs writable. Paths refer to the same filesystem seen
by the daemon; translated Docker Desktop/controller-container mount roots are
not yet supported and must not be guessed. Base rechecks real inventory.
Before reading preparation, Fleet checks all four original directories under
the configured ordinal root and the original agent marker; symlink/junction
components, foreign markers and missing directories cannot reach Base start.
Receipt hashes use sorted nested JSON; Engine text is bounded ASCII, matching
Base's ASCII canonical wire representation.

The optional private `container` member is persisted in existing immutable
runtime_launches.binding JSON. Historical native bindings omit it byte-for-byte;
there is no new migration or public API schema. Source hashes/context, effective
revision/hash and original registration are pinned before start. A claimed
launch becomes Starting/desired Running before the protected command. An unknown
ACK leaves the claim open; another start cannot submit it again. The same
controller may finish a DB ACK only by reading Base's original durable receipt.
Controller restart/takeover remains held, not automatic adoption.

All Hermes HTTP paths, including acceptance/control/PM readback, use the original
container endpoint. HTTP readiness is separate from namespace identity. Stop
uses a stable generation-bound operation key and original snapshot hash; only
positive namespace exit closes the DB launch. A stopped generation is never
restarted: restart needs a newly prepared generation. Docker configuration
activation is deliberately held before file changes until replacement creation
and rollback are implemented; native gateway exit is not used as substitute.

## Remaining Wiring And Acceptance

Required next integration: guarded single-agent Compose rendering/create,
credential rendering/isolation, explicit trusted Fleet bridge access,
container stdout/stderr capture, controller takeover and loaded-generation
attestation, plus receipt-based configuration drain/activation/rollback. The
current prepared-container consumer is not automatic provisioning. Java control remains
phase2. No public OpenAPI, database migration, SDK pin or installed image changes
are implied by the client.

Base native protocol QA verifies two synthetic authenticated HTTP containers,
not this Rust client or actual Hermes. Fleet client unit/build evidence is
recorded in [verification](../CHAT_CLARIFICATION_VERIFICATION.md): Rust1.88 fmt,
workspace all-target check/strict Clippy and six client units pass. Neither
substitutes for real container supervisor, PM,
seven-agent SDLC or deployed application acceptance.
