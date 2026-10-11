# Genuine Docker/Hermes Protocol4 Preparation Successor

**Historical document below, superseded for current execution.** Current source
is `c3fc175b97168736717c72c2b32e1036c5b6f9db`; current input/resource/command
instructions and coverage limits are in [../README.md](../README.md). Fresh
source-built/offline-qualified images are mandatory, not the historical IDs.
No native matrix has executed for this successor. The old content is retained
as historical provenance only and is not an executable current recipe.

Preparation only, not acceptance. No Docker, Cargo, PostgreSQL or native process
has been executed for this packet. QA24/be1 and all product checkouts remain
immutable. This private driver retargets the reviewed bf27 helper and does not patch
product source, migrations, API contracts or locks.

## Prerequisites

- Exact merged source: `b249bc895e5160fe13383c49d42a24c9308852b3`.
  This contains the original bf27 driver target, UI98, durable controls
  and recovery, Docker15/16, preparation17, config18/19, sequential recovered activation and complete capability
  fixtures. Its backend, frontend, selected scripts and runtime contract blobs are
  exported directly from Git. Browser/UI behavior is not exercised by this
  native driver and receives no acceptance claim.
- Original frozen18 ancestor: `bde6486219b40571b9b31a2d6cbaa739aad2df32`.
- Required canonical mapped/preparation fixes:
  `be1b040597a9ddd0847aca2c10fdaadb96e4c4a9` must also be an ancestor.
  The historical bde packet does NOT satisfy this requirement. It and its
  helpers remain immutable in the original QA directory; seal `474783ae...`.
  Successor additionally requires UI98, capability c1ff and activation fix906
  ancestry and the separate compiled FOUR-module executable `CONTROL_SHA256`
  seal. Historical three-field receipt provenance is not executable authority.
  No source override or pin substitution.
- Rust/UI SDK: `19a7a381ae6dbea61a643bb96189e483fa64df5c`, clean own sibling.
- Original Base protocol4 utility/launcher:
  `9b53de7b23593949a9e6c05bd5a4f94b930e50a0`, FOUR raw Git blob hashes checked.
  This is distinct from the unchanged Rust/UI SDK. The original launcher is
  byte-identical to ae8; no new image/producer primitive is required for this
  preserved scenario. Utilities are exported to `input/base-runtime`, mounted
  read-only at `/base-runtime`; the controller proof explicitly opts in with
  `recovered_activation=true`. Missing/changed modules or opt-in fail closed.
- Genuine Hermes: `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3` from the
  read-only local Git donor. All 13,770 Git blobs are exported and checked
  against `/opt/hermes` in the pre-existing native image before any launch.
- Maintenance path is read fresh from User `SDLC_MAINTENANCE_BASE` on Windows.
  `scripts/compose_helpers.py` SHA256 must be
  `2569882c3bb6a8b86ebb42367254720c5f802d5423bc98ef02c6fe3283e0874f`.
- Existing immutable controller/Hermes/PG image IDs are recorded in the manifest.
  These are candidates from earlier genuine local evidence, NOT independently
  inspected during preparation. Execution verifies IDs, genuine Hermes source
  label and all source blobs. Missing images fail closed; no build/pull fallback.
- At least 30 GiB free, reviewed source/helper seal, fresh exclusive heavy-slot
  grant, Linux daemon and original Compose/Docker security contract are required.
  Local model provider key is synthetic; no accepted secrets are read.

## Commands

Run from this `qa` directory (Python 3.11+):

```powershell
python -B -X utf8 -m unittest -v test_driver.py test_protocol4.py
python -B -X utf8 run.py --prepare --fleet-revision b249bc895e5160fe13383c49d42a24c9308852b3
python -B -X utf8 run.py --verify --packet C:/absolute/owned/qa/sdlc-qa-fleet-native4-<12hex>
```

Only after coordinator reviews this successor seal and explicitly releases
the heavy slot with a source-qualified native ACK:

```powershell
python -B -X utf8 run.py --execute --packet C:/absolute/owned/qa/sdlc-qa-fleet-native4-<12hex> --docker-context desktop-linux --heavy-slot-ack $NativeAckFromCoordinator
```

The coordinator-supplied ACK must contain `exclusive`, `native4` and
`b249bc895e51`. The old native18 grant cannot authorize this successor.
No default ACK is provided. Execution is one attempt per
immutable packet. No retry overwrites receipts. The standalone QA crate builds
against immutable exported Fleet dependencies using Rust 1.88 and an appended
QA-root lock record; frozen product lock bytes/records are preserved. Actual
Cargo dependency resolution, compilation and live behavior remain unqualified
until the reviewed cold run. No dependency download occurs in preparation.
Future build additionally requires exactly one Cargo binary artifact at the
sealed QA source path, one successful build-finished record, actual ELF bytes,
and an exact binary copy. Host preflight checks this compile proof against
sealed QA source/product lock/QA lock and actual Cargo output hashes before
starting PostgreSQL or native scenarios. Synthetic parser unit fixtures never
serve as actual build receipts.

## Independent Review Closure

Leibniz reviewed the immutable `474783ae...` driver and reported two P2s.
The bf27 helper already closed these findings. This protocol4 retarget preserves
all of that helper's runtime assertions byte-for-byte; the only Rust edit is the
Base launcher mount alias. Pure parity tests bind that claim to this repository's
immutable baseline commit `a9ced17e1f16d27e0713b065944674499b292ccd`.

