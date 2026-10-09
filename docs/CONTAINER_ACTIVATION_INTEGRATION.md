# Docker Activation Integration

Source integration only, not native acceptance or production admission.
The normal merge retains these exact direct parents, without rewriting either:

- Unit18: `bde6486219b40571b9b31a2d6cbaa739aad2df32`.
- Pascal16 fixes plus17: `be1b040597a9ddd0847aca2c10fdaadb96e4c4a9`.

The only migration relative to the second parent is
`m20261009_000018_container_activation`. Its blob and all historical migration
blobs remain exact. Canonical and historical split lineages contain17 and20
entries respectively. No SDK/Base mutation, pin/lock/generated/UI changes,
controls13/approval14 or giant historical tail imports are included. Pascal's
separate native preparation QA packet and future Docker driver are not changed.

## Semantic Integration

The merge had no textual conflicts. CI/docs combine all unit16/17 guards and
selectors with unit18 activation, rather than choosing one side's evidence.
Unit-specific evidence documents remain historical source results.

- Shared Base-compatible canonical hashing covers mapping, config snapshots,
  private preparation/activation plans, file maps and saved receipt identities.
  ASCII escaping includes DEL, Unicode keys and UTF-16 surrogate pairs. Existing
  hashes/receipts are not upgraded in place; a mismatch remains held. Activation
  plan roundtrip tests now include a Unicode skill path and content; PG activation
  and rollback fixtures pin an independently computed Base Unicode snapshot hash.
- Utility hashes are the canonical Git/LF Base169 blobs. Native loading remains
  strict byte-for-byte: CRLF native exports are rejected, never normalized. Both
  fake-contract runners first verify the exact Git revision and then require
  newline-equivalent checkout text for Python tests. That test-only CRLF handling
  accepts no source edits and does not change native custody or executable bytes.
- The independent per-agent renewal loop retains Pascal's current-owner-only
  heartbeat delivery, unknown foreign-heartbeat hold and original lease checks.
  Its task registry is shared with ordinary reconcile and Docker activation;
  one pending worker per agent prevents repeated delivery. Completed workers
  are reaped and shutdown aborts pending workers.
- Docker start/stop/health and activation share a per-agent operation lock.
  Long readiness polls do not hold sibling agents' operation locks. Recovery
  renewals do not take these locks; recovered stop uses a nonwaiting try-lock.
  Ordinary reconcile refreshes the agent and checks drain inside its worker.
  Freshly claimed next revisions remain queued locally while a predecessor's
  worker completes; they are not discarded as duplicate pending readback.
  No process/Java command, origin or configuration fallback was added.

Desired remains the requested revision throughout drain and rollback. Effective
changes only after original stop, exact fresh preparation, physical snapshot,
file readback and authenticated readiness proof. Rollback keeps the previous
effective revision and restores its exact working bytes on a fresh generation,
only after known candidate exit. Unknown stop/prepare/start/attachment remains
drained with original command/receipt and no new namespace or credential rotation.
All unit18 durable plan, CAS, run/outbox/journal fencing and OS-lock checks remain.

## Light Evidence

Local Windows checks on 2026-10-09:

- Original Base activation9 plus preparation5 fake-engine cases:14 passed,
  zero skips. The original loader/README9 cases also passed, preserving all23
  frozen18 cases. Canonical utility verification5 adds28 distinct Python cases.
- Exact Base169 Git blob hash verification passed. SDK sibling stays clean at
  `19a7a381ae6dbea61a643bb96189e483fa64df5c`; pinned revision verifier passed.
- Rustfmt, offline locked metadata (eight packages), README structural check,
  static Docker invocation audit and whitespace checks passed.
- CI YAML, run-block Bash syntax and selector/count inventory are checked without
  executing Cargo tests, PostgreSQL or native Docker.

The following mandatory focused CI inventory is source inventory, not local
Rust execution. Existing full-workspace, lineage/journal and process/Java gates
also remain required.

