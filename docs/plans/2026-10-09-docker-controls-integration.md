# Docker And Runtime Controls Integration

Status: source assembly; native acceptance is pending.

## Verified Compiler Finding And Scoped Fix

Actual hosted run37964514708 on controlsbc4ee52/source98 terminates check101.
The safe authenticated artifact11632134258 reports E0609 at
backend/infra/src/runtime/acceptance_readback.rs:58:30, with no truncated context.
Its ZIP digest is eeea7798af77fa3870cd0bb399cdc05c53bb8fbf1f4c4cbf6a2b65e6d72d5d41.
Both independent readbacks and scratch/DB/platform cleanup succeed. This is
failure evidence, not backend acceptance.

Scoped direct successor of98, e2e33b2e655df84d47ceabcc781495d60ed4e8c0,
uses the domain archive status instead of a nonexistent DTO field and extends
the existing scope regression, without changing test identities, API or schema.
Normal parent mergea634e01052f6b2a658b104e589b08ab5157ac17c combines it with
recoveriese391f25/452. The sole source conflict preserves the async Base-verified
origin and generation checks while using AgentStatus::Archived. Parent formatting,
README and diff checks pass; no Rust compilation/PG/native proof is claimed.
The next full46 hosted gate uses exact fixede2e33b2; it cannot certify the larger
container65 union. Frozenbf27 native packet is retained unchanged but needs a new
corrected source/image packet before execution.

## Sources And Ownership

This isolated assembly normally merges frozen UI/control/recovery source
`98d950eb618071de7647e56626ad991259058be2` with Docker15/16/preparation17
`be1b040597a9ddd0847aca2c10fdaadb96e4c4a9`. Neither source is rewritten.
The SDK remains `19a7a381ae6dbea61a643bb96189e483fa64df5c`.
Base runtime utilities retain their separate sealed169 identity.
No installed runtime, Tracker or Workflow source is changed.

The combined branch is a QA assembly, not a single main release unit. Preserve
separate migration ownership and prerequisite order when preparing release PRs.
Normal merge `bf27a1d782f87d6f03f728831e700fda78892c8c` adds configuration18
successor `906e102bdc2e48c349d7fecbfd708ad417089768` to fixture fix `c1ff1a3`.
Independent source review closes oversized-target and pre-plan retry findings;
native activation/recovery acceptance remains pending.

The isolated normal successor merges exact parent
`3488863ea90d1e377dce18ee3cd658b08e8dc06a` with reviewed restart recovery
`452709e08756793ca2f437480328f85fe33f4750`. The parent checkout is unchanged.
Only five documentation introduction blocks conflict; retain both complete
contracts and separate the Docker restart section. Backend/CI hooks merge
automatically and are independently checked against both source trees.

The independent bounded merge review finds no new issues in the repository/
runtime hook union, migration assertions and retained CI selectors. Activation
implementation/tests and migration18 atbf27 are byte-identical to906. This review is
source evidence only, not Rust compilation or PostgreSQL execution.

## Semantic Resolutions

- Preserve unknown original-key acceptance lookup, fresh accepted free-chat
  context, atomic terminal persistence and the late-event terminal guard.
- Resolve runtime addresses through the async Base-verified original container
  origin. Verify the sealed generation before and after recovery/status reads.
- Keep exact-request approval preflight and its pinned POST origin; do not
  resurrect the removed generic run-control mutation helper.
- Make control reservation and approval recovery SQL use
  `fleet_container_origin` with the journal's original capabilities. A private
  address alone is not authority; localhost process compatibility is preserved.
- Retain every historical migration. The union has19 canonical and22 split
  versions; eight tail migrations follow task chats. Update boundary/downgrade
  assertions without skipping tests or rewriting the ledger.
- Keep both CI selector sets. Add one PostgreSQL regression for original Docker
  control/approval context, wrong origin/generation and lost-generation holds;
  the controller PG selector now requires exactly six cases.

## Evidence Boundaries

