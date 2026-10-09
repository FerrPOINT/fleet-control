# Docker Configuration Activation, Unit18

Source-only successor of `337aac87092be42027f669fc0730858f3724824f`.
This is not native acceptance or production runtime readiness. The single new
migration is `m20261009_000018_container_activation`; historical15/16/17 files
are unchanged. No controls13, approval14, UI, generated contracts, lockfiles,
SDK, Base, Tracker or Workflow changes are included.

## Supported Path

An automatically prepared, acknowledged, running original generation owned by
this live Fleet custodian can activate a validated desired config revision.
The image, process, original per-agent API credential, isolated four-directory
layout and physical controller/volume identity remain pinned. This unit changes
configuration files and generation, not image recipes or credentials.

1. The existing activation request sets `draining=true`, blocking assignments
   and dispatch. The queue and every activation transaction check active runs,
   prepared/submitted Hermes acceptance, and dispatching/uncertain outbox work.
   No active/unknown run is cancelled to make activation proceed.
2. Before native effects, Fleet saves a create-new, fsynced0600 private plan
   outside agent mounts, with exact target and previous bytes (including absence),
   original recipe/credentials, both reserved generations/operation/stop IDs,
   mappings, marker hash and source hashes. PostgreSQL pins its hash, previous
   launch and both recipe hashes. A permanent-inode Linux descriptor lock and
   exact journal CAS prevent concurrent delivery. No clock lease steals custody.
3. `planned -> stopping_previous -> previous_stopped` records the original stop
   ID before Base stop. Only the original `namespace_exited` receipt matching
   original snapshot hash permits file writes. Physical exit is read back again.
4. `applying_candidate -> preparing_candidate -> candidate_prepared ->
   starting_candidate -> candidate_running` uses original Base `prepare`, then
   acknowledged registration, mapped attachment and one original start. Re-entry
   uses `reconcile_preparation` or read-only original start observation, never a
   new create/start permit. The native claim itself is committed by Base before
   its native effect. A crash between Fleet permit and native claim stays held.
5. Physical snapshot and original private endpoint, exact file readback,
   authenticated `/health` and required capabilities are bracketed and bounded.
   `candidate_ready -> committed` atomically publishes effective revision,
   config state and running launch, then releases drain. Desired stays the
   requested target throughout. Runtime health alone does not grant admission.

The fresh generation recipes are retained as immutable private evidence, with
hash authority in PostgreSQL, allowing subsequent activations without treating
the initial unit17 receipt as authority for a different generation.

## Rollback And Holds

A known candidate API-readiness failure follows `stopping_candidate ->
candidate_stopped -> applying_rollback -> preparing_rollback -> rollback_prepared
-> starting_rollback -> rollback_running -> rollback_ready -> rolled_back`.
Base169 cannot restart an exited namespace: rollback creates a fresh generation
using the exact previous working files, image and process. Candidate exit proof
precedes restoring any files. The target becomes failed, previous effective
revision (including an initial null revision) is unchanged, and drain is released
only after rollback physical/file/API proof. A file-apply failure before candidate
preparation can use the same rollback path after original exit proof.

Unknown stop, preparation, attachment or start does not initiate rollback,
rotate credentials, adopt an endpoint or create another namespace. It retains
drain and original command/receipt. Only this live controller resumes its journal
with original readback. Foreign markers, links/hardlinks, layout/path changes,
source/recipe hashes, physical mapping/volume drift and DB/disk mismatches hold.
Partial private plan files are never overwritten. Evidence and exited original
namespaces are not automatically deleted.

## Exact Dependencies And Capability Blockers

- Base utility169 `ae8af2342b61090094292e75a7c23bf464757468`: existing
  prepare/reconcile_preparation, stop, observe, start and mapped endpoint/attach.
  No replacement API, arbitrary origin or host controller is invented. Agent
  containers receive no Docker socket or controller evidence mount.
