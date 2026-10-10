# Managed Request Observer V1

Status: Linux/component and selected two-Hermes native core verified; interrupted
activation/Fleet-death acceptance remains pending. This is not
loaded-configuration, inventory, admission, PM or SDLC acceptance.

## Design

New configuration drafts may explicitly opt into the four-file Base observer at
`43b365fd97e8955821312cbc2e68b4d83e5bd240`. Preparation reads bounded Git objects
from the configured Base checkout, never its working tree, HEAD or the network.
The snapshot records immutable repository/commit/file hashes. Existing renderer
1/2 snapshots retain their serialized bytes and are never implicitly enabled.

Observer installation and removal are configuration operations. The complete
file plan participates in the signed activation journal, backup of original
bytes/absence, fresh disk readback and rollback. Preparation/GET never installs,
repairs, discovers, reloads or starts inference. Arbitrary plugin source paths
and caller-provided hashes are not authority.

Production authenticated readback uses only the existing original Fleet run,
native run/session, controller/launch/origin and a pinned observer incarnation.
Custody is checked before and after bounded HTTP. Missing, foreign, stale or
malformed observations fail closed; GET never dispatches or replays a prompt.
Fleet launch revision is provenance, not proof of native loaded revision.
`complete=false`, `runtime_ready=false` and both admission blockers remain.

## Consumer Contract

The operator uses the existing configuration PUT with
`config_json.fleet_request_observer = {"enabled":true}`. Preparation stamps the
exact repository, `revision` and four raw-byte `sha256` values and seals the
native `plugins.enabled` entry. Only new Hermes renderer-3 snapshots may carry
this proof. Java and immutable renderer-1/2 snapshots are not upgraded.
New drafts cannot enable the reserved native plugin without this proof. Once
adopted by the desired configuration, omission is denied under the existing
agent row lock; explicit `{"enabled":false}` produces a renderer-3 revision with
four managed absence entries. Other plugins are preserved. Unknown observer
files, symlinks, Unix hard links and forged provenance block without deletion.

The files `__init__.py`, `plugin.py`, `store.py`, `plugin.yaml` under
`config/plugins/fleet-hermes-request-observer` are normal journaled plan entries.
The private proof is omitted from native config bytes. Managed launch disables
Python bytecode creation; no source is copied into an image or producer SDK.
Source comes only from bounded Git objects in `fleet.base_package_checkout`
(10 seconds total, 262144 bytes per blob, exact four-file tree). Read-only
preparation does not fetch absent objects. The separate SDK pin is unchanged.

`GET /api/v1/sessions/{session_id}/runs/{run_id}/request-observation` requires
existing authenticated session read access and original accepted free-chat
dispatch. Task-bound sessions return 503 even for their owner, without Tracker
calls or native observation. Access/binding is rechecked after observation.
Handler results use `Cache-Control: no-store`; no raw prompt, tools, credential,
private dispatch context, launch/controller identifiers or plugin bytes escape.

Before the original POST, validated observer capabilities/incarnation are frozen
in the existing immutable dispatch journal's reserved `fleet_request_observer`
fact, alongside original `fleet_launch` and optional `fleet_recovery`. Prepared
submission compares that fact; original-key recovery retains all original facts
rather than replacing them with filtered fresh capabilities. Unknown acceptance
cannot use observer GET to discover/adopt a run. The existing recovery lookup is
the producer's non-dispatch POST protocol, not a repeated `/v1/runs` POST.

Readback holds lifecycle exclusion under a 15-second total bound and uses three
authenticated producer GETs: capabilities, exact original run/incarnation, then
capabilities again. Each response is exact 200, application/json, identity
encoding and at most 16384 bytes under a 3-second HTTP bound. Redirects, retries,
foreign/duplicate fields and readiness claims are rejected. Native run/session,
origin/default credential, original controller/launch, launch snapshot and disk
source are checked before/after; physical custody brackets the final HTTP too.
Factory restart, stale original launch, missing source/proof and partial/unknown
observation fail closed. A source/launch pin is not native loaded-revision proof.

