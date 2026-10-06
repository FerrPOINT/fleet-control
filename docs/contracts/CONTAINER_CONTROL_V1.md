# Private Container Control v1

Status: private client and opt-in supervisor routing are implemented in the
integration candidate. Automatic generation preparation is connected
behind private deployment configuration; installed enablement and live
Fleet/Hermes acceptance remain incomplete. Native execution is not
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
The follow-up candidate captures each source once (maximum1 MiB), rejects
symlink/junction components from the filesystem root, and hashes the captured
bytes. The private child wrapper carries exactly those three sources as base64
plus the original request. Its fixed loader compiles those bytes in dependency
order; it never rereads checkout modules or accepts cached bytecode. The synthetic
scripts package has an empty search path: package initialization and unpinned
checkout imports are unavailable. Python runs with isolated mode and bytecode
writes disabled; `-B` alone would not prevent cached-code reads. The standard
library and selected interpreter remain operator-trusted, not agent-supplied.
The bootstrap does not inherit PYTHONPATH or model/provider secrets.
Docker context is explicit; conflicting host/TLS env
is passed to Base's rejecting guard, not silently used as a fallback endpoint.

Only structured stdin is used, never a caller-provided shell command. The original
Base request and both output streams remain bounded to64 KiB. The Fleet-only
source wrapper is separately bounded by three1-MiB captures plus base64/JSON
encoding; it is not a new Base/public protocol. Binary stdin is restored for
Base's existing request reader. Overall deadline remains60 seconds. A
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

Prepare adds process/operation_id/creation_compose/creation_journal. Base renders
one immutable generation-specific service and owns its bridge creation. Fleet
validates a closed prepared receipt and permits only the network ID to change
from the unallocated zero sentinel; all original policy fields and registration
hash/identity must match. A held receipt cannot reach DB claim/start. Base commits
an original payload/Engine claim before its one Compose create. An unknown create
may recover only a never-started exact generation through readback; absence stays
held without another create. This is preparation, not health or execution admission.

Before writing creation intent or calling prepare, Fleet permits agent projects
only in sdlc1/sdlc2 or an owned valid sdlc-qa-* project. Shared infrastructure,
demo/build names and malformed QA suffixes are rejected before effects. Java
with Docker configuration fails with existing Unavailable503; native fallback
is forbidden. The existing Java native lifecycle is available only without
Docker configuration and is not container or SDLC acceptance.

## Supervisor Binding

### Trusted Fleet Bridge Attachment Candidate

Private optional `fleet.container_control.bridge_controller` contains full
`container_id`, immutable `image_id` and exact Compose `service` (`fleet-backend`
or `fleet-control-backend`). It is operator configuration, not agent/user input.
Fleet calls Base `attach_controller` after preparation, before DB launch claim
and protected start; it revalidates the same attachment before resolving any
Hermes HTTP endpoint. Missing option retains explicit host/operator networking,
not automatic discovery of a running Fleet container.

The automatic creation intent pins the controller selection; drift conflicts
before launch. A generation-specific private attachment SQLite journal lives
outside agent storage. Base seals original controller PID/start/inventory, Engine,
registration and bridge; only that running same-project trusted Fleet backend
is attached. Unknown acceptance cannot repeat connect; exact positive membership
readback can recover the original claim. External membership is not adopted;
controller replacement/restart holds for operator reconciliation. Agent topology
stays one private bridge, without host ports/socket or sibling mounts.

This candidate does not implement daemon-path translation, controller takeover,
container logs/config activation, PM admission or model/chat acceptance. Native
two-Hermes trusted-controller HTTP and Rust consumer gates must be recorded
separately; fixture receipts cannot certify the installed Fleet backend.

Operator configuration `fleet.container_control` selects Docker without native
fallback. The controller reads a private mode0600 file
`<controller_root>/<agent_uuid>.container-prepared.json`, with closed fields:
agent_id, paths, api_port, configuration_revision, configuration_sha256, container.
Container fields are registration, policy, compose, journal, stop_journal,
source_sha256 (three Base file hashes) and context. No public API accepts this
document. Controller storage is an existing mode0700 directory outside agent
storage; links, overlapping roots, foreign ownership and relative paths fail.