| Package / Target / Selector | Cases |
| --- | ---: |
| infra lib `runtime::container_control::tests::` | 9 |
| infra lib `runtime::container_lifecycle::tests::` | 4 |
| infra lib `runtime::container_mapping::tests::` | 6 |
| infra lib `runtime::container_control::mapped_tests::` | 6 |
| infra lib `runtime::container_recovery::tests::` | 2 |
| infra lib `runtime::container_workers::tests::` | 2 |
| infra lib `runtime::container_preparation_tests::` | 3 |
| infra lib `runtime::container_preparation::tests::` | 2 |
| infra lib `tests::docker_bootstrap_projection_preserves_process_and_java_paths` (exact) | 1 |
| app lib `container_activation::tests::` | 2 |
| infra lib `runtime::container_activation::tests::` | 3 |
| infra integration `container_controller` (ignored PG) | 5 |
| infra integration `container_preparation` (ignored PG) | 3 |
| infra integration `container_activation` (ignored PG) | 3 |
| migration integration `container_controller` (ignored PG) | 1 |
| migration integration `mapped_controller_recovery` (ignored PG) | 1 |
| migration integration `container_preparation` (ignored PG) | 1 |
| migration integration `container_activation` (ignored PG) | 1 |

Total:55 focused Rust cases, all with mandatory nonzero exact CI count gates.

## Pending Native Gates And Blockers

Parent owns canonical Linux source export, locked compile/Clippy/full regressions,
all55 focused Rust cases and both PG lineages, plus actual Docker/Compose acceptance.
No heavy work, Docker, Rust build/test, PG, cache preparation, push, deployment or
default-branch merge was performed here. Native evidence must refer to this merge
SHA and its exact compiled sources, not either frozen parent's QA packet.

Physical gates still include first preparation and original unknown readback,
no pull/build/recreate, actual mapping/controller/volume/marker checks, exact
readiness and original namespace stop, active/unknown run drain, successful
activation and failed-readiness rollback to exact previous bytes, concurrent
sibling operations/renewals, crash/lost-ACK and foreign-owner holds.

Base utility169 remains `ae8af2342b61090094292e75a7c23bf464757468`, separate from
the SDK. Activation still supports only the live original custodian with an
acknowledged running generation and no recovery history. Mid-activation process
restart, recovered-owner replacement, stopped/unstarted activation, image changes
and a missing original native prepare/start claim remain held. A queue claim lost
before its private/PG activation intent is sealed is not automatically reissued.
Loaded provider/model/tools/skills self-attestation and Workflow provenance remain
missing admission capabilities; source integration or `operator_prepared` status
does not make production runtime ready. Log ingestion remains a separate unit.

## Integration-Owned Changes

Beyond the two frozen source units, semantic edits are limited to:

```text
.github/workflows/ci.yml
backend/infra/src/runtime/container_activation.rs
backend/infra/src/runtime/container_recovery.rs
backend/infra/src/runtime/container_workers.rs
backend/infra/src/runtime/mod.rs
backend/infra/tests/container_activation.rs
scripts/check_container_activation_contract.py
scripts/check_container_preparation_contract.py
scripts/tests/test_verify_container_utilities.py
scripts/verify_container_utilities.py
docs/API.md
docs/DATA_MODEL.md
docs/RUNTIME.md
docs/CONTAINER_ACTIVATION_RELEASE.md
docs/CONTAINER_ACTIVATION_INTEGRATION.md
docs/contracts/AGENT_RUNTIME_CONTRACT.md
docs/contracts/HERMES_ADAPTER_CONTRACT.md
docs/contracts/JAVA_AGENT_ADAPTER_CONTRACT.md
```

Full review inventories are `git diff --name-status HEAD^1 HEAD` and
`git diff --name-status HEAD^2 HEAD` on the final normal merge commit.
