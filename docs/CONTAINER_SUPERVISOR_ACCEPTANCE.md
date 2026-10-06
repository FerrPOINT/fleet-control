# Container Supervisor Acceptance

## Scope

This opt-in gate runs the real Rust `LocalRuntimeSupervisor`, the source-pinned
Base Compose control utility and two real Hermes gateways. Inference uses a
controlled local OpenAI-compatible model endpoint. It is not a mock Hermes
runtime and is not acceptance of model quality, task scheduling, PM workflow,
CI/CD deployment, or the full automatic SDLC.

The Rust test is ignored by default. Never opt it in against installed databases,
agent roots or permanent Compose projects. Only the owned driver supplies its
controller proof and environment:

- [Driver](../scripts/container_supervisor_live/run.py).
- [Fresh offline compilation](../scripts/container_supervisor_live/build.sh).
- [Exact Cargo artifact selection](../scripts/container_supervisor_live/select_artifact.py).
- [Actual supervisor scenario](../backend/infra/tests/container_supervisor_live.rs).

## Inputs

Require a clean SDK checkout matching `.base-revision`, a tracked-clean Base
control checkout, the pinned Hermes Git object and four existing immutable image
IDs. The driver does not pull images or change deployment pins. The Rust image
must contain the existing offline Cargo dependencies, Rust 1.88.0 and the
hash-verified Swagger UI archive used by the other Linux gates. The Docker image
supplies only Docker CLI/Compose binaries. The Hermes image supplies immutable
dependencies; a new disposable source layer is built from the exact Git archive.

```powershell
python scripts/container_supervisor_live/run.py `
  --base-sdk <clean-pinned-sdk-checkout> `
  --base-control <clean-base-control-checkout> `
  --hermes-source <hermes-git-checkout> `
  --rust-image sha256:<existing-offline-rust-image> `
  --docker-image sha256:<existing-docker-cli-image> `
  --hermes-image sha256:<existing-pinned-hermes-dependency-image> `
  --postgres-image sha256:<existing-postgres-image> `
  --context desktop-linux `
  --artifacts <private-disposable-evidence-directory>
```

## Checks

The gate freezes source inputs and verifies their hashes before fresh compilation.
Rust formatting and workspace/all-target strict Clippy are mandatory. Only Cargo's
exact test-profile artifact is copied to this invocation's own output volume.
It verifies every tracked file in the staged Hermes source image before launch.

The actual controller runs as `999:999` with supplementary Docker socket group
access, not as root. Its private controller volume is mode `0700`. Each agent has
separate runtime/config/workspace/logs subpaths in an owned named volume, UID 999
files and mode `0600` dotenv. Agents never mount the Docker socket or private
controller state. Runtime mounts remain read-only.

The scenario checks real `/v1/runs`, final mirror messages and model-observed SOUL
isolation. Cross-agent API tokens are denied. Replaying the same message key does
not invoke the model or create a run again. A held model response proves that
configuration drains before changing files or replacing the original namespace.
After completion, a fresh activation generation must load the new SOUL; the
other agent's generation remains unchanged. Restarting the peer also creates a
fresh generation and preserves its effective SOUL.

This scenario does not yet inject readiness failure to prove actual Docker
rollback, restart the Fleet controller, restore lost private journals, test
actual Docker log ingestion or admit task-bound PM assignments. Focused fake
boundary tests for those paths are separate evidence, not substitutes.

## Evidence And Cleanup

The private invocation directory retains frozen sources, source hashes, image and
build logs, `live.log`, the sanitized `evidence/live-report.json` and `report.json`.
Only a passed live report can set actual Rust/Hermes acceptance fields true.
Permanent `sdlc1`, `sdlc2` and `sdlc-common` lifecycle facts are compared before and
after. The gate does not read installed secret values.

Cleanup verifies the original Engine and project/task/purpose ownership. It then
uses one exact union Compose manifest to stop initial services and generated
agent namespaces. Only the invocation's three disposable volumes are removed,
after confirming no container uses them. Own image aliases are removed. No global
prune, protected-volume replacement or default agent purge is allowed. A failed
gate must still report cleanup; unknown ownership refuses automatic deletion.

Current results and remaining gates belong in
[CHAT_CLARIFICATION_VERIFICATION](CHAT_CLARIFICATION_VERIFICATION.md),
[CURRENT_STATE](CURRENT_STATE.md) and [GAP_REGISTER](GAP_REGISTER.md).