Independent review of74d found an incomplete capability fixture in the new PG
regression: generation alone failed durable dispatch protocol validation before
the intended assertions. The successor supplies the full server-agent/bearer/
durable-idempotency/endpoints contract plus the original generation. Production
validation is unchanged. Independent source review closes the fixture finding;
actual PG execution remains pending.

Formatting/parser and eleven sealed-loader/utility Python cases pass on the
merged working tree. On the cleanbf27 source, the original Base169 activation
nine-case and preparation five-case fake-engine selectors also pass with no
skips. The selectors overlap;14 executions are not14 distinct tests. They use a
separate clean utilities checkout atae8, not the SDK19a checkout. These checks
do not compile Rust test fixtures or execute
PostgreSQL, Docker, Hermes, native controls or full SDLC. The new PG regression
is authored, not executed. Hosted OpenAPI evidence and328 frontend tests belong
to the frozen98 source, not a full gate of this backend assembly.

The combined CI now requires65 focused container Rust cases: original
controller20, mapped recovery/isolation17, preparation10 and activation18.
Controller PG6 (including original-origin regression), activation intent10 and
activation PG5 are retained explicitly. Parent348 required60; worker452 required64
because it did not contain the extra parent original-origin case. These counts
are mandatory source inventory, not Rust execution results.

## Restart Integration Light Evidence

On the merged source: Rust1.88 `cargo fmt --all -- --check` and locked/offline
no-deps metadata pass (eight workspace packages); README, SDK19a verification,
all three canonical Base169 utility hashes and `git diff --check` pass. CI YAML
has six jobs and62 run blocks; every run block passes Bash syntax-only parsing.
Source-backed inventory checks match all65 exact focused names and CI counts.

The complete migration tree is unchanged from parent348:19 canonical/22 split,
down8 tail guard and activation predecessor-by-name. Controls13/time14,
original-origin regression, frontend, OpenAPI, SDK pin and lockfiles are unchanged.
The five activation-owned source/test/runner files exactly match452; parent
runtime hooks and controls/approval fences remain present. No new migration.

Current Windows pure runs:14 loader/utility/README tests and five preparation
fake tests pass, zero skips. The unchanged activation fake runner executes12
passing cases and three Linux-only skips; its required zero-skip exit is1,
not local acceptance. Parent independently proved the exact452 Linux runner:
15/15 pass, zero skips, exit0. No WSL/Linux execution was repeated here.

Independent crash-matrix packet22/22 passes with zero skips; parent separately
reruns that exact immutable packet22/22 in3.055s, verifying all three source pins.
This proves only actual Python/SQLite fake-Engine/clock behavior plus static
source assertions, not Rust/PG/native behavior. The old packet is unchanged.

No Rust compile/test/Clippy, PostgreSQL, native Docker/Hermes, heavy build,
publication or runtime mutation is performed. Frozen native packet remains
exactbf27, not retargeted to this source. Recovered generation replacement and
rollback remain unsupported by Base169; typed audited holds are not permits.

## Hosted Backend Attempt