When the deployment has `container_control.provisioning`, a missing document
triggers automatic preparation. Fleet persists an exclusive mode0600
`<agent_uuid>.container-creation.json` before calling Base. It contains the original
agent/paths/revision, generation/operation UUIDs, policy/process and source/context
pins; resolved secrets never appear in Debug/public DTO/DB receipts. Restart after
a failed prepare uses that same intent, not a new UUID. Changed revision, credentials,
paths or operator process settings conflict before another create. On success Fleet
writes the prepared document, then separately commits the immutable DB launch before
sole start. Existing operator-prepared documents remain compatible when provisioning
is absent. Partial files fail closed and are never overwritten.

Before claiming a previously prepared automatic generation, Fleet reconstructs
the current recipe using the original generation/operation IDs and compares its
complete process, derived token, policy, paths, revision and source/context with
the saved creation intent. It then validates the prepared registration/policy
against that intent, allowing only Base's originally allocated network ID.
Missing intent or recipe/credential drift blocks before observe, launch claim or
start; it never edits the old files or silently creates another generation.
Disabling automatic provisioning does not turn an existing automatic intent into
an operator-prepared compatibility launch. Historical operator documents without
an automatic intent retain their explicit trusted-operator path. This guard is
not proof of loaded configuration, current model credentials or agent readiness.

After an original confirmed namespace exit closes the launch, the next candidate
uses the count of immutable agent launch-history rows as its preparation ordinal.
The initial ordinal0 retains the filenames above; later ordinals use
`<agent_uuid>.<ordinal>.container-creation.json` and the corresponding prepared
filename. One database snapshot counts history and rejects any outstanding
claimed/started launch. No timestamp, filesystem reset or mutable head pointer
selects a new generation. A create uncertainty before DB claim stays on the same
ordinal/intent. A start uncertainty retains the open DB launch and cannot create
the next ordinal. Old files/containers are preserved, never restarted or deleted.
Operator-prepared compatibility generations must use the current ordinal filename;
new-generation Docker restart remains subject to live acceptance.

Process uses HOME/HERMES_HOME=/config, cwd=/workspace, serve host0.0.0.0 inside its
sole bridge, derived per-agent token and image-owned explicit entrypoint. No ports
are published. Renderer v2 writes container-visible config paths/address in Docker
mode; native mode retains its loopback/address/layout. This is not certification
of UID/file permissions, controller bridge connectivity or loaded revision.

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
restarted: restart prepares a new generation after confirmed exit. Docker configuration
activation is deliberately held before file changes until replacement creation
and rollback are implemented; native gateway exit is not used as substitute.

## Remaining Wiring And Acceptance

Required next integration: trusted daemon/controller mount mapping and UID/file
access checks, actual Hermes container acceptance, explicit trusted Fleet bridge access,
container stdout/stderr capture, controller takeover and loaded-generation
attestation, plus receipt-based configuration drain/activation/rollback. The
generation preparation does not complete the container lifecycle. Java control remains
phase2. No public OpenAPI, database migration, SDK pin or installed image changes
are implied by the client.

Base native protocol QA verifies two synthetic authenticated HTTP containers,
not this Rust client or actual Hermes. Fleet client unit/build evidence is
recorded in [verification](../CHAT_CLARIFICATION_VERIFICATION.md): Rust1.88 fmt,
workspace all-target check/strict Clippy and six client units pass. Neither
substitutes for real container supervisor, PM,
seven-agent SDLC or deployed application acceptance.

The newer captured-source loader has six host behavioral checks, including a
demonstrably valid poisoned cache and post-capture file replacement. Its Linux
Rust source/hash/path/size regressions and the final502-case workspace gate pass,
including project/Java pre-effect guards and history-ordinal restart. Previous
six client units and494-case evidence cover only older bytes; see the latest
verification section. Real Fleet/Docker/Hermes acceptance remains required.