- Build SDK remains `19a7a381ae6dbea61a643bb96189e483fa64df5c`.
- Parent must normal-merge Pascal's canonical hashes/Unicode mapping and recovery
  fixes (`be1b040`, combining `337aac` and `ce5ca`) before source/native acceptance.
  This unit makes only visibility/serialization additions to lifecycle/control
  helpers; it does not alter their hashes, canonicalization or recovery logic.
  This describes the frozen unit18 dependency. The standalone normal integration
  now includes both frozen heads; see [integration evidence](CONTAINER_ACTIVATION_INTEGRATION.md).
- Recovered/foreign-owner generations, stopped/unstarted activation and image
  upgrades remain held. Base169 has no process-only owner takeover or authority
  to resend a missing native prepare/start claim. Mid-activation controller
  restart requires a separate custody/recovery contract; no lease expiry bypass
  or generic config finisher can release that drain.
- API readiness plus a fresh-started namespace and exact mounted bytes is not
  native self-attestation of loaded provider/model/tools/skill inventory, nor
  Workflow assignment/deployment provenance. Existing admission blockers remain;
  this release cannot close production runtime status as `operator_prepared`.
- Log ingestion, business run controls, UI/integration/publication and config
  replacement under recovered custody are separate units, not bundled here.

## Evidence And Mandatory Gates

Light checks: rustfmt parsing/check, offline locked metadata, original Base fake
engine selectors, sealed-loader/README tests, SDK pin, static Docker invocation
audit and whitespace checks. These do not compile or run the Rust tests.

CI requires exact nonzero counts: app state machine2, Linux intent/lock3,
PostgreSQL activation3 and migration18 roundtrip/history1, plus9 original Base
fake-engine stop/preparation/start/mapping selectors. PG tests use only dedicated
`fleet_container_activation_test` and `fleet_container_activation_migration_test`
databases. Existing lineage tests cover both canonical and split histories.

Pending parent/native gates: compile/Clippy/full regression; those Rust/PG tests;
canonical Linux Git-SHA export; real Compose creation/replacement; fresh physical
mapping and original credential probes; active/unknown run drain; successful
activation and failed-readiness rollback to exact previous bytes; concurrency,
crash/lost-ACK and foreign-owner holds. No Docker, Rust build, PostgreSQL, push or heavy cache
preparation was performed by this unit. Source freeze is review evidence only.

Actual local light results:9 original activation fake contracts,5 inherited
preparation fake contracts and9 loader/README regression tests passed (23 total).
SDK verifier, Docker invocation audit, README verifier, rustfmt check, offline
locked metadata (8 workspace packages), whitespace and protected-path diff checks
passed. Rust unit/PG/migration tests were not executed locally.

## Changed Paths

```text
.github/workflows/ci.yml
backend/app/src/container_activation.rs
backend/app/src/lib.rs
backend/infra/src/container_activation.rs
backend/infra/src/config_revisions.rs
backend/infra/src/lib.rs
backend/infra/src/runtime/container_activation.rs
backend/infra/src/runtime/container_control.rs
backend/infra/src/runtime/container_lifecycle.rs
backend/infra/src/runtime/container_preparation.rs
backend/infra/src/runtime/mod.rs
backend/infra/tests/container_activation.rs
backend/migration/src/m20261009_000018_container_activation.rs
backend/migration/src/lib.rs
backend/migration/src/lineage_tests.rs
backend/migration/tests/container_activation.rs
backend/migration/tests/message_order.rs
docs/API.md
docs/DATA_MODEL.md
docs/MIGRATIONS.md
docs/RUNTIME.md
docs/CONTAINER_ACTIVATION_RELEASE.md
docs/contracts/AGENT_RUNTIME_CONTRACT.md
docs/contracts/HERMES_ADAPTER_CONTRACT.md
docs/contracts/JAVA_AGENT_ADAPTER_CONTRACT.md
scripts/check_container_activation_contract.py
```
