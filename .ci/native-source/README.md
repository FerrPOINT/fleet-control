# Hosted Fresh-Source Native Preparation

This README supersedes current-status claims in historical wrapper reports and
the historical sections of `qa/README.md`/`qa/CUTS.md`. Historical packets and Git
objects remain unchanged; they are not receipts for this successor.

**Build-only candidate, not accepted.** The unit is a cold source image build,
the unchanged offline qualifier, then the existing two-agent native driver and
separate protocol4 cut supplement on the **same Linux daemon**. The caller is
[native-source-build-only.yml](../../.github/workflows/native-source-build-only.yml),
one Ubuntu24.04 job on the dedicated build-only push branch. This two-phase
recipe successor requires review and execution authorization. Current results
require authenticated run readback; prior attempts did not reach native acceptance.
No registry
transport, service, scheduler, Hermes patch or production source edit is added.

## Exact Inputs

- Fleet `c3fc175b97168736717c72c2b32e1036c5b6f9db`, tree
  `beeb851103acf908c395b90e7bbcc816d979034e`, sole parent
  `324a8000e6b766a77c1bab8ca494a9de16409bc1`.
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
are rejected. The exact aabe-to-c3fc delta is the reviewed PM continuation
custody refusal, its fixture/test file and four docs. Production byte parity
with aabe is explicitly false; the candidate is not backend-qualified here.
`qa/source_coverage.py` verifies that delta
and catalogs current config admission/activation, migration026, free-session
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
The source-only recipe successor uses uv0.11.6's supported two-phase path:
`sync --frozen --no-install-project` selects the original runtime lock and extras,
then `pip install --no-deps --editable` installs only the Hermes root into that
same venv with the two original hash-URL build constraints. Both phases retain
the complete registry no-build package vector; build isolation stays enabled.
Project sync uses a space-delimited no-build-package vector; pip install uses
the equivalent comma-delimited only-binary vector. No project/lock/dependency or
resource pin changes; this candidate still requires actual cold/offline/native
qualification and does not reclassify the prior UNKNOWN/FULL failure.
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
`7aeeef584b88c8cf366912d6c14187af29a3925e`; their compact path/SHA256 map digest is
`a0e9c37135cc4ed28c8b1dae38d5488a29041c9ae4e19a30fbd9585c3135dc94`.
Only this README and the workflow are caller-specific. No cache, Git donor,
credentials or prepared packet is included. In that one host job, set absolute
exact Git input directories (no dependency copies):

The canonical raw-Git build input inventory is14085 files (Fleet222, SDK85,
Base7, maintenance1, Hermes13770), SHA256
`0fb4147c1dbc3904600135027ae8d78cf1fcb6a545cc88c09701ed699cbe6b68`.
This is source-only hashing, not a build, image or native receipt.

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

Build ACK: exactly `exclusive-source-image-build-<packet-name>-c3fc175b9716`.
Native ACK contains `exclusive`, `native4`, `c3fc175b9716`; cut ACK uses
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

Hermes failed-build diagnostics may add only `recipe_instruction`
(`apt_setup`, `uv_sync`, `account_setup` or null) and `inner_exit_code`
(1..255 or null). A unique complete FULL terminal frame must exactly match a
RUN from the hash-checked sealed recipe and its reviewed command hash. No
command/path/frame/body is projected. Missing/conflicting/partial/untrusted
evidence stays null/unknown; old receipt shapes, categories, bounds and timeout
behavior remain compatible. These are diagnostic observations, not a cause or
native acceptance proof. Publication and execution require final independent
review and explicit authorization; acceptance requires validated run receipts.

Original-nine failed output exposes only fixed first-failure phase/class and
matrix cleanup proof (true/false/null); terminal-save failure uses the same
projection, never the private report. Successful output and all runtime checks
are unchanged. This does not recover the cause of prior failed attempts.

Build failures may additionally expose a fixed shell step, original exit code
and at most eight Rust Edddd codes; optional coordinates refer only to the fixed
live main.rs. Missing or invalid evidence remains null, with no error text or
private path. Input/record bounds are 8MiB/16384 lines and 2KiB; no new artifact
is uploaded. Successful output, original commands and locked inputs are unchanged.

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
Successful pulls and unrelated private logs are never read.
Success console shape and all receipt validators are unchanged; no new step,
artifact or reader. Prior617 run38074207531 had no cause retained; d4dd
run38075925575 proved only parent_pull/BuildFailure/command_nonzero, not a
specific parent or registry cause. Neither past report is retroactively changed.

