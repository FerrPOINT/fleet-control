# Automatic Container Preparation Unit17

Source-only release from `fc2e27b78538f58ca014b251487024977b91f8a9`.
It removes the operator-prepared-document requirement for the configured initial
Docker generation. This is not native acceptance or production runtime readiness.

## Dependency Boundary

- Original Base utility169 `ae8af2342b61090094292e75a7c23bf464757468`:
  `resolve_mounts`, `prepare`, `reconcile_preparation`, then the existing original
  registration/start/attachment/observation/stop contracts. Utility bytes remain
  hash-checked through the existing private subprocess loader, not a host service.
- SDK remains `19a7a381ae6dbea61a643bb96189e483fa64df5c`. No Base, SDK, public API,
  generated files, lockfiles, foreign pins, controls13/approval14 or tail18..22.
- One new migration: `m20261009_000017_container_preparation`. Historical15/16
  migration files are untouched; both accepted migration lineages append17.
- Parent must merge the separate unit16 successor before native acceptance:
  canonical Git utility hashes, Unicode mapping hashes, recovery lease isolation
  and unapplied foreign heartbeat holds. Unit17 does not implement those fixes.
  Overlap is limited to lifecycle entry and new control-client methods in
  `runtime/container_lifecycle.rs` and `runtime/container_control.rs`.

## Configured Path

Trusted `fleet.container_control.provisioning` supplies `project`, immutable
`image_id`, `task`, `purpose`, numeric non-root `user`, explicit `entrypoint`,
`network_internal`, `pids_limit`, `memory_bytes`, `nano_cpus`. All are required;
Base validates the closed recipe and limits before native effects. Hermes command,
port, credentials, HOME and cwd come from Fleet, not agent input. Projects are
limited to sdlc1/sdlc2 or owned temporary QA projects accepted by Base.

Initial filesystem provisioning uses `/config` and `/workspace` for the configured
Docker path. The existing effective-file renderer uses the same path projection;
this adds no activation transaction, drain/restart engine or replacement flow.
Existing native-layout files are not rewritten or adopted. They must pass fresh
effective readback or remain held. Java and unconfigured process paths retain
their original path rendering and lifecycle.

When an initial generation is requested, Fleet verifies the original agent marker,
four concrete paths, private files and effective config revision. A configured
mapping controller is resolved by original Base into the original named volume
and physical controller snapshot. No daemon-side path becomes bind authority.

Before native create, Fleet fsyncs a create-new 0600 intent and its 0700 parent,
then commits a PG claim sealing its canonical hash. The intent contains exact
agent/paths/port/revision/config hash, generation, original operation UUID, image,
local and mapped recipes, process/credentials, utility hashes, context, mapping
and local file fingerprints. Secrets remain outside agent mounts in private
storage; PG contains identities/hashes and the non-secret original prepared receipt.

One PG CAS permits `prepare`. Every subsequent attempt uses only
`reconcile_preparation` with the original command, paths, operation and credentials.
Native effects remain exclusively inside original Base's guarded Compose contract.
Unknown readback never allocates another generation, namespace, network or token.
Concurrent contenders cannot acquire another create permit. The prepared receipt
is immutable, persisted before the existing pre-start claim and physical gates.
Launches require that exact acknowledged receipt. Effective revision changes are
fenced; drafts remain allowed. Any preparation history prevents downgrade/deletion.

Without provisioning, legacy operator-prepared15/16 remains supported. Preparation
history is sticky: removing configuration cannot enable native fallback.

## Holds And Acceptance

Missing/partial/foreign intent, file/PG mismatch, changed credentials/config/recipe,
foreign marker/path/volume, unknown Base claim or mapping/controller changes hold.
Crashes between intent and PG claim, or between delivery permit and native claim,
can hold without effects: original Base cannot safely turn missing native evidence
into permission to create. Missing original mapping/creation journals are not
reconstructed. Recovery of an unstarted preparation across controller restart
needs an additional original Base custody contract; unit16 running-owner recovery
does not authorize this. No new control API is invented to fill these gaps.

Mandatory CI selects five original Base fake-engine cases, three private adapter
cases, two intent cases, one process/Java projection case, three PG
permit/ACK/config/launch-fence cases and one
additive migration/downgrade case, with exact non-zero result counts. Local light
evidence is not Rust compilation, PostgreSQL or Docker evidence.

Pending parent native gates: canonical Linux Git export after the unit16 fixes;
locked Rust check/clippy/tests; both PG lineages and17 upgrade/down; actual pinned
image/Compose creation, no pull/build, original volume/marker guards, lost ACK and
crash readback, API reachability, physical readiness and original namespace stop.
Automatic new-generation replacement/restart, config activation and durable log
ingestion remain separate releases. Admission/model/workflow gates remain intact;
`runtime_ready` is not promoted by preparation or operator acceptance.

## Local Light Evidence

On 2026-10-09: five original Base fake-engine cases passed against the read-only
utility169 source; nine existing sealed-loader/README unit cases passed; rustfmt,
offline locked metadata (eight workspace packages), pinned SDK verification,
README validation, static Docker-invocation gate and whitespace check passed.
These Windows checkout checks do not resolve the separately reported canonical
Linux-export P1. Rust adapter/intent/projection and PostgreSQL selectors are
mandatory CI work, not claimed local passes. No build, live Docker, database run,
cache preparation, push, merge or deployment was performed.
