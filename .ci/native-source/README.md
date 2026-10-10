# Hosted Fresh-Source Native Preparation

This README supersedes current-status claims in historical wrapper reports and
the historical sections of `qa/README.md`/`qa/CUTS.md`. Historical packets and Git
objects remain unchanged; they are not receipts for this successor.

**Prepared, not executed or accepted.** The unit is a cold source image build,
the unchanged offline qualifier, then the existing two-agent native driver and
separate protocol4 cut supplement on the **same Linux daemon**. The caller is
[native-source-build-only.yml](../../.github/workflows/native-source-build-only.yml),
one Ubuntu24.04 job on the dedicated build-only push branch. This diagnostic
successor is not yet published or executed; prior attempts did not reach native
acceptance.
No registry
transport, service, scheduler, Hermes patch or production source edit is added.

## Exact Inputs

- Fleet `5db4ff92d2168c46ce96b56f37acbbf7de92db33`, tree
  `5328b7de2d2e1922ae6748b02f974471a01d9de4`, sole parent
  `e00b73070c327d28883c3f7a2a523b759df6662b`.
- SDK `19a7a381ae6dbea61a643bb96189e483fa64df5c`.
- Base cold recipe, launchers and four utilities
  `9b53de7b23593949a9e6c05bd5a4f94b930e50a0`.
- Base ComposeHelper v2 `1cf6f3f72fc10043293d4804d05c9b1156b14ac3`, helper SHA256
  `2569882c3bb6a8b86ebb42367254720c5f802d5423bc98ef02c6fe3283e0874f`.
- Unmodified Hermes `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3`, **13770**
  ordinary Git blobs, exact pyproject/uv lock and frozen web/messaging extras.

[images/inputs.json](images/inputs.json) pins five official parent image
digests, exact public URLs/SHA256/size bounds for wheel0.45.1, setuptools83,
Rust1.88's manifest and signed Debian snapshot metadata. Anonymous registry
challenge/token/manifest metadata was checked during preparation; **no layer or
image was pulled**. Installation/platform closure remains unproven.

`images/build.py --prepare` exports named Git inputs through framed
`cat-file --batch`, verifying every Git blob identity. No worktree copy,
`git archive`, credentials, dependency directory or historical image enters the
context. The seal covers sources/recipes; extra context/qualifier-parent files
are rejected. The exact e80-to-5db delta is three infra fixture/test files and two
docs; production bytes are unchanged. `qa/source_coverage.py` verifies that delta
and catalogs current config admission/activation, migration024, free-session
projection and stored request hash. Two additive native assertions cover
`task_bound=false` and the exact request hash. All inherited assertions remain.

## Cold Build And Resource Boundary

The controller derives from pinned official Rust/Docker parents. Hermes derives
from the exact Base cold runtime recipe, pinned Debian/uv and original launchers.
No historical Fleet server is inherited. Current QA Rust binaries are compiled
later by the original wrapper with its mandatory actual ELF/source proof.
This unit does not build the API server/Swagger or generate OpenAPI.

Signed snapshot metadata is hash-checked before APT installs. Registry Python
sdists are prohibited; Hermes' own editable build uses explicit hashed
wheel/setuptools constraints. No resolver has run: missing platform wheels fail
closed, without fallback or lock mutation. Cargo retains product lock checksums.
The original offline qualifier is LF-byte-identical: Rust/Cargo1.88, fmt/clippy,
Docker/Compose, all13770 blobs and `uv sync --check --frozen --offline` with the
original extras. UID/GID999 and socket boundaries remain unchanged.
As in the original wrapper, the build service owns fresh build volumes as root;
native controller and offline qualifier explicitly select999:999.
The exact original `/var/run/docker.sock` endpoint and observed socket GID are
bound to the build receipt. Only the native controller receives that additional
group (plus its original group0) and read-only mount; no socket chmod/chgrp.

