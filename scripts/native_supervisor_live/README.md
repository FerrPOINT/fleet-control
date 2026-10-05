# Managed Native Supervisor Gate

Opt-in Linux acceptance of Fleet's actual provisioner, configuration activation,
supervisor, prompt outbox/dispatcher, transcript mirror and restart readback against
the real pinned Hermes gateway CLI/API/AIAgent. Only the model is a deterministic
loopback OpenAI fixture. No fake Hermes HTTP server, model credentials, paid
inference, installed Fleet process or accepted runtime HOME is used.

The default `lifecycle` case in the ignored `infra/tests/native_supervisor_live.rs`
target creates two concrete
agents with installed skill content and separately activated renderer-2 revisions.
It verifies separate HOME/workspace/ports, loaded distinct SOUL, cross-token
denial, original run/transcript identity, exact-once prompt replay, one assistant
mirror, preserved native status after process restart and tracked parent stop.
The test runs as the dependency image's non-root user; root override is forbidden.
It checks private mode-0600 dotenv ownership. These are not OS/tool isolation,
descendant quiescence or task/PM admission proofs. Unknown-ACK recovery is a
separate `recovery` case below, not a claim made by the lifecycle case.

## Lost Acknowledgement And Fleet Process Restart

Pass `--scenario recovery` to select only
`managed_native_lost_ack_recovers_original_run_across_fleet_processes`.
The harness copies the four recovery plugin files from the exact committed Base
revision and verifies their byte hashes before startup. A separate QA-only platform
plugin closes the connection after the real native handler returned `202`; it
never invents an accepted run, rewrites the request, or replaces Hermes inference.
Authentication precedes its lookup hold, observations and fault injection.

The first Fleet subprocess activates the agent and dispatches through its real
prompt outbox. A QA lookup barrier keeps the submitted journal without a native
ID and capacity held. That Fleet process exits without destructors. The parent
owns only the local model and test coordination, never a runtime supervisor;
it verifies native completion via authenticated GET before removing the barrier.
A distinct Fleet subprocess then restores the original mapping using the actual
Base witness lookup and terminal GET. The test requires unchanged original
request hash/key/origin/credential context/capabilities/horizon, one native POST,
one inference, one run and one assistant mirror, with no native SSE request.

This case covers recovery of an already-terminal run after Fleet exits, not a
running/approval recovery or native gateway crash. Orphan gateway cleanup belongs
to the disposable Compose namespace, not to a proven safe-stop ownership transfer.
Secrets and the synthetic prompt remain inside QA tmpfs; observations contain
only operation kind, hashes/IDs and lookup hold state. The fault plugin is not
installed in any accepted runtime or Base production plugin.

## Native Run Controls

`--scenario controls` selects the real AIAgent steer/stop case. A loopback model
barrier keeps one original inference active while Fleet verifies native
capabilities/run/session and sends steer, then stop. Native status must record
the steer; interrupt ACK cannot become completed or an assistant result. After
the barrier releases, native terminal readback must retain the original journal
and run, with one inference and rejected late steer. This is not approval/tool
replay, per-command recovery, a task stage receipt or OS-descendant safe stop.

## Original Control Outcome Recovery

`--scenario control-outcomes` selects
`managed_native_original_control_outcomes_recover_lost_http_ack`. It enables the
Fleet outcome consumer only in its disposable agent. The harness copies all four
control plugin files from the exact committed Base revision and checks their
hashes. A separate QA-only observer loses actual steer/stop HTTP200 responses
after the native handler applies the action; it never fabricates an ACK or
modifies the witness store. Authentication precedes all fault observations.

The test holds GET lookup until the caller receives unknown acceptance, then
requires original-key GET recovery without another POST. Stop recovery follows
independent terminal observation and an actual gateway PID restart with the same
HOME. The late ACK must preserve terminal receipt state/timestamp and the original
private context. Each command must have one POST and one real native ACK, while
the original run/journal and single inference remain unchanged. This is not a
Fleet OS-process restart, approval decision recovery, loaded-generation admission,
task/PM proof or safe descendant stop. The legacy `controls` case remains separate.

## Control Outcomes Across Fleet Process Death

`--scenario control-restart` selects
`native_control_restart::managed_native_control_outcomes_survive_fleet_process_death`.
It uses three distinct Fleet OS processes and one surviving real Hermes gateway.
The first sends steer, loses the real ACK, saves its durable context and is killed
with SIGKILL. The second replays without POST while lookup is held, recovers steer
by the original-key GET, sends stop, loses its real ACK and is also SIGKILLed.
The third reads the interrupted native run, records independent terminal state,
then recovers the late stop ACK by GET without changing the terminal timestamp.