- Volume subpath is checked in exact `HostConfig.Mounts`: volume Source,
  Target, ReadOnly (Base's absent-false default), NoCopy and Subpath. Physical
  `Mounts` still must have four exact volume names/destinations/RW values.
  No assumption is made about the daemon's physical `Mounts.Source` suffix.
- After an idempotent message replay, `answer` rereads the transcript and
  requires exactly one assistant mirror with the same original ID and body.
  The existing single model-call/run assertions are retained.

Two additional source-backed model regressions use the original Base fixture
counterexample and duplicate-mirror counterexample. They are static/pure
closure evidence, NOT Rust execution or native acceptance. The independent
reviewer's report/proofs and the old packet remain read-only.

## Coverage Matrix

These are expected classifications, not achieved results:

| Scenario | Required classification / proof |
| --- | --- |
| Two-agent preparation | accepted: actual Rust supervisor calls original Base create/register/attach/start |
| Isolation, health, chat | accepted: marker, controller/engine/volume mapping, HOME, cwd, four subpaths, cross-token denial, distinct CID/PID and SOUL |
| Transcript idempotency | accepted: one genuine model request, one run and one assistant mirror per prompt/replay |
| Drain and activation | accepted: blocked in-flight run preserves old bytes/generation; release drains then activates new revision/generation |
| Readiness failure | accepted: real 60s production readiness deadline, verified physical stops, exact config/SOUL/env rollback, distinct restored generation |
| Peer preservation | accepted: peer launch/snapshot and physical PID/StartedAt unchanged |
| Unknown native ACK | held: real ACK consumed/lost, one POST; no fabricated native ID or completed transcript |
| Physical controller restart | accepted only if same CID restarts and Rust recovery restores healthy custody |
| Unknown ACK after restart | held: original message replay, unchanged POST ledger/run/preparation/activation/launch counts and generations over multiple worker intervals |

Interrupted config activation after controller death is explicitly
unqualified, not a passed scenario. Current Fleet includes migration19 and its
protocol4 consumer, but this driver completes activation and rollback BEFORE
the physical controller restart. It does not interrupt native ACK/DB-CAS or
exercise recovered configuration resume/rollback or next-recovered-child (F6).
Enabling protocol4 does not make those native scenarios covered.
Stopped/unstarted recovery is likewise explicitly held/unqualified: this
driver first starts genuine native namespaces. Neither boundary is skipped
into acceptance, and both appear separately in `held_unqualified` in the seal.
No PM, task-bound admission, UI, real-model quality or published runtime claim.
`runtime_ready=false` and `sdlc_completion=false` remain mandatory even after
this scoped native matrix passes.

## Fixture Boundary

The baseline is genuine pinned Hermes, not an API mock. A QA-only local
OpenAI-compatible model returns unique synthetic responses and can hold one
actual inference. Each native container runs the original Base launcher behind
a transparent QA proxy. The proxy forwards auth/status/events/body bytes,
counts POST hashes, and for the explicit unknown prompt consumes a real native
2xx ACK before closing the socket without ACK. A separate hashed ACK-loss ledger
must record exactly that one real 2xx loss. It never resends or invents a
run/capabilities/readiness response. A designated SOUL sleeps 120s before boot
to exercise the unchanged 60s product readiness deadline.

Initial `config.yaml` and `SOUL.md` are explicitly seeded as bootstrap fixture
after original `FilesystemProvisioner`; original marker and `.env` stay intact.
No effective revision, launch authority or native receipt is inserted into SQL.
Subsequent revisions use real public repository requests and Rust activation
worker transactions. This does not qualify production initial-config admission
or an unmodified production entrypoint. Only free-chat sessions are created.

## Ownership And Cleanup

One unique temporary project, no host ports. Seven exact disposable volumes:
agents, controller, compiled, postgres, registry, target, scratch. Two outer
networks: internal fleet and isolated cold-build network. No shared cache.
The controller alone receives the explicitly read-only Docker socket; this is
still privileged daemon authority, NOT a read-only API/security sandbox.

Original Base creates native per-generation Compose resources in the same
project because mapping requires the volume project identity. Its genuine
boundary labels have no maintenance cleanup-id: they are never forged or
rewritten. Before ComposeHelper v2 closes, the driver reads exact private Base
creation recipes, quiesces the controller, validates all native resource
project/owner/image/generation/network/volume identities, and performs exact
Compose down using a cleanup-only union of the original manifests. The union
does not launch services. Its synthetic-token-bearing copy is deleted after
use; hash and non-secret native service/network ledger are retained. V2 then
removes only its seven registered disposable volumes. Any foreign/unregistered
resource refuses cleanup and fails acceptance rather than guessing/deleting.
If the controller is already dead or a creation raced recipe capture, cleanup
may remain held: preserve evidence/resources for journal-aware recovery.

Finally checks source/helper seals, pins, maintenance hash, image availability,
permanent runtime/image parity, project resource emptiness, exact volume and
network absence and v2 `cleaned` journal. Original scenario failure is retained
even if cleanup fails. Terminal report is fail-closed and separate from logs;
an unsavable report emits a failed compact fallback, never a PASS receipt.
Synthetic config/tokens and raw original recipes remain in disposable private
volumes only; model prompts are synthetic and POST ledger stores hashes only.
No secrets or raw command arguments are emitted by Python error reporting.
Native CID/image/generation/StartedAt inventory is retained before and after
controller restart and must match exactly; DB counters alone do not qualify it.

The historical b7f/e45/dad captures informed fixture design only. They are not
current source proof, were not rewritten, and are not inherited as gate PASS.

Historical failed prepare `sdlc-qa-fleet-native18-2e7389023803` remains in the
read-only donor, non-executable; it is not copied into this successor.
Its broad scripts export correctly rejected tracked `__pycache__/*.pyc` in the
product Git tree. Successor exports only six named contract/source `.py`
scripts, preserving bytecode/link/cache rejection. Selected source members are
checked before packet creation; product history and tracked files are unchanged.