One exclusive Linux host job, local Unix-socket daemon and **existing**
same-context `docker`-driver Buildx builder; no bootstrap/new builder service.
Daemon minimum: 6GiB memory/two CPUs. Measured available physical **and** commit
memory minimum: 6GiB each, capped by cgroup v1/v2 limits. Local disk floor stays
**30GiB**; explicit disposable GitHub Linux CI floor is **5GiB**. Major phase
preflight requires **8GiB estimated additional headroom** (38/13GiB free).
This is measured preflight, not a quota/reservation or promise a 14GiB runner
will fit. Both packet and DockerRootDir filesystems are measured, with floor
checks repeated between phases. No temporary8GiB file, toolcache
deletion, global prune, waiver or foreign cleanup.

Base v2 `with`/finally journals own temporary Compose resources. Qualification
has no network/socket and a read-only root. Native wrappers retain seven exact
disposable volumes per run and original native-generation cleanup. Candidates
remain for the subsequent same-daemon stages, not runtime promotion. Source,
image and permanent-runtime parity checks remain mandatory. Historical image
constants are parser fixtures only; execution requires a fresh successful
build/qualification receipt, exact source/seals/IDs/daemon and cleanup.
An `always` caller step removes only the two exact fresh candidate aliases after
both native phases or a failed attempt. Admission records their initial absence
and daemon; removal requires the sealed source ownership labels and execution
marker, and successful builds also require the exact qualified IDs. Hermes is
removed before its controller parent, using `image rm --no-prune` without force.
No official parent/cache/unrelated alias is deleted. Missing cleanup proof
prevents success output; terminal VM teardown is not cleanup evidence.

## Future Commands: Not Execution Authorization

The authored caller requires parent/independent review before publication. Its
public preflight verifies the exact source tuple and additions before any private
Base token use. All35 copied QA/image files match component
`0567f9a0b722d9ea58d5df997db035413244a970`; their compact path/SHA256 map digest is
`b44db400eaf9ddf51e51a4bcf68adc8b7dfec562bad6018fa23638e520e5a202`.
Only this README and the workflow are caller-specific. No cache, Git donor,
credentials or prepared packet is included. In that one host job, set absolute
exact Git input directories (no dependency copies):

```bash
set -euo pipefail
export FLEET_QA_SOURCE_REPO="$FLEET_REPO"
export FLEET_QA_SDK_REPO="$SDK_REPO"
export FLEET_QA_BASE_REPO="$BASE_UTILITY_REPO"
export FLEET_QA_HERMES_REPO="$HERMES_REPO"
python3 -B images/build.py --prepare --profile disposable-ci \
  --fleet-repo "$FLEET_REPO" --sdk-repo "$SDK_REPO" \
  --base-repo "$BASE_UTILITY_REPO" --maintenance-repo "$BASE_MAINTENANCE_REPO" \
  --hermes-repo "$HERMES_REPO"
# BUILD_PACKET = exact returned absolute owned path, not a guessed directory.
python3 -B images/build.py --verify --packet "$BUILD_PACKET"
python3 -B images/build.py --execute-build --packet "$BUILD_PACKET" \
  --docker-context "$DOCKER_CONTEXT" --builder "$EXISTING_DOCKER_BUILDER" \
  --heavy-slot-ack "$REVIEWED_BUILD_ACK"
export SDLC_MAINTENANCE_BASE="$BUILD_PACKET/context/sources/maintenance"
python3 -B qa/run.py --prepare --image-build-packet "$BUILD_PACKET"
# NATIVE_PACKET = exact returned original-nine packet path.
python3 -B qa/run.py --verify --packet "$NATIVE_PACKET" --image-build-packet "$BUILD_PACKET"
python3 -B qa/run.py --execute --packet "$NATIVE_PACKET" \
  --image-build-packet "$BUILD_PACKET" --docker-context "$DOCKER_CONTEXT" \
  --heavy-slot-ack "$REVIEWED_NATIVE_ACK"
python3 -B qa/cut_run.py --prepare --image-build-packet "$BUILD_PACKET"
# CUT_PACKET = exact returned separate supplement packet path.
python3 -B qa/cut_run.py --verify --packet "$CUT_PACKET" --image-build-packet "$BUILD_PACKET"
python3 -B qa/cut_run.py --execute --packet "$CUT_PACKET" \
  --image-build-packet "$BUILD_PACKET" --docker-context "$DOCKER_CONTEXT" \
  --heavy-slot-ack "$REVIEWED_CUT_ACK"
```

