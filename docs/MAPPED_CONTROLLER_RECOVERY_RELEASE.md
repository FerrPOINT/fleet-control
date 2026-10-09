# Mapped-Volume And Controller Recovery Source Release

This independent unit extends `333c06d9557dc46e65cff88aa8bc8f6208ce4c1c`
with the original Base named-volume protocol and fenced controller restart
recovery. It is not production runtime acceptance. Automatic provisioning,
generation replacement/restart and configuration activation remain required
follow-on units; operator preparation is only a compatibility entry point.
`runtime_ready` remains false.

## Dependencies And Boundaries

- One new migration: `m20261009_000016_mapped_controller_recovery`. It extends
  the existing 000015 snapshot constraint and adds a private recovery journal;
  historical migration files and original journal12 guards are unchanged.
- Base utility169 remains `ae8af2342b61090094292e75a7c23bf464757468`;
  its three captured-source hashes are unchanged from 000015. Utility source
  `dc43d0e25d60afa073c201e74d1a2cfe9aab8939` has the same bytes.
- Rust/UI SDK remains `19a7a381ae6dbea61a643bb96189e483fa64df5c`.
  No Base, Tracker, Workflow, generated API, UI, lockfile or pin changes.
- Read-only donor references: mapping `d924799be5ec77935b71decce20059f22b919e10`,
  fenced recovery `5970cdeeff5663d201d2f394e759d29932b07700`, and later donor
  exact-delivery/readback patterns. Only private protocol validation is reused.
  The donor's runtime-launch17/recovery20/21/22 tables are not imported.
  No control13 or approval14 dependency is introduced.

## Original Mapped Custody

Trusted `fleet.container_control.mapping_controller` adds an explicit
`container_id`, immutable `image_id` and `service` (`fleet-backend` or
`fleet-control-backend`). These must match the original saved Base mapping;
an agent, HTTP request or prepared document cannot select another controller.
The existing Linux private root, utility-source isolation and 0700/0600 guards
remain mandatory. Agents receive neither Docker credentials/socket nor private
controller storage.

The private `ContainerBinding.mapped` contains the original `mapping`,
`mapping_file`, `attachment_journal` and `recovery_journal`. All files are distinct
direct children of the controller root. Mapping is a retained canonical Base
receipt, not a new host-path or endpoint authority. Registration v3 seals its
exact `mount_mapping_sha256`. Optional fields are omitted for historical v2
bindings so existing serialized identities remain unchanged.

Fleet validates the local recipe hash, daemon identity, trusted controller
inventory and physical snapshot, original volume fingerprint and exactly four
ordered areas of one `agentN`. Agent launch mounts are named-volume subpaths,
never projected daemon bind paths. `/runtime` stays read-only; `/config` is both
HOME and HERMES_HOME; cwd is `/workspace`. Existing ownership-marker, API key,
port, effective revision/hash and private Compose checks still apply.

Before start, the existing Fleet claim is committed and Base's original
`attach_controller` journals the sole bridge attachment. Unknown attachment is
not blindly repeated or replaced. Initial endpoint readback requires that exact
attachment receipt; the saved origin remains the original agent bridge address.
After controller recovery, Base protocol3 verifies original custody and both
lease/generation guards for observe, endpoint and physical stop. No new network,
volume, registration, origin or container is adopted during recovery.

## Recovery And Crash Windows

The startup/periodic worker runs only for mapped, acknowledged `running` or
`stopping` generations. Base must prove a different `StartedAt` of the same
immutable controller container, unchanged private inventory, original volume
and original agent namespace snapshot. A changed agent PID/StartedAt, daemon,
controller container/image, mapping file or volume retains a hold.

Fleet commits the exact recovery command before native delivery. Each epoch
binds original controller UUID, current process UUID, launch identity hash,
mapping/registration hashes, original agent PID and physical controller snapshot.
The launch hash excludes only mutable lifecycle state; original prepared data,
snapshot, origin and stable stop UUID stay sealed. A second claimant is fenced.
The original launch/controller and Hermes journals are never rewritten.

The full Base recovery ACK is retained immutably. Historical ACK readback does
not grant live ownership. Fleet separately claims each heartbeat command before
Base, retaining its original heartbeat ACK and a guarded original observation.
Database deadline and Base Linux boot-clock lease must both be live. Each lease
is at most 30 seconds; the worker attempts renewal every five seconds. Slow or
unavailable native calls fail closed, not into a grace-period permit.

Lost recovery ACKs use the frozen command: the same process may invoke Base's
idempotent delivery; another process can only read its original receipt. A lost
heartbeat ACK can only use its exact original version/deadline, never extend a
dead owner's lease. Unaccepted/ambiguous expired deliveries remain held.
An expired acknowledged owner needs the exact predecessor and another physical
controller restart before a new epoch. The budget is 1024 epochs per generation.

Recovery claims immediately fence the original owner and journal admission.
Unacknowledged heartbeat or expired lease closes admission again. Repository and
DB transition guards require the exact current recovery lease for stop/exit.
The worker resumes a durable `stopping` intent using its original stop UUID;
success requires Base's original physical namespace-exit receipt. Unknown runs,
draining and unknown dispatch acceptance still hold capacity. Recovery does not
release runs, repeat POSTs, rotate original credentials or bypass origin checks.

## Verification And Handoff

Mandatory CI selects the existing seven client and four lifecycle/path tests,
five new mapping tests, five new recovery/attachment receipt tests, five PG
controller cases (two existing, three mapped/recovery), and separate one-case
000015/000016 migration suites. Exact passed counts make missing selectors fail.
Lineage tests retain canonical and split histories and the task-chat downgrade
guard with one appended migration. Empty 000016 downgrade restores the original
snapshot constraint and preserves v2 history; mapped/recovery history blocks it.

Passed local source-only checks: nine Python loader/README tests, README validation, rustfmt,
offline locked Cargo metadata, SDK revision verification, static Base Docker
invocation gate and diff checks. Rust compilation/unit tests, PostgreSQL and
Docker acceptance belong to parent-owned exported-source QA, not this checkout.
No build, target/cache preparation, runtime/container operation, push or deploy.

Parent runtime QA must verify two agents, original subpath mounts, private-storage
isolation, initial attach/start/readiness, repeated same-container controller
restart, dual claimants, lost ACK/heartbeat/stop delivery, original endpoint and
namespace identity, lease expiry and physical stop through Base ComposeHelper v2.

## Real Remaining Dependencies

- Automatic preparation needs a separate pre-create durable intent and original
  Base `prepare`/`reconcile_preparation` receipts, including the exact immutable
  image, local recipe, volume mapping, generation and credential context. No
  migration17/18 or preparation behavior is bundled into 000016.
- Automatic restart/replacement must allocate a new never-started generation,
  preserve capacity/unknown holds, and reconcile old physical exit before reuse.
  Restarting a registered exited container is not supported by Base's contract.
- Configuration activation requires drain/revision claims, guarded file writes,
  new-generation preparation, physical readiness and compensation/recovery.
  It still returns unavailable before file mutation in Docker mode.
- Process-only controller restarts without a distinct Engine `StartedAt`,
  replacement controller containers and launches missing the original Fleet
  start ACK are held; this unit cannot manufacture their custody.
- Docker log collection/replay, task/PM admission, loaded skill/config inventory,
  model and deployment readiness remain separate unfulfilled gates.

These are dependencies of the production target, not a production completion
claim based on an operator-prepared container.
