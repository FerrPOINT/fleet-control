# Fleet Sequential Recovered Activation Successor

Own checkout: `C:/git/azhukov/sdlc/.local/fleet-protocol4-activation-release-20261009/fleet-control`.
Branch: `feat/recovered-configuration-activation-20261009`.
Normal immutable parent: `f326cdab4045f726449bf07e61a96a1b75e3063a`.
Resulting normal successor SHA is supplied in the handoff; f326 is not amended.
SOURCE implementation for independent review, NOT native/production acceptance.

## Implemented In This Successor

- Next revisions activate over an effective committed OR rolled-back recovered
  child using the existing activation machine, not a new business planner.
- A new private plan/claim seals root anchor/family and exact terminal predecessor
  ID/hash before effects. Original controller, receipts and credentials remain
  immutable. Old plans/claims omit optional lineage, preserving their hashes.
- All private ancestors are read/hash-checked with cycle/depth bounds. New claims
  require fresh original root exit/readback, current native lease/ACK, latest
  predecessor authority, effective config, desired drain and no active/unknown runs.
  Stable next plan replays are serialized/idempotent; conflicting plans are held.
- Original mapping evidence/root journal is reused under Base4 fresh custody,
  never resolve/new child recovery. SQL lineage insertion/progress and child
  control mutations retain inherited latest-root fencing. First predecessor stop
  follows own CAS; restored stop reconciles exact original claim/ACK/physical exit.
  Recovered pre-plan proof failures become audited HOLD, not native retry permits;
  sequential hold diagnostics name the root custody and current intended command.
- Candidate-head readback rechecks original predecessor exit. Failed-next rollback
  uses CURRENT effective bytes/revision and reserved new rollback generation;
  candidate exit/full physical/file/API readiness precede effective publication.
- Only existing unreleased migration19 is extended; no second migration/table,
  no edits to15..18. Inventories remain20/23. This is not an in-place DB upgrade
  for already-applied f32619: final source release applies updated19 from18.

## Successor Checks

- Linux sealed loader + exact Base9b private fake Engine:21/21,0 skips,47.785s
  (final repeat; preceding21/21 repeat60.692s).
  Six added native-contract cases cover next committed/rolled-back chains,
  lease expiry/unknown/new epoch without effects, duplicate/conflicting plans,
  failure rollback after exact current candidate stop (no repeated kill).
- Earlier successor attempt:20 passed/1 error,111.955s. A positive fixture lost
  its20-second lease before start. Repeated with supported30-second bound; no
  lease guard/clock bypass. Negative expiry checks remain active.
- Windows captured loader7/canonical utility5/README3:15/15,33.945s.
- `cargo fmt --all --check`; locked/offline/no-deps metadata (8 packages), pinned
  SDK19a, canonical four utility Git hashes, README/Python/CI/inventory and diff
  checks are light gates only, not Rust compilation or SQL execution.
- Authored mandatory selectors, NOT executed here: activation intent14 (two
  new); activation PG13 (four new). App2/replacement3/migration19 roundtrip1 and
  original focused selectors retained. CI count checks updated, zero ignored.
  PG cases explicitly cover both terminal outcomes, next failure rollback,
  third-plan lineage, active runs, exact immutable predecessor, concurrent replay,
  conflict, unknown heartbeat, expired lease and changed epoch authority.
  Exact new PG selectors (all mandatory in CI):
  `recovered_committed_child_next_activation_and_failure_rollback_keep_original_anchor`;
  `recovered_rolled_back_child_next_activation_and_failure_rollback_keep_original_anchor`;
  `recovered_next_claim_rejects_foreign_lineage_unknown_lease_and_duplicate_plan`;
  `recovered_next_claim_expiry_and_new_epoch_require_latest_original_authority`.

## Integration And Pending Gates

SDK remains19a7a381ae6dbea61a643bb96189e483fa64df5c; utilities remain exact9b53de7b23593949a9e6c05bd5a4f94b930e50a0.
Base/SDK/parent/UI/PM/Forge unchanged. Parent fixture/lineage fix0be22c7 is a
separate normal integration dependency, not duplicated/cherry-picked here.
Preserve its corrected activation test fixture plus this successor's two new
tests when merging the same runtime activation file tail.