The parent observes the actual loopback inference barrier before the first
control, not just a running DB state. The test asserts one POST/real native ACK
per command, one ACK audit entry, unchanged private contexts and dispatch identity,
the same gateway PID, one inference and no false assistant completion. It checks
both SIGKILL exit statuses, not a second supervisor object or graceful shutdown.
Preflight verifies every pinned Hermes source file and all four committed control
plugin files before starting. This does not attest approval decision recovery,
task authority, central-auth integration, combined extensions or safe OS descendants.

## Native Exact-Action Approvals

`--scenario approvals` selects
`native_approvals::managed_native_approval_decisions_are_exact_once_and_unknown_ack_is_held`.
The local model requests a terminal command changing the permissions of one
disposable file in that agent's workspace. The real Hermes guard must emit the
approval request; no test inserts an approval into Fleet or Hermes storage.
The three independent chats exercise owner `once`, `deny` and a lost real ACK.
The test checks the actual file mode, one native POST per decision, unchanged
dispatch identity, terminal mirror, same-key replay and changed-payload conflict.
An unrelated human and unsupported `always` choice must not reach Hermes.

Only the QA-only approval observer loses the final ACK, after the exact native
handler has applied and acknowledged that one decision. Authentication comes
before body reads and observations. The plugin requires an explicit QA opt-in
and its existing fixed Linux tmpfs root; it is not installed in accepted agents.
Observations contain action identifiers and choices, never tokens or commands.
After the lost ACK, Fleet must retain `uncertain` even when the run completes;
neither replay nor terminal state may authorize a second decision POST.
This is local human HTTP authentication, not central JWKS/PAT, fenced task
admission, recovery of a waiting approval after a Fleet crash or safe OS stop.

## Original Approval Decision Outcomes

`--scenario approval-outcomes` selects
`native_approvals::managed_native_original_approval_outcomes_recover_lost_http_ack`.
It reuses the real terminal guard/owner HTTP scenario with the exact committed
four-file Base control plugin and the default-off Fleet consumer enabled only
inside its disposable agent. The legacy `approvals` scenario remains separate.

The observer closes the actual ACK transport and holds only the original GET
witness. While that GET is held, completion of the native run must leave the
decision uncertain. Releasing the hold must settle it by original-context GET,
not another POST or terminal inference. The test checks original decision UUID,
store epoch, exact request hash, once/deny file modes, one POST/native ACK/audit,
unchanged context/dispatch/terminal timestamp and replay/conflict behavior.
It does not substitute a fake approval request, handler or witness. This is not
Fleet OS-process death, combined recovery extensions, task/PM admission or safe
descendant stop. Preserve actual evidence after running it; source alone is not
accepted runtime behavior.

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
- For `recovery`, that same committed revision must include the complete
  `deploy/hermes-recovery-plugin` inventory. Missing files fail before Docker starts.
- For `control-outcomes`, `control-restart` and `approval-outcomes`, it must include the complete four-file
  `deploy/hermes-control-plugin` inventory. The fault observer stays in QA and is
  never copied into accepted runtime agents.
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
native service hashes and runs that binary with the selected exact test name.
The final gate requires one named
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

The report also fingerprints the repository delivery/journal, managed-file
persistence and runtime context/control/approval source files. These hashes bind
the executed candidate behavior to source bytes; they do not certify complete configuration generation,
task admission or release-head dependency compatibility.

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

The `approval-recovery` scenario uses two distinct Fleet OS processes and one
surviving real Hermes gateway. A loopback model waits until original run/session
pinning; an opt-in QA observer then loses actual waiting GET responses and rejects
the original SSE request. The first Fleet process exits before storing an
approval. The second restores that exact current request by GET, sends one owner
HTTP once decision, observes the real chmod effect and stores one final answer.
The observer never manufactures native approval/status/tool events. Only the
owned Compose namespace reaps the orphan gateway; this is not a safe-stop proof.
Run with `--scenario approval-recovery`; gate results belong to the exact source
hashes and binary in evidence, not an older PR's CI or an installed runtime.

Missed historical tool/approval replay, unknown decision outcome lookup, complete
loaded-config/plugin inventory, process-tree safe stop, central Fleet auth/UI,
assignment/first-step/PM continuation and full SDLC/deployment remain separate
acceptance requirements. A completed chat run is not a Tracker stage receipt.
