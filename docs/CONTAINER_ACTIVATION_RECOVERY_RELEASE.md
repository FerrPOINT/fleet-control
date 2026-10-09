# Config18 Restart And Recovered Custody

Source-only successor of `906e102bdc2e48c349d7fecbfd708ad417089768`, in a separate
checkout/branch. Frozen906, its parents and parent integration are unchanged.
No migration, public API/schema, SDK/pin, Base, Tracker, Workflow, UI, generated
contract or lockfile change. This is not native acceptance or production readiness.

## Contract Checked First

The906 handoff explicitly limited pre-plan retry to a live worker. Discovery
required either `claimed_at IS NULL` or an activation owned by that worker's
random controller UUID. A process restart lost both routes. A complete private
plan saved just before the PG claim also had no resume path.

Checked exact Base169 Git sources at
`ae8af2342b61090094292e75a7c23bf464757468`, including
`docs/contracts/RUNTIME_CONTROLLER_RECOVERY_V1.md`,
`docs/platform/RUNTIME_BOUNDARY.md` and `scripts/runtime_control.py` (not an
assumption about another checkout's HEAD). Fleet's three canonical utility hashes
remain unchanged. Base recovery accepts distinct physical start of the same
immutable controller container/Engine/volume. It preserves original generation,
registration and receipt; process-only restart is not a takeover witness.

Protocol3 permits original lifecycle/readback, but **not prepare,
reconcile_preparation, attach or resolve/new generation**. Protocol2 preparation
readback requires its original mapping, including original controller start.
Neither an expired lease nor historical ACK grants recovered authority. Config18
claim/controller and generation identities remain immutable; its existing PG
transactions reject recovery history. This release does not weaken those guards.

## Product Changes

- Bounded16-row read-only keyset discovery scans claimed, desired, draining,
  activating Docker revisions, including pre-plan claims and foreign nonterminal
  records. Cursor wrap prevents permanently held early agents starving later
  agents. Discovery never clears `claimed_at`, reclaims an owner or creates permits.
  Per-agent workers and the existing operation/OS locks remain exclusive; unit16
  heartbeat workers stay independent. Java/process discovery is unchanged.
- A complete private plan plus both byte/hash-checked recipes can be reloaded
  without resealing after lost PG claim ACK or live worker cancellation. Only
  the original live owner with no recovery history can continue. Exact original
  plan/recipe IDs, target and rollback bytes, credentials and mappings are reused.
  Partial/missing/foreign evidence is never repaired, replaced or re-forked.
  Existing phase CAS and run/drain/native guards still govern every continuation.
- A restarted/foreign owner never enters the native-effect loop. It reads
  original protocol2 observation where still physically valid, or
  `read_controller_recovery` using the **immutable original command**, not the
  current heartbeat. The original historical receipt/snapshot is checked before
  retaining its hash. It neither settles that receipt in unit16 nor renews/adopts
  custody. A current verified recovered lease allows an additional original
  observation, but still does not allow generation replacement or publication.
- Typed `RecoveryHold` is persisted in existing `last_error`, with atomic system
  audit `agent_config.recovery_required`. Repeated identical decisions add no
  audit. The transaction locks the agent and checks exact desired snapshot,
  original launch and whole activation record; stale/foreign command identities
  cannot replace newer diagnostics. No exception text, config/secret bytes,
  arbitrary endpoint or mutable lease is stored in this payload.

## Audited Recovery Actions

These are explicit durable requests for recovery, **not executable permits** or a
new control API. They are visible through existing revision/audit reads.

| Reason | Required action | Authority retained |
| --- | --- | --- |
| `original_custody_required` | `recover_original_custody` | Original launch/registration; process-only takeover not supported |
| `unknown_original_effect` | `reconcile_original_command` | Exact recorded phase generation, operation/stop ID and plan hash; never resend unknown effects |
| `recovered_generation_change_unsupported` | `resume_or_rollback_original_plan_with_compatible_base` | Verified original receipt/current recovered observation; Base capability still missing |
| `evidence_unavailable` | `reconcile_original_command` | Original stored custody; missing/invalid private evidence not reconstructed |

Command generation and observable custody generation are distinct. In particular,
unknown candidate/rollback preparation records name that reserved operation,
even if the only observable launch is the exited previous generation. A hold
leaves desired/effective, `activating`, drain, original receipts, run capacity
and admission unchanged. No `active`, `rolled_back`, `operator_prepared` or
runtime-ready status is manufactured.

## Regressions And Gates

Three added runtime regressions: exact complete-plan reload with no new keys or
reseal; partial/missing/foreign recipes remain untouched; unknown command identity
is not confused with the last observable generation and contains no private bytes.
Two added PG cases cover claimed pre-plan discovery after restart, concurrent
deduplicated audit without reclaim/drain release, immutable original launch,
unknown command identity, and stale activation/configuration rejection.

Mandatory CI requires10 activation module cases and5 PG activation cases, while
preserving app2 and migration1 and all other existing selectors. The original
Base fake-contract runner now requires15 cases with **zero skips**, adding original
recovery readback/concurrency/expiry, foreign journal, closed protocol and missing
start/evidence guards. Linux-only boot-clock cases are not silently waived.

Local light results:

- `cargo fmt --all --check` and locked offline no-deps metadata (eight packages)
  passed. No Rust compile/test/Clippy was run.
- Base169 activation fake runner:12 passed,3 Linux boot-clock cases skipped on
  Windows, so the mandatory zero-skip runner correctly returned nonzero. These
  three cases and the full15-case Linux gate remain pending, not accepted.
- Preparation fake runner:5 passed, zero skips. Loader/README/canonical-utility
  unittest runner:14 passed, zero skips. These are suite execution counts, not a
  claim of distinct cases across runners.
- SDK19a7 verification, all three Base169 canonical Git blob hashes, static
  Docker invocation audit, README structural check and `git diff --check` passed.
- CI YAML:6 jobs and59 Bash run blocks parsed; exact activation selectors are
  app2/runtime10/PG5/migration1. Focused union is64 Rust cases on this906 branch,
  not the parent's later integration count. Source-order/side-effect inventory
  confirms restart readback cannot call lifecycle effects, discovery cannot claim,
  and held persistence only updates `last_error` plus the atomic audit.
- Historical migration files/registries (canonical17/split20), original unit16/17
  controller/mapping/recovery/preparation/workers, utilities, SDK pin, lockfiles,
  generated contracts and UI are unchanged. Sibling SDK checkout is clean19a7.

The three added Rust regressions and two added PG regressions are source, not
executed evidence. Parent owns heavy/native acceptance and normal integration.

## Exact Remaining Gaps

No safe automatic new-generation/rollback preparation under recovered custody
exists in pinned Base169. A compatible original-command preparation/attachment
contract and a separately reviewed Fleet recovered-activation fence are required
before this action can execute. Returning the old random controller UUID,
rewriting mapping start/receipts, manually clearing queue/DB claims or substituting
an operator-prepared document is not recovery. This release adds no such bypass.

Process-only restart has no Base takeover proof; candidate pre-publication custody
is also not automatically recovered by unit16's effective-config guard. Unknown
stop/start/prepare without original proof stays held; there is no read-only stop
receipt settlement API added here. Physical readiness is still not loaded-model,
tool/inventory attestation or SDLC admission. Full production runtime remains open.

Parent-owned gates: Rust compile/Clippy and all focused Rust/PG tests; Linux
canonical export and zero-skip fake runner; actual Fleet/controller restart at
pre-plan, sealed-plan/PG-ACK and each unknown native-effect boundary; verify no
duplicate namespace, credential rotation, foreign adoption or drain release;
same-agent concurrency and sibling lease isolation; eventual compatible recovered
replacement/rollback and real physical/readiness/admission acceptance. No heavy,
native Docker, build/cache preparation or push was run in this unit.

## Changed Paths

```text
.github/workflows/ci.yml
backend/app/src/container_activation.rs
backend/app/src/lib.rs
backend/infra/src/container_activation.rs
backend/infra/src/lib.rs
backend/infra/src/runtime/container_activation.rs
backend/infra/src/runtime/mod.rs
backend/infra/tests/container_activation.rs
docs/API.md
docs/DATA_MODEL.md
docs/CONTAINER_ACTIVATION_RECOVERY_RELEASE.md
docs/contracts/AGENT_RUNTIME_CONTRACT.md
docs/contracts/HERMES_ADAPTER_CONTRACT.md
docs/contracts/JAVA_AGENT_ADAPTER_CONTRACT.md
scripts/check_container_activation_contract.py
```

No donor tail or additional dependency was imported. One internal repository
signature changes for read-only discovery and one is added for audited hold; no public endpoint or
wire schema changes. Integration overlap is limited to these activation methods,
the config worker region of `runtime/mod.rs`, CI selectors and contract docs;
parent must reconcile its own union counts during a normal merge. Parent
controls/approval/original-key files and Pascal's unit16/17 source are untouched.