Pending: parent locked compile/Clippy/Rust tests, PG both20/23 lineages and19
roundtrip, canonical Linux export, actual Docker/Compose/restarts/lost ACK and
authenticated Hermes readiness. No heavy/native/PG/cache prep/push/dispatch.
Base4 permits at most256 commands per root family, including rollback. Fleet
preflights a conservative128-plan ancestry budget before sealing/stopping,
reserving two native commands per plan, even when earlier plans did not roll back.
Capacity exhaustion holds with current effective child untouched, never rotates
journals or mints a new anchor. Missing original private plan/prepare/start/attachment/stop
proof or permit remains typed HOLD. Process-only takeover, image/process/token
rotation, log ingestion and business/runtime admission are not implemented here.

## Successor Changed Paths

`.github/workflows/ci.yml`, `README.md`, `REPORT.md`;
`backend/app/src/{container_activation.rs,lib.rs}`;
`backend/infra/src/{container_activation.rs,lib.rs,runtime/container_activation.rs}`;
`backend/infra/tests/container_activation.rs`;
`backend/migration/src/m20261009_000019_recovered_activation.rs`;
`scripts/check_recovered_activation_contract.py`;
`docs/{API.md,DATA_MODEL.md,RUNTIME.md,RECOVERED_ACTIVATION_CONSUMER.md}`;
`docs/contracts/{AGENT_RUNTIME_CONTRACT.md,HERMES_ADAPTER_CONTRACT.md,JAVA_AGENT_ADAPTER_CONTRACT.md}`.

---

# Historical f326 Handoff (Superseded Only For Sequential Activation)

The following is the prior source packet/evidence. Its next-plan limitation and
counts describe f326, not this successor. The frozen f326 commit is unchanged.

Own checkout: `.local/fleet-protocol4-activation-release-20261009/fleet-control`.
Branch: `feat/recovered-configuration-activation-20261009`.
Normal parent: `30f0993d65bd6435769f34f43fa7db30a21845a7`.
This is source freeze for independent review, NOT native/production acceptance.
The handoff message supplies the resulting immutable commit SHA.

## Implemented

- Opt-in recovered ORIGINAL activation resume/rollback uses sealed Base4
  prepare/attach/start/endpoint/stop and immutable reserved commands. Full physical,
  managed-byte and authenticated readiness proof precedes effective publication.
- Original claims/controllers/operation/generation/stop IDs, credential payload,
  plan hashes and saved receipts are unchanged. Latest recovered lease and exact
  whole-record CAS are separate authority; drain/active/unknown run fences remain.
- Only additive migration19: immutable activation authority, original exited-
  anchor renewal, inherited child SQL custody/mutation fence, no child takeover.
  Historical15..18 are unchanged; canonical/split inventories are20/23.

All five prereview findings have product fixes and specific regressions:

1. Child SQL origin/launch mutations inherit latest ORIGINAL anchor lease under
   locks, including missing ACK, unknown heartbeat, expiry and newer epoch.
   `recovered_child_sql_origin_serializes_heartbeat_then_holds_unknown_expired_and_new_epoch`
   covers concurrent origin/heartbeat serialization and direct SQL mutation denial.
2. Replacement has secret-safe custom Debug, never derived command/env Debug.
   `recovered_reentry_has_no_new_native_permits` checks redaction and stop-only permit.
3. StoppingPrevious restored after native stop ACK but before PG phase CAS invokes
   Base9b's read-only `stop_readback` with fresh recovered mount guard. It never
   invokes stop/claim/kill or initializes a missing journal. Five new Linux fake
   cases cover exact receipt/unchanged evidence, absent/wrong claim, no start ACK/
   no exit, wrong/unapplied lease and child unknown stop/exit. Native evidence is
   original start ACK + exact immutable stop claim + bracketed physical exit;
   Base has no separate durable stop-ACK row. Unknown running/no-proof stays held.
4. Terminal child first stop is enabled only after this worker's successful
   running-to-stopping CAS. Restored stopping is reconcile-only; no prepare/start/
   attachment permit is gained. Committed AND rolled-back SQL stop paths are in
   `recovered_terminal_draft_keeps_historical_custody_and_fenced_regular_stop_both_outcomes`.
