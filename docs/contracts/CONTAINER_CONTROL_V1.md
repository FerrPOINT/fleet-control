# Private Container Control v1/v2

Status: private client and opt-in supervisor routing are implemented in the
integration candidate. Automatic generation preparation is connected
behind private deployment configuration. Real Rust/Docker/Hermes controlled-model
free chat, isolation, drain/replacement and restart pass the
[owned gate](../CONTAINER_SUPERVISOR_ACCEPTANCE.md), including the explicit
controlled candidate-readiness-timeout rollback. Installed enablement,
controller recovery and Docker log ingestion remain incomplete.
Native execution is not
silently relabelled Docker.

## Original Controller Restart Observation

Base candidate
[9171fe6b1b6b05b9504d33fb881f274c0b5541e0](https://github.com/FerrPOINT/services-base/commit/9171fe6b1b6b05b9504d33fb881f274c0b5541e0)
adds private protocol2 `observe_controller_restart`. Only original mapped
policy3/registration/files are accepted. A fresh Engine observation must prove
a distinct `StartedAt` of the same controller ID, immutable inventory, Engine,
named volume and mount projection. Base reads the original agent ACK twice and
does not rewrite mapping/registration/SQLite, attach, claim, start or stop.

The closed result contains `contract_version=1`,
`state=controller_restart_observed`, `original_mapping_sha256`,
`registration_sha256`, `original_controller_snapshot`,
`current_controller_snapshot` and original `receipt`. Fleet checks both hashes,
exact original snapshot, same controller/inventory, valid distinct start and an
acknowledged original running/exited namespace. A recycled controller PID is
allowed only with distinct Engine start; PID-only change is denied. Extra/missing
fields, unknown ACK, source drift and unmapped legacy requests remain held.

The restarted supervisor may report this witness as degraded health only when
the fresh immutable DB launch/ACK/PID remain exact before/after the read. It does
not cache ownership, write runtime state, release capacity or call Hermes. All
mutating lifecycle and dispatch paths still require original controller custody.
This is **not ownership transfer or complete crash recovery**. New controller
ID/source upgrade, lost storage, interrupted activation, durable ownership fence
and actual Fleet/Base/Docker restart acceptance remain separate gates. The SDK
and installed source pins are unchanged; older source rejects this action.

See [verification](../CHAT_CLARIFICATION_VERIFICATION.md#original-controller-restart-observation-7-october-2026)
and [operations](../OPERATIONS.md#controller-restart-observation).

## Private Log Tail Candidate

The Rust client adds `log_tail` for the versioned Base `logs` operation. It uses
the same captured-source hashes, explicit context, original registration and
mapped-file guards. The response must contain an acknowledged original running
or exited receipt and exactly two valid base64 streams, at most 32 KiB together.
Never-started, held, foreign, extra-field, malformed or oversized responses fail
closed. Raw bytes deliberately have no `Debug` or public DTO implementation.

This is controller-private diagnostic transport, not production log ingestion.
Callers must still establish current launch custody and redact resolved credentials
before persistence/API/audit. It is not wired to health polling or public logs yet;
existing pinned Base versions reject the unknown action without fallback. Bounded
tail reads do not certify durable pagination, complete history or rotation recovery.
The actual Base diagnostic gate reads four acknowledged exited originals from
two real Hermes agents without persisting raw output; see
[evidence](../CHAT_CLARIFICATION_VERIFICATION.md#actual-private-base-docker-log-readback-6-october-2026).
That probe calls Base directly, not the Rust client. Actual Rust ingestion,
redaction and durable generation-bound collection remain open.

## Private Source Pages

Rust `log_page` sends the existing protected request with `action=log_page`,
original `registration` and closed `cursors={stdout,stderr}`. Each cursor has
`offset` (integer from 0 to 64 MiB) and lowercase 64-character `sha256`; offset 0 requires
the SHA256 of empty bytes. No caller-supplied container name, file or command
is accepted. Source/context/mount guards remain the original launch's.

The closed result is `{receipt,streams:{stdout,stderr}}`. Each stream has
`start_cursor`, `next_cursor`, `source_bytes`, boolean `has_more` and
`body_base64`. Start must equal the requested cursor, body must equal the
bounded available range (maximum 16 KiB), next offset advances by exactly its
decoded length and both source lengths together cannot exceed 64 MiB. Initial
digest is recomputed locally; a noninitial digest is verified against the actual
prefix by the pinned Base reader. Empty polls preserve the exact cursor.
Raw binary page types have no Debug/public serialization. Receipt must identify
the same acknowledged running/exited namespace; held, drift or malformed data
returns reconciliation, not empty success.

Base checks the original receipt before/after its concurrent bounded scan and
requires an explicitly inspected blocking non-rotating non-TTY json-file source.
The current generated Compose allowlist does not admit that logging profile;
production source admission needs a bounded-storage/retention contract first.
Do not enable unlimited logging to bypass this gap. `has_more=false` is current
snapshot EOF, not runtime or SDLC completion. No cursor/DB commit, secret masking,
production ingestion, installed pin or public API is changed by this client.
Eight component cases include a real Unix subprocess with pinned wire fixtures;
they are not the actual Rust-to-Base/Docker/DB collection gate.

## Ownership

The configuration-replacement candidate checks phase/revision/hash before
preparation and again before launch. It stops the original namespace before
rewriting shared agent files; activation and rollback use new generations.
Unknown preparation or start blocks rollback file mutation/spawn and retains
the private activation journal. Stopped config readback is not runtime readiness.
See [ADR0029](../adr/0029-container-configuration-generation-replacement.md);
31 focused Linux/PostgreSQL cases and strict Clippy pass. Full workspace fmt,
check, strict Clippy and523 cases pass with29 explicit ignores in the earlier
configuration packet. Its fake-boundary rollback evidence is not actual Docker
failure-injection acceptance. The new real gate is recorded separately above.

Fleet's additive000019 now seals the Base-verified endpoint to the original
started launch/PID before use. Dispatch requires the exact sealed origin and
private launch ID, not merely a private IP address. This is a Fleet database
guard, not a changed Base wire protocol or user-facing endpoint setter. See
[ADR0030](../adr/0030-generation-bound-container-endpoint.md).

Automatic preparation additionally uses Fleet's000018 pre-create DB fence,
before private intent creation and Base prepare. It commits original agent/
history ordinal/controller/generation/operation/intent hash, not the resolved
environment. Missing private files cannot authorize a new generation or native
fallback. The Base protocol is unchanged by this Fleet-private additive table;
same-controller exact restore/readback is not controller takeover. See
[ADR0028](../adr/0028-durable-container-precreate-fence.md).

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

Protocol version1 carries Base boundary policy version1 or2. Mapped lifecycle
uses protocol2 with boundary policy3 and requires the original mount_mapping
plus private mapping_file on every command. Resolve_mounts itself remains a
read-only protocol1 prerequisite using the original local policy2. Common fields are
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

### Named-Volume Consumer

Fleet's opt-in automatic creation now resolves the exact configured Fleet
backend and agents_root through Base. The closed typed receipt validates the
original controller/image/service, snapshot, Engine, volume metadata digest,
four ordered areas of one agent and the canonical original local-policy hash.
Daemon bind projections are proof only: launch policy3 uses the original named
volume with exact agentN/runtime, config, workspace and logs subpaths. Runtime
is readonly; agent containers receive neither the volume root nor Docker socket.

The exclusive creation intent stores the original mapping and generation-specific
mapping-file path before prepare. An identical retry compares fresh readback with
that proof; it cannot adopt another controller, PID, volume or recipe. Base owns
the immutable mode600 mapping file and seals its canonical digest in registration.
Prepared documents and DB launch bindings carry the same mapping/path/hash;
registration Engine must equal the original mapping Engine. Only bridge-ID
allocation may differ from the original local policy. Removing either mapping
field, changing local AgentPaths or downgrading protocol/version fails closed.

Prepare/start/observe/endpoint/attachment/stop forward protocol2 with this original
proof. Base revalidates controller/Engine/volume and local filesystem guards on
every effect/readback. Fleet also rechecks current controller/root configuration
and private storage paths. A missing/changed proof file or controller restart
remains a reconciliation hold, not takeover or permission to resend. Existing
native/host-bind records omit the new fields unchanged; automatic creation with
a configured bridge controller cannot upgrade an old unmapped intent in place.
The mapped transport itself leaves the Base SDK pin, default deployment flags,
public API and existing migrations unchanged. The later pre-create fence uses
separate additive000018; it does not rewrite transport/launch history. See
[verification](../CHAT_CLARIFICATION_VERIFICATION.md) for
the Rust gate and the still-required actual mapped Hermes/model/config acceptance.

### Base Prerequisite Evidence

Current Base sourcee083651 / candidate98a5bbd implements the next layer:
private protocol2 + boundary policy3 uses external named-volume subpaths,
not bind sources inside Docker's own data root. Each mount has type=volume,
source=original volume name, subpath=agentN/area, destination=/area and RO flag.
The immutable mapping_file precedes create; preparation/registration seal
mount_mapping_sha256 and all effects/readbacks revalidate the original proof.
Old protocol1 cannot service mapped registrations. Native own-volume lifecycle
0a1f2bdd97e1 passes with UID999, no socket, local symlink denial and exact cleanup.
The read-only-only evidence below is historical. The Rust consumer above adds
typed policy3 and mapping/file/hash binding; this does not prove installed
acceptance. Do not replace binds with rslave.

Base adds read-only resolve_mounts with explicit controller/local_root alongside
the common policy/compose/journal/context. Exact controller inventory and Engine
are read twice; the mounted root must be one writable local named volume of the
same project, without options or nested/aliased mounts. Existing local directories
and symlink/private-root guards precede and follow readback. Four projected bind
sources identify one agentN, not sibling or private controller storage.

The receipt contains state=resolved, controller/snapshot/engine/local_root,
volume_name/volume_sha256, mounts and input_policy_sha256. It never carries raw
inspect/env or writes a journal. It is a private prerequisite, not a launch permit.
The Rust consumer now invokes it and binds the original proof in creation intent;
mapped lifecycle validation preserves local guarded-path checks. Do not pass
the returned daemon paths to existing local guarded_mount_sources and pretend
the namespace gap is closed. Installed opt-in and full runtime remain held.

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

New intents additionally contain optional private `environment_snapshot` with
closed `version=1`, `dotenv` and `dotenv_sha256` fields. Present input is the exact
guarded UTF8 file, not a re-parsed environment; absent file uses both nullable
fields, while an empty file retains the SHA256 empty digest. Reads reject links,
nonfiles, hardlinks, invalid UTF8 and input beyond64 KiB. A revision-bound launch
requires the current file to equal that revision's rendered `.env`; its snapshot
identity is rechecked against the requested configuration hash. The complete
serialized intent must still fit the existing64-KiB private-file limit before
DB claim. Original bytes/hash are frozen by that same precreate intent fence.

Retry and prepared-start checks include this input snapshot. File drift or
deletion cannot create/start an obsolete recipe; the original private document
remains unchanged. Historical intents lacking the optional field are not
backfilled and remain unsupported as original dotenv evidence for collection.
Deleting the field from a claimed new intent still violates its DB hash. This
does not prove which secrets Hermes actually loaded: dotenv interpolation,
ASCII sanitization, external sources and managed overlays require separate
native evidence and resolved-value custody before production redaction.

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
