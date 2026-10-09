# Bounded Docker Lifecycle Source Release

This release adds real, opt-in Compose start and physical stop to the existing
Fleet supervisor. It starts an operator-prepared, never-started original Base v2
container. It does not install a new host controller or change runtime images.
Process Hermes and the Java jar lifecycle keep their existing defaults.

## Exact Dependencies

- Fleet parent: `cb720d7258294ca5d71c7f586a86f7407e9201b1` (journal12).
- Rust/UI SDK: `19a7a381ae6dbea61a643bb96189e483fa64df5c`, unchanged.
- Separate Base utility169: `ae8af2342b61090094292e75a7c23bf464757468`.
  Its three utility files also match source `dc43d0e25d60afa073c201e74d1a2cfe9aab8939`.
- Reference introduction: Fleet `c0ad3b745fd22526b62f751f8b192009a01f7839`.
  Only the private v1/v2 subprocess/receipt protocol is reused; the loader executes
  captured hash-checked UTF-8 sources instead of importing mutable checkout files.
- One new migration: `m20261009_000015_container_controller`.
  No control13, approval14, runtime-launch17, preparation18 or migrations16..22
  are required. Historical migration files are unchanged.

`000015` adds `runtime_container_launches` and extends the journal12 origin guard
at its exact original expression. The rest of the accepted trigger is preserved.
Existing journals, credential fingerprints, exact requests, ACKs and origins are
never rewritten. Empty downgrade restores the original function and constraint;
any container history blocks downgrade, including exited generations.

## Operator Contract

Docker is disabled unless trusted server configuration defines
`fleet.container_control` with `python`, `base_root`, `context`, and
`controller_root`. The Python executable and explicit Docker context are operator
authority. Host/TLS environment overrides are rejected by Base. `base_root` must
contain the three utility169 sources exported as canonical Git blobs with the
compiled hashes, not CRLF checkout copies. The unit16 follow-up corrects those
hashes without repinning historical bindings; old mismatched generations remain
held. See [the follow-up evidence](MAPPED_CONTROLLER_RECOVERY_RELEASE.md#bounded-correctness-follow-up-from-fc2e27b).
It is independent
of the adjacent SDK checkout. No user/agent request supplies a Docker endpoint,
utility source, Compose file or container identity.

This bounded version requires a Linux Fleet process with direct access to the
same guarded host paths as its selected Linux Docker daemon and the original
container bridge. Running Fleet inside an umbrella container with translated
named-volume paths requires the later mapped protocol and is unsupported here.

Provision the four `agentN` areas and their existing Fleet ownership marker
through the normal provisioner. A trusted operator uses Base preparation to
create/register one immutable-image Compose service in `sdlc1`, `sdlc2`, or an
owned temporary `sdlc-qa-<task>-<unique>` project. The service has exactly four
bind mounts: read-only `/runtime`, writable `/config`, `/workspace`, `/logs`.
Base verifies the full private inventory, bridge identity, isolation, limits,
labels, immutable Compose specification and original never-started namespace.
No direct Docker run/create is used.

Store the original Base policy, registration and private file paths as
`<controller_root>/<agent_uuid>.container-prepared.json`, matching the private
`PreparedContainer` Rust schema. Include exact agent paths, API port, effective
configuration revision/hash, utility hashes and context. This is a private local
operator document, not a new signed public receipt or an endpoint authority.
Keep original journals and receipts; never re-register a started container.

The controller directory must already exist, mode `0700`, owned by Fleet, outside
agent and utility roots. Prepared/Compose files require `0600`, a single hard
link, bounded size and no symlink/junction components. Journals remain private.
The process requires `HOME=HERMES_HOME=/config`, cwd `/workspace`, its original
derived `API_SERVER_KEY`, headless server mode and the exact managed API port.
Agent mounts never include controller storage, Docker socket or host credentials.

## Lifecycle And Holds

Fleet commits one immutable generation/controller claim before Base Compose
start. Original registration, daemon, inventory, network, container ID,
`StartedAt` and init PID bind the ACK. The original observed bridge endpoint is
persisted once. HTTP health/capabilities must pass within a bounded readiness
deadline, followed by another physical generation check. Readiness failure tries
the original physical stop and reports success only after a verified exit.

Stop first commits a stable stop UUID and stops dispatch admission; Base stops
the original namespace using its retained snapshot. Missing/foreign receipts,
unknown starts/stops and a different controller process retain holds. Health
observes an original namespace exit without releasing unresolved run capacity.
There is no native fallback after container history, even if opt-in is disabled.
An exited container cannot be restarted under its old registration; restart
requires a newly prepared never-started generation after capacity is reconciled.

Free-chat dispatch retains journal12's one-POST permit and original credential
fingerprint. Capabilities additionally seal `fleet_container_generation`. Both
transactional prepare/claim and HTTP dispatch/readback/control verify the original
origin/generation. Unknown acceptance remains occupied. Existing signed protocol
and credential receipts are preserved; this release grants no new signing authority.

## Remaining Runtime Gaps

- Parent-owned source export, Rust compilation/unit/PG execution and real Docker
  acceptance have not run in this checkout. This source is not deployed/ready.
- Automatic Fleet preparation, mapped-volume v3 custody and controller restart
  recovery are outside this release. The private prepared document is required.
- Container configuration activation/replacement returns unavailable before
  touching files and retains draining. Operator reconciliation is required.
- Docker stdout/stderr collection and replay are not implemented here; preserved
  native log persistence is unchanged. No log collection receipt is advertised.
- Task/PM admission, loaded configuration/skill inventory, model admission and
  deployment gates remain unsatisfied. `runtime_ready` stays false.
- Lifecycle operations are serialized within one supervisor; database uniqueness
  and owner checks fence other processes. No controller takeover or unknown
  operation replay is implemented.

## Mandatory Verification

Local light verification passed: nine Python loader/README tests, README structural
validation, rustfmt, offline locked Cargo metadata (eight packages), SDK revision
verification, Base's static Docker invocation gate, and `git diff --check`.
Rust tests/compilation, PostgreSQL and Docker were not run because the parent
reserves the heavy slot and owns exported-source QA. No `target`, dependency
cache, `node_modules`, container or runtime preparation was created here.

CI explicitly selects seven receipt/client unit cases, four Linux lifecycle/path
cases, two isolated PostgreSQL controller cases and one additive migration case,
with exact passed-count assertions. Existing mandatory journal12 dispatch, atomic
ACK, terminal/readback and historical-lineage suites remain enabled. Six Python
loader contracts run without Docker. Parent runtime QA must additionally exercise
real readiness, failure-stop, descendants/namespace exit, foreign generation,
lost start/stop ACK, second-controller holds, and two isolated agents through the
existing Base ComposeHelper v2 disposable-resource workflow.