The reviewed build-only controls411a were ordinarily pushed once to their
dedicated branch. Actual [run37960216273](https://github.com/FerrPOINT/fleet-control/actions/runs/37960216273),
attempt1/job113920853411, terminates failed before checkout or compilation.
The job-container default shell was `sh`; `set -euo pipefail` requires Bash.
Fallback cleanup also attempted to call the not-yet-checked-out helper. The
platform stop-containers step succeeds; no backend test or Base secret-checkout
step executes, and no PASS artifact exists. No rerun is authorized from this
failure. A new normal successor must explicitly select Bash, cover early
checkout failure in cleanup and pass review before publication.

This gate targets exact98, notbf27. Its result cannot certify the container
assembly even after a corrected hosted run succeeds.

The reviewed normal controls successor9d75 explicitly selects Bash and guards
early cleanup on controls-checkout success. Parent and independent reviewer
pass24 pure tests. Actual [run37961376127](https://github.com/FerrPOINT/fleet-control/actions/runs/37961376127),
attempt1/job113924765007, passes setup, all source/SDK/Auth checkouts and compiler
prerequisites, then fails the `check` stage. No success artifact is uploaded.
Private Cargo diagnostics were not published and cannot identify the exact cause
from the generic stage receipt. A new diagnostic-controls packet must expose
only bounded error codes and allowlisted source locations, never raw messages,
rendered source, private Base logs or credentials, before another invocation.

## Subsequent Source And Physical Gates

Configuration restart successor452709e (normal parent906) adds read-only
interrupted-claim discovery, complete original-plan reload and deduplicated
audited recovery holds. It is now explicitly integrated by the normal merge
described above; frozen nativebf27 evidence does not cover the successor.
Its15-case original Base169 fake-engine selector runs on Linux/WSL:
15 pass, zero skips, exit0. This closes only the Windows-skip verification gap;
the new Rust/PostgreSQL regressions remain uncompiled/unexecuted. Base169 lacks
recovered new-generation preparation/attachment, so automatic recovered
replacement/rollback remains unsupported rather than reported successful.

The new two-Hermes packetfea2 targets exactbf27; seal7a24be65c5470920c35bebd8a1fb1ee7bf9e31cced24e0197949260e20465990
is verified independently, with30 pure driver cases passing. Both driver findings
are source-closed: volume subpaths use `HostConfig.Mounts`, and replay rereads
the single assistant mirror and compares its ID/body. No native run is admitted:
the exact controller/Hermes images are absent and native6/6GiB commit headroom
is insufficient. The old packet is not repaired or relabelled. Separately
qualified candidate image recipes/provenance and a new sealed input packet are
required; accepted runtime tags/images are never replaced to make tests pass.

Fresh Docker-group audit exits0/complete=true, with37 desktop containers and
zero containers on each rootless runner daemon, no violations. This verifies
grouping only, not service health, image availability or runtime acceptance.
The observed host free commit is3888193536 bytes and C free98441158656 bytes;
neither a memory reserve nor Forge's stricter108279229428-byte reserve is waived.

Forge source25be retains accepted main. Parent independently runs29 successor
and40 transport pure tests successfully; this is not the full12 native gate.
The admitted earlier prepare8904 exits1/OSError without native launch/seal; exact
project cleanup is empty and protected cache/exports/evidence remain retained.
Its missing errno is unknown, not retrospectively inferred from low disk space.
New25be preparation is not invoked while capacity fails. A separately reviewed
hosted alternative must preserve the original complete matrix and disk reserve.

## Producer Authority

Fresh remote review retains Tracker PR114 source357caa7 and Workflow PR90
source9b4107. Workflow `require_owner_execution_evidence` still raises Conflict
unconditionally, and PM bind still requires an already-running Fleet callback.
Neither is predispatch/first-step authority. Those read-only producers must
provide compatible trusted contracts before task-bound model calls are enabled;
no bypass, synthetic receipt or healthy-process substitution is permitted.

Before release require all-target Rust check/strict Clippy, both PostgreSQL
lineages and populated downgrade guards, every mandatory control/recovery/
container selector, schema equality, actual isolated two-Hermes lifecycle/chat/
configuration acceptance and independent review of semantic resolutions.
Do not infer admission, first Workflow step, safe process stop or PM completion
from healthy API or run ACK. Missing producer predispatch authority remains held.

## Parallel Work

Forge prepares a transport-overhead correction without relaxing300/30/10-second
deadlines or the full assertion matrix. A separate worker prepares the real
two-Hermes driver. Configuration18 restart/recovered-custody handling has its
own implementation worker; an independent reviewer checks the live-driver
assertions and cleanup. Hosted full Linux/PostgreSQL gate preparation
targets frozen98 separately. None of these preparation tasks is native PASS;
heavy execution requires reviewed immutable inputs and exact cleanup ownership.
