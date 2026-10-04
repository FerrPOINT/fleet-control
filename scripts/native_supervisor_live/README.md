# Managed Native Supervisor Gate

Opt-in Linux acceptance of Fleet's actual provisioner, configuration activation,
supervisor, prompt outbox/dispatcher, transcript mirror and restart readback against
the real pinned Hermes gateway CLI/API/AIAgent. Only the model is a deterministic
loopback OpenAI fixture. No fake Hermes HTTP server, model credentials, paid
inference, installed Fleet process or accepted runtime HOME is used.

The ignored `infra/tests/native_supervisor_live.rs` target creates two concrete
agents with installed skill content and separately activated renderer-2 revisions.
It verifies separate HOME/workspace/ports, loaded distinct SOUL, cross-token
denial, original run/transcript identity, exact-once prompt replay, one assistant
mirror, preserved native status after process restart and tracked parent stop.
The test runs as the dependency image's non-root user; root override is forbidden.
It checks private mode-0600 dotenv ownership. These are not OS/tool isolation,
descendant quiescence, unknown-ACK recovery or task/PM admission proofs.

## Prerequisites

- Clean Hermes source at `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3`.
- BuildKit access to stage the exact clean Git archive in a disposable QA layer
  over the dependency image. No mutable source bind, network build step or
  runtime HOME enters that layer; the archive cannot contain `.venv` or links.
  Before model startup, every tracked file hash is compared with the immutable
  archive. Dependency image labels alone are not byte attestation. Its existing
  virtual environment and non-root user remain unchanged. This is a QA candidate
  image, never a replacement for the accepted runtime image.
- A clean Base SDK checkout matching Fleet `.base-revision` for offline compilation.
- A Base Git checkout supplying `deploy/fleet-hermes-launch.py`; the launcher is
  copied from its exact committed Git blob, not mutable worktree content.
- An already-built Base Hermes dependency image with the matching revision label
  and non-root default user. The harness builds only the owned QA source layer;
  it never installs or retags accepted runtime images.
- Explicit existing build-cache volume names. No cache or accepted volume is
  deleted; these are external Compose volumes, not QA database storage.

```powershell
python scripts/native_supervisor_live/run.py `
  --hermes <clean-hermes-checkout> `
  --base-sdk <exact-clean-sdk-checkout> `
  --base-checkout <base-launcher-git-checkout> `
  --image <existing-base-hermes-dependency-image> `
  --target-cache <existing-rust-target-cache> `
  --cargo-cache <existing-cargo-cache> `
  --rustup-cache <existing-rustup-cache>
```

The build service uses Rust1.88, locked/offline dependencies, fmt/all-target check
and strict native-test Clippy. Compiler JSON supplies exactly one executable; the
native service hashes and runs that binary. The final gate requires one named
test passed, no failures/ignored tests and the complete source preflight; a zero
exit with zero tests is rejected. PostgreSQL17.6 and both HOME directories are
disposable tmpfs. Services share only the owned internal network, publish no host
ports and have explicit `sdlc.task`/`sdlc.purpose` labels. Native model/runtime
traffic is loopback; the service has a read-only root, dropped capabilities and
no-new-privileges. This configuration is not a production sandbox certification.

## Evidence And Cleanup

Ignored `tmp/native-supervisor-live/` holds exact source/image/launcher/test/harness
hashes, build/native logs and cleanup evidence. A dirty Fleet source is recorded,
not misrepresented as an exact-head CI gate. Generated secrets remain in private
tmpfs and are not exported. Diagnostic startup logs come from Fleet redaction,
with the known synthetic/derived keys additionally masked. Previous failed
attempts remain failures and must not be counted as passing acceptance.

The runner always issues exact-project `down --remove-orphans`, removes only its
unique QA source/dependency-alias tags and independently checks for remaining
containers and those tags. Timeout/failure retains diagnostic evidence and
does not certify readiness. Cleanup failure invalidates the result. After an
external interruption, reconcile only the exact project recorded in evidence;
never prune other containers, caches, images or volumes. No repository migration,
SDK/package pin or installed runtime is changed by this gate. The shared BuildKit
cache and dependency image are preserved; no image/volume prune is used.

Host-only CI tests:

```bash
python3 -B -m unittest discover -s scripts/native_supervisor_live -p test_harness.py -v
```

Managed lost-ACK/original-key recovery, native tool/approval replay, complete
loaded-config/plugin inventory, process-tree safe stop, real Fleet HTTP auth/UI,
assignment/first-step/PM continuation and full SDLC/deployment remain separate
acceptance requirements. A completed chat run is not a Tracker stage receipt.
