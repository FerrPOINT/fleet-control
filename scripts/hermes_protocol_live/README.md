# Native Hermes Protocol Acceptance

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