5. Terminal custody depends on effective published generation, not new desired
   draft. Exact historical launch lookup survives draft/next activation requests;
   effective revision, byte/readback and lease guards remain. The preceding PG
   case uses actual create/validate/request/claim contracts for the next revision,
   preserves the old anchor, and rejects a new child custody anchor/new plan.

## Verified Light Gates

- Linux sealed four-module private fake Engine:15/15, zero skips,21.861s
  (final selector-enforced repeat; preceding corrected run48.705s also passed).
- Linux original activation fake selectors:15/15, zero skips,8.290s.
- Linux original preparation fake selectors:5/5, zero skips,0.495s.
- Windows captured-loader7 + canonical utility5 + README3:15/15,12.628s.
- Four canonical utility Git hashes, pinned SDK, README validation, Python parse,
  CI YAML/62 Bash blocks and mandatory selector inventory pass.
- Cargo metadata locked/offline/no-deps passes (8 packages); source formatting
  and diff whitespace checked. This does not compile Rust or execute SQL.
- One earlier fake run failed (226.093s): two inherited positive cuts exceeded
  the live lease during filesystem stalls and a new fixture omitted Status/PID.
  Fixture fixed, real lease checks retained; subsequent15/15 pass is separate.

## Dependencies And Pending Acceptance

SDK stays `19a7a381ae6dbea61a643bb96189e483fa64df5c` (clean sibling).
Utility pin is separately `9b53de7b23593949a9e6c05bd5a4f94b930e50a0` (frozen Base clean).
PR180 merge815982b has identical four canonical module bytes; no substitution.
Mandatory CI: app-state2, activation-intent12, replacement3, activation-PG9,
migration19 roundtrip1, existing control9/mapped6 and migration18 gates retained.
Authored Rust/PG selectors were NOT executed locally. Parent owns compiler fixes
from later integration, locked compile/Clippy, SQL on both20/23 lineages, canonical
Linux export, real Docker/Compose, controller/Fleet restart, lost-ACK/lease races
and authenticated Hermes readiness. Windows Linux skips never count as PASS.

Remaining product limits: no pre-plan/claim reconstruction; old native preparation,
missing original start/attachment/stop proof or absent permit remains typed held.
Stop CAS before the native stop claim is also held on restart. Saving a next draft
and stopping current effective runtime work; a NEW activation plan chained from a
recovered child is not implemented. This is a Fleet planner/authority extension,
not a claim that Base4 lacks generic linear replacement. Generic restart/image/
token rotation/log ingestion/provider-model-tools/Workflow admission are separate.
No heavy build/native Docker/PG/cache prep, push, hosted dispatch, parent/UI/SDK/
Base/Forge mutation or publication occurred. See
[contract and phase coverage](docs/RECOVERED_ACTIVATION_CONSUMER.md).

## Changed Paths

Product: `backend/app/src/{container_activation.rs,lib.rs}`;
`backend/infra/src/{container_activation.rs,container_recovery.rs,container_runtime.rs,lib.rs}`;
`backend/infra/src/runtime/{container_activation.rs,container_control.rs,container_lifecycle.rs,container_preparation.rs,container_recovery.rs,container_replacement.rs,mod.rs}`;
`backend/shared/src/config.rs`.
Migration/tests: `backend/migration/src/{lib.rs,lineage_tests.rs,m20261009_000019_recovered_activation.rs}`;
`backend/migration/tests/recovered_activation.rs`; `backend/infra/tests/container_activation.rs`.
Gates: `.github/workflows/ci.yml`; `scripts/{check_recovered_activation_contract.py,verify_container_utilities.py}`;
`scripts/tests/{test_container_control_loader.py,test_verify_container_utilities.py}`.
Docs: `REPORT.md`, `README.md`, `docs/{RECOVERED_ACTIVATION_CONSUMER.md,API.md,DATA_MODEL.md,RUNTIME.md}`;
`docs/contracts/{AGENT_RUNTIME_CONTRACT.md,HERMES_ADAPTER_CONTRACT.md,JAVA_AGENT_ADAPTER_CONTRACT.md}`.