Producer authority is the exact published
[Base contract](https://github.com/FerrPOINT/services-base/blob/43b365fd97e8955821312cbc2e68b4d83e5bd240/docs/contracts/HERMES_REQUEST_OBSERVATION_V1.md),
not PR HEAD, local working-tree files or mutable discovery. The returned
`fleet-managed-request-observation/v1` envelope contains scoped Fleet identities,
the closed digest-only native observation and unchanged admission blockers.

## Required Acceptance

- Actual Fleet activation and production Rust readback through two isolated
  pinned Hermes processes; selected HMACs equal requests received by the
  deterministic model. Repeated GET does not create model calls.
- Observer coexists with recovery/control extensions; lost ACK recovers the
  original run without redispatch. Peer, credential, launch and incarnation drift
  are rejected.
- Missing/tampered extension, interrupted activation and rollback restore exact
  previous bytes/absence under confirmed stop and a fresh runtime generation.

Host/component tests do not satisfy these native gates. No accepted runtime,
SDK pin, producer repository or database migration is changed by this slice.

## Verification Queue

Source tests cover explicit provenance/renderer/enablement, preservation of
foreign inventory, closed bounded authenticated HTTP, original scope and signed
backup bytes/absence after journal reopen. Existing renderer tests now also
assert renderer-2 snapshot/plan stability and reject implicit renderer3.

The ignored production test
`native_request_observer::managed_native_observer_activation_reads_two_original_runs_with_existing_extensions`
activates renderer2 then explicit renderer3 through real Fleet with two native
Hermes processes and a deterministic model. It checks original prepared/unknown
ACK recovery, recovery/control coexistence, actual unknown steer/stop ACK via
GET-only outcome lookup with one POST per command, selected wire HMACs, owner/foreign
API GET/no-store, task-bound denial, tampered source, once-only inference, failed
installation rollback to absence, failed removal rollback to original bytes and
fresh incarnation, and explicit successful removal. Packet3c1e9ff60f84 passes
this actual core gate in196.53s, with271 source hashes and owned cleanup verified.
Its startup-fault wrapper, triggers and task-binding row are owned QA fixtures,
not installed runtime edits or admission authority.

The frozen647-case Linux/PostgreSQL workspace gate passes fmt, all-target check,
strict Clippy and Rust-generated OpenAPI parity, including the eight non-native
selectors below. Thirty-three special cases remain explicitly ignored; ordinary
cargo tests are not native acceptance. Generated TypeScript/spec agreement,
tsc --noEmit and Markdown links also pass. The fake authenticated HTTP case rejects foreign
credentials, wrong MIME/encoding, oversized bodies, redirect and timeout; actual
API owner/foreign-owner/no-store and task refusal are exercised by the queued
native scenario. Earlier native attempts failed before observer activation
because of QA startup/cache defects; the corrected core scenario now passes.
See the [exact evidence](../CHAT_CLARIFICATION_VERIFICATION.md#managed-observer-source-slice-8-october-2026).

The parent has completed the frozen Linux workspace/PG and OpenAPI gate. The
ignored test now passes in its fresh owned temporary Compose harness. Required mounts are
the exact SDK plus read-only Base Git object cache containing 43b365f, both existing
`/qa/recovery-plugin` and `/qa/control-plugin`, isolated output/HOME/DB/controller
state and `FLEET_OBSERVER_BASE_CHECKOUT` pointing at that cache. Existing scenario
launcher now selects this test with `--scenario observer`, an explicit fixture
revision and a self-contained read-only bare observer cache. Fifty-four host
harness cases pass, including rejection of mutable/foreign fixture selection,
linked/common Git paths, shared alternates and changed observer bytes. This is
not native acceptance. The binary is built from a separate read-only full Fleet
backend/scripts snapshot; the report verifies original and frozen input hashes
after execution. The QA startup launcher and exact literal opt-in are read-only
binds; the production cleared environment and noexec temporary directory remain
unchanged. The separate readonly QA Git config trusts only the exact cache path;
it does not alter shared Git configuration or bypass origin/blob checks.
SIGKILL/interrupted activation recovery and actual unknown control
recovery across Fleet process death while observer is enabled remain separate
real acceptance gates. The core gate does not close those additional cases.

## Frozen Candidate Handoff

The candidate is based on Fleet `f04abbb411c69c04a796060d6d981d39f30e1e4d`.
Frozen source compilation, workspace tests, Rust OpenAPI parity and the selected
native core pass; additional crash gates and exact-head publication remain open. Heavy gates remain
parent-coordinated against immutable snapshots.
All readiness flags and both admission blockers remain unchanged.

These are the exact eight new non-native test selectors, from `backend`:

```sh
cargo test --locked -p domain --lib configuration_snapshot_tests::observer_snapshots_require_explicit_renderer_three_without_legacy_backfill -- --exact
cargo test --locked -p infra --lib request_observer_package::tests::provenance_is_fixed_and_requires_explicit_new_renderer -- --exact
cargo test --locked -p infra --lib request_observer_package::tests::foreign_inventory_is_preserved_and_denied -- --exact
cargo test --locked -p infra --lib request_observer_package::tests::absence_is_limited_to_managed_instruction_or_exact_observer_files -- --exact
cargo test --locked -p infra --lib runtime::request_observation::tests::capabilities_never_accept_readiness_or_foreign_protocol -- --exact
cargo test --locked -p infra --lib runtime::request_observation::tests::observation_binds_original_run_session_incarnation_and_partial_scope -- --exact
cargo test --locked -p infra --lib runtime::request_observation::tests::capabilities_http_is_authenticated_get_and_rejects_oversize_or_redirect -- --exact
cargo test --locked -p infra --lib runtime::activation_journal::tests::observer_removal_retains_signed_original_bytes_and_absence_after_reopen -- --exact
```

After compilation, the separate owned native harness must select:

```sh
cargo test --locked -p infra --test native_supervisor_live native_request_observer::managed_native_observer_activation_reads_two_original_runs_with_existing_extensions -- --ignored --exact --nocapture
```

Producer inputs and mount requirements:

- Prepared task-owned observer Git object cache:
  `C:/git/azhukov/sdlc/.local/fleet-observer-base-cache-20261008.git`.
  It is self-contained and bare, without alternates or shared Git mappings.
  Origin was verified as `https://github.com/FerrPOINT/services-base.git`. Read the four
  `deploy/hermes-request-observer/{__init__.py,plugin.py,store.py,plugin.yaml}`
  objects at `43b365fd97e8955821312cbc2e68b4d83e5bd240`; all four raw hashes were
  verified against the Rust constants. Set `FLEET_OBSERVER_BASE_CHECKOUT` to the
  container-visible read-only cache, for example `/producer-base`.
- The earlier producer reference was a linked worktree. Its resolved Git directory is
  `C:/git/azhukov/sdlc/services-base/.git/worktrees/services-base4`; its common
  object directory is `C:/git/azhukov/sdlc/services-base/.git`. Mounting only the
  worktree does not provide a usable Linux cache. The parent must supply a
  task-owned self-contained cache or preserve valid read-only Git directory
  mappings; never rewrite shared Git configuration or delete shared objects.
  The native runner now rejects a linked checkout and requires the prepared bare
  cache; this preflight is not evidence that the native scenario has passed.
- Observer commit 43b365f and the inspected cache HEAD
  `c083783a37791e277db796361203884b87828a7d` do not contain the recovery/control
  plugin directories. Local objects at
  `dd9ce31a6b97e31e2467662658e38dd7a6485f45` contain the existing
  `deploy/hermes-recovery-plugin/{__init__.py,plugin.py,store.py,plugin.yaml}` and
  `deploy/hermes-control-plugin/{__init__.py,plugin.py,store.py,plugin.yaml}` plus
  `deploy/fleet-hermes-launch.py`. This is an available separate fixture epoch,
  not an assertion of publication or acceptance. The parent must bind the
  authorized producer fixture epoch and record raw hashes before native use;
  mount its four-file outputs read-only at `/qa/recovery-plugin` and
  `/qa/control-plugin`. Do not extract these paths from mutable Base HEAD.
- Compilation still requires a separate exact SDK checkout at
  `cbb4e99230420dc2659431b1c9fb5090e5c940f0`, mounted read-only at
  `/work/services-base` beside `/work/fleet-control`. The advanced producer
  checkout is not a substitute for this unchanged SDK pin. Native Hermes must
  use exact source `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3` and the parent's
  independently verified immutable dependency image.
- `scripts/native_supervisor_live/run.py` now has the observer selector and
  read-only bare-cache mount. Pass the explicit fixture revision above and a
  task-owned self-contained observer cache; it refuses shared Git alternates.
  Capture gate-input hashes including both new production Rust files and the new
  native helper before/after. Set `FLEET_NATIVE_SUPERVISOR_TEST=1` and an owned
  `FLEET_TEST_DATABASE_URL`; retain isolated HOME/controller/output/target cache
  and exact Compose finally cleanup. No shared cache or accepted runtime changes.

## Changed Files

All paths below are Fleet-relative. New source files are not staged. Generated
TypeScript is an ignored local output, reproduced from the tracked OpenAPI.

```text
backend/api/src/lib.rs
backend/api/src/routes/agents.rs
backend/api/src/routes/sessions.rs
backend/app/src/lib.rs
backend/domain/src/lib.rs
backend/infra/src/base_package.rs
backend/infra/src/config_revisions.rs
backend/infra/src/effective_configuration.rs
backend/infra/src/lib.rs
backend/infra/src/request_observer_package.rs (new)
backend/infra/src/runtime/activation_journal.rs
backend/infra/src/runtime/lifecycle_tests.rs
backend/infra/src/runtime/mod.rs
backend/infra/src/runtime/prepared_dispatch.rs
backend/infra/src/runtime/request_observation.rs (new)
backend/infra/tests/native_supervisor_live.rs
backend/infra/tests/support/native_request_observer.rs (new)
openapi/openapi.json
frontend/src/api/generated.ts (generated, ignored)
docs/API.md
docs/CHAT_CLARIFICATION_VERIFICATION.md
docs/DATA_MODEL.md
docs/GAP_REGISTER.md
docs/CURRENT_STATE.md
docs/IMPLEMENTATION_PLAN.md
docs/QUALITY_GATE.md
docs/SECURITY.md
docs/TRACEABILITY.md
docs/RUNTIME.md
docs/contracts/AGENT_RUNTIME_CONTRACT.md
docs/contracts/HERMES_ADAPTER_CONTRACT.md
docs/contracts/JAVA_AGENT_ADAPTER_CONTRACT.md
docs/contracts/MANAGED_REQUEST_OBSERVER_V1.md (new)
.github/workflows/ci.yml
scripts/native_supervisor_live/README.md
scripts/native_supervisor_live/preflight.py
scripts/native_supervisor_live/run.py
scripts/native_supervisor_live/test_harness.py
scripts/native_supervisor_live/observer_fault_plugin.py (new)
scripts/native_supervisor_live/test_observer_fault_plugin.py (new)
scripts/native_supervisor_live/observer_start_fault.py (new)
scripts/native_supervisor_live/test_observer_start_fault.py (new)
```
