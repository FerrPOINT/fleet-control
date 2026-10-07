# Native Hermes Protocol Acceptance

## Control Outcome Producer

Explicit `--scenario controls --control-plugin-root <Base/deploy/hermes-control-plugin>`
selects the new producer, never installs it into an accepted runtime. The two
cases run real native API/AIAgent steer/interrupt, drop their actual HTTP ACKs,
read original-key outcomes after gateway restart, and retain a rejected/unknown
command without reexecuting its handler. Source/plugin/helper/probe/log hashes
and exact Compose cleanup are recorded. Native source and dependency image stay
pinned; the model is loopback. This is not production Fleet outcome recovery,
positive native tool approval, CLI lifecycle, central identity or safe OS stop.
See [wire v1](../../docs/contracts/HERMES_CONTROL_OUTCOME_V1.md).

## Baseline Protocol

This opt-in source gate runs the actual pinned Hermes `APIServerAdapter`, real
`AIAgent`, native auth/profile middleware and SQLite in separate Python processes.
Only the upstream model is a deterministic, loopback OpenAI fixture. It does not
modify Hermes, use a fake Hermes HTTP server, require paid inference, or attach
to the installed fleet. This is native protocol evidence, not native gateway CLI
lifecycle, tool/approval execution, Fleet integration, PM admission or SDLC success.

```powershell
python scripts/hermes_protocol_live/run.py `
  --hermes C:/git/azhukov/sdlc/прототипы/hermes `
  --image sdlc-fleet-canonical-runtime:20261003-r1
```

The clean source must be `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3`.
Supply an already-built Base-packaged dependency image with that revision label;
the harness resolves its immutable image ID and compares `uv.lock`/`pyproject.toml`
inside it with the pinned source. It never installs or updates runtime images.
Source is extracted from Git into an isolated read-only snapshot. No host runtime
folders, credentials, volumes or published ports are mounted. The temporary
Compose project has owner/purpose labels, an internal network, a read-only root
and disposable tmpfs homes. All native requests and model traffic use loopback.

Cases cover parsed native SSE and terminal success flags, two independent homes,
foreign token/profile rejection, an actually dropped HTTP 202, eight concurrent
exact-key replays, changed-payload conflict, killed/restarted process with the same
SQLite and token, unavailable SSE after restart with durable status readback,
and credential rotation. Rotation intentionally shows that the same key under a
new credential can start another native run: Fleet must guard its recovery scope.
An independent inference barrier also kills the native process after a lost ACK
but before completion/replay: the original run must recover as interrupted,
without another model call. Requested session identity and native transcript
are checked, not merely a nonempty ID. Unknown profile rejection does not prove
multi-profile multiplexing isolation or compression/session-rotation behavior.

Secrets are generated inside the disposable process, never placed in tracked
Compose or evidence. Native logs remain private in tmpfs and are not published.
Evidence contains source/image/harness/log hashes and named successful cases.
The runner always performs exact-project `down --remove-orphans` and checks for
remaining containers; it does not prune or delete shared caches/volumes. After
external interruption, inspect and clean only the exact project recorded in its
artifact directory before rerunning. Results remain under ignored `tmp/`.
Native lazy installs, auto-title generation, background review and memory are
explicitly disabled using upstream settings. SSE reads and the Compose runner
have deadlines; a timeout is failure with retained partial diagnostics, not success.

## Original-Key Recovery Scenario

`--scenario recovery --recovery-plugin-root <Base-deploy/hermes-recovery-plugin>`
snapshots the explicit candidate plugin into disposable homes and enables only
those QA configs. Native discovery and the supported platform handler hook are
used; no native route monkeypatch or installed agent is involved. The producer
module bytes are hashed in evidence. Cases require a real dropped 202, process
restart, eight non-dispatch original-key lookups, native terminal/session readback
and exactly one inference. A separate prune/tombstone/reset scenario denies
replacement run admission and old epoch. Plugin absence in normal scenarios is
unchanged. This proves native extension interoperability, not Fleet end-to-end
integration, installed enablement, task/PM admission or OS sandbox.

## Actual Rust Renderer Scenario

The renderer fixture also carries the same sealed skill-discovery policy used
by pinned Base package preparation. The native consumer creates only disposable
synthetic HOME/project/external skill directories. An unsealed negative control
must discover the extra roots; the actual Rust policy must exclude them and
resolve the HOME skill without a same-name shadow. No model is run, private Base
text is exported, plugin inventory is attested or runtime admission is granted.
Both real rendered agents must pass this loader-only check.
The same native helper must read synthetic unlisted Markdown, Python and `.bin`
asset files with UTF-8 fixture bytes from an allowed skill directory. This
negative control motivates the production HOME guard that rejects all unattested
support files; it does not
execute those files, read private package instructions or attest plugin behavior.

The optional `--scenario renderer` consumes the private fixture generated by the
ignored Rust test `versioned_renderer_export_native_fixture`. In an owned Linux
QA Compose service mount a disposable evidence directory at `/renderer-evidence`
and set `FLEET_RENDERER_EVIDENCE_ROOT=/renderer-evidence`, then run:

```bash
cargo test --locked -p infra --lib versioned_renderer_export_native_fixture -- --ignored --test-threads=1
```

The exporter creates two synthetic agent homes using the actual provisioner and
renderer; it is not a hand-written imitation. Its `.env` contains synthetic
derived credentials, so keep the directory ignored/private and never attach
accepted agent homes or production secrets. Run the consumer with the same pin
and already-built dependency image:

```powershell
python scripts/hermes_protocol_live/run.py `
  --hermes C:/git/azhukov/sdlc/прототипы/hermes `
  --image sdlc-fleet-canonical-runtime:20261003-r1 `
  --scenario renderer `
  --renderer-evidence-root <owned-private-fixture-directory> `
  --artifacts tmp/hermes-renderer-live
```

The evidence mount is read-only. The loader-only service uses UID 0 to match the
Rust QA exporter that owns the mode-0600 fixture, with all capabilities dropped
and no-new-privileges. File modes are not relaxed; protocol and installed runtime
users are unchanged. This does not certify production UID/OS isolation.
Independent native processes verify exact file
hashes, direct YAML processing without swallowed exceptions/env fallback, then
combined dotenv/config precedence against stale shell host/port/key. HOME, port,
credential hashes and configured CORS must match and identities must be distinct.
There is no model call or listener startup: this proves loading Rust-rendered
configuration, not gateway lifecycle, native storage, effective installed config
attestation or SDLC admission. The same exact-project cleanup/deadlines apply.