Build ACK: exactly `exclusive-source-image-build-<packet-name>-5db4ff92d216`.
Native ACK contains `exclusive`, `native4`, `5db4ff92d216`; cut ACK uses
`native4cut`, not `native4`. One attempt per packet; no overwrite/rerun defaults.
Native preparation is held until genuine fresh image qualification completes.

## Pending Acceptance And Privacy

Original nine scenarios: two real Hermes namespaces with synthetic local model,
isolation, transcript idempotency, drain/activation, readiness rollback, peer
preservation, real consumed/lost ACK, physical controller restart and held
original-command replay/no second POST. The separate cut exercises **known
original stop ACK before PG CAS**, recovered A-to-B, then bounded next-child C
failure restoring B. Neither matrix has executed for this successor.

General interrupted activation/F6 crash coverage beyond that exact cut remains
missing. Original-nine interrupted/F6 classifications stay held, not silently
promoted by the supplement. PM/task-bound dispatch, real Auth/Tracker/Workflow,
initial SDLC admission/publication, UI/browser, real-provider quality and
stopped/unstarted recovery are outside this unit. Mandatory
`runtime_ready=false`, `sdlc_completion=false` stay even after scoped success.

Full source inventories, Base sources, build/qualification logs, package
inventory and native recipes are private. Never upload whole packets/broad logs.
Any future public handoff is restricted to reviewed closed statuses, source,
seals/image IDs/digests and bounded cleanup proof. Same-daemon execution requires
no image tar or new registry credentials. No workflow/push/dispatch/heavy run has
been performed by this preparation task.

The caller's only artifact is a success-only `receipt.json` (maximum64KiB).
It revalidates existing original-nine and cut reports, actual compile proofs,
original live assertions, cut gate/trace and cleanup before projecting the
closed pass/held matrices, source/run/controls/pins/image IDs and helper seals.
JSON evidence is ordinary, bounded and strict; the cut trace has an aggregate
1MiB limit. Missing/failed evidence produces no success artifact. No private
compile proof, inventory, recipes or logs are uploaded; failures have no
diagnostic artifact. Native/Linux compile, resolver closure and all acceptance
remain pending actual hosted execution.

Existing cold-builder failure console output now additionally projects a closed
failure class, source-attested operation category, typed reason where proved,
and the five parity booleans (null means unobserved/invalid). This diagnostic
extension is capped at1KiB; unknown exceptions/reasons are OtherError/unspecified,
never guessed from an exception message. Only a failed owned public-parent pull
adds `parent_pull`: exact source-enum kind, actual numeric process return code
(negative means a signal; null on timeout/unavailable), and a fixed symptom
category. At most64KiB+1 is read from the same exclusively created pull-log
handle; truncation, read failure, unknown or conflicting symptoms, and ambiguous
repository-missing/login denial stay `unknown`. A plain401/Bearer challenge is
not denial proof. Categories are rate_limit/registry_denied/manifest_unavailable/
dns/tls/timeout/unknown; they do not establish registry visibility or root cause.
No log text, URL, args, env, token or exception text enters the projection.
Successful pulls, candidate build and other private logs are never read.
Success console shape and all receipt validators are unchanged; no new step,
artifact or reader. Prior617 run38074207531 had no cause retained; d4dd
run38075925575 proved only parent_pull/BuildFailure/command_nonzero, not a
specific parent or registry cause. Neither past report is retroactively changed.

The direct qualified source would be5db only. Later UI-only aca/152 commits and
the subsequent infra test-only fixture repair require explicit source parity
review, not a claim that this job directly checked their heads. They do not
replace the frozen source inventory or introduce PM/F6 coverage.
