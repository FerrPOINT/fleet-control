# Testing

Backend checks:

```bash
cd backend
cargo fmt --check
cargo test --workspace -- --test-threads=1
```

Frontend checks:

```bash
cd frontend
pnpm typecheck
pnpm build
pnpm test
```

Required scenarios:

- create Developer Hermes and Tester Hermes
- verify `agent1` and `agent2` folder layout
- ensure distinct `HERMES_HOME` values
- reject path traversal
- start/stop/restart Hermes through a fake runtime command
- keep Java Agent operations typed as phase 2
- edit one agent's skills without changing another
- create a session and hand it off to another agent