A failed candidate build now optionally adds `candidate_build`: exact source
role controller/hermes, actual numeric return code (null on real process
timeout), fixed symptom category and `log_scope=full|tail|unavailable`. Only its
same exclusively created build-log handle is read from the beginning, at most
8MiB+1. Files above8MiB, more than65536 complete lines or failed reads stay
unknown/unavailable without classifying a partial prefix or tail. Within those
bounds the complete captured log is scanned; an unterminated last line is
ignored. This is a symptom projection, not root-cause diagnosis. Up to8 distinct
complete anchored Rust
`error[E####]:` frames are retained as codes only, with existing BuildKit
prefixes; the list is a bounded subset, not a complete compiler inventory.

Complete anchored existing CLI/recipe refusals can also establish
docker_cli_refused, compose_config_refused, pinned_fetch_refused,
pinned_hash_refused, apt_refused or account_refused in either a full log or tail.
Only the current or retained historical build flags, Compose build shape and UID/GID999 are
recognized. The existing fetch script's closed ValueError proves a fetch
refusal, not a hash mismatch; only the explicit checksum-check diagnostic maps
to pinned_hash_refused. Generic summaries, incomplete lines or conflicting
symptoms stay unknown. Only original-size full logs of at most64KiB may
additionally expose legacy network, no_space or dependency_resolution symptoms;
larger logs and explicit tail inputs use only existing anchored complete frames.
No text, path, URL, body, argv, env or token is projected.
The combined extension remains within1KiB, preserves first failure and does not
read success/qualification/cleanup logs. Controller image construction installs
tools and does not compile Fleet: a Rust cause is never assumed from the role.
Actual1fa run38077155580 proved candidate_build/command_nonzero with all parent
parity passed, but retained no candidate role, child exit or build cause. This
change does not retroactively diagnose it or qualify any native scenario.

Direct qualification targets pinned sourcec3fc;2dc/5db evidence stays historical.
Parent authorized isolated candidate QA separately from backend qualification,
not runtime promotion. Published f876/sourcec3fc run38113987817/1 failed candidate
build; original-nine/cuts did not execute and root cause remains unproven.
The authentic aabe codegen receipt38107719356 retains its original provenance;
reuse requires exact76-file API closure parity, not backend/native acceptance.
Any new successor still requires review and explicit execution authorization.
Existing PM/F6 coverage limits remain unchanged.

## Host Compose compatibility

Run38078412938 used runner image20261004.327.1, whose official software manifest
lists host Compose2.38.2. This version supports builder/pull/no-cache but not the
provenance flag introduced in2.39.0. The cold image's Docker pin does not select
the host CLI. The closed failure retained docker_cli_refused/controller/exit1,
not the flag name; the compatibility defect is independently source-attested.

The successor removes only optional `--provenance=mode=max` from the existing
Compose build command. Repository plans/contracts and the actual qualification,
source, ELF/native and receipt validators have no BuildKit/SLSA attestation
consumer or max-metadata requirement. This is an explicit compatibility
simplification, not a waiver of required acceptance or a SLSA claim.
`--builder`, `--pull=false` and `--no-cache`, recipes, all existing pins,
resources, same-daemon checks, matrices, cleanup and validators remain unchanged.
No host plugin bootstrap/upgrade, Docker config, daemon/store change, new
transport, step or fallback is introduced. Native execution remains pending.

Evidence: [actual runner software manifest](https://github.com/actions/runner-images/blob/e3fe113a581eb9a44ca43f479b69f9c93f36df34/images/ubuntu/Ubuntu2404-Readme.md#L75),
[Compose2.38.2 build flags](https://github.com/docker/compose/blob/9e17a091be5abf792fcb4c4e35a80a7cc51cbe6b/cmd/compose/build.go#L123),
[Compose2.39.0 release](https://github.com/docker/compose/releases/tag/v2.39.0).

## Paired UV fatal symptoms

### Historical UV Pair Extension (Before Recipe Fields)

Within the bounded fully captured failed-build log, a complete anchored
UV0.11.6 fatal header is recognized only with a later complete
`#N ERROR: process ... exit code: N` for the SAME vertex. Vertex correlation
occurs before prefix stripping; the nonzero inner exit must be1..255 and is
used internally only. No command, vertex, inner exit, frame, new DTO or field
is emitted. Only existing category may gain uv_build_refused,
uv_download_build_refused, uv_no_solution or uv_no_platform_distribution.
These are observed UV symptoms, NOT root-cause or authenticity claims.

Partial/ANSI/quoted/source-gutter/mismatched/reversed or malformed pairs do not
qualify. Identical pairs deduplicate; conflicting reasons stay unknown. A paired
uv_no_solution supersedes only the equivalent generic dependency_resolution
label; independent Rust/CLI reasons remain conflicts. Unpaired original-size
full-log generic resolution behavior is unchanged. Existing first-failure,
1KiB projection and explicit-tail partial-line handling are preserved. No extra
artifact, progress flag, recipe, lock, input pin or resource admission change.
Actual38079988227 and38081865039 remain UNKNOWN/TAIL; this extension cannot
reconstruct their omitted private cause or qualify a native scenario.
