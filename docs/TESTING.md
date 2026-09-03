# Testing

Backend checks:

```bash
cd backend
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --test-threads=1
```

Frontend checks:

```bash
cd frontend
pnpm generate:api
pnpm typecheck
pnpm lint
pnpm format:check
pnpm test
pnpm build
pnpm exec playwright test
pnpm screenshots:local
pnpm screenshots:verify
```

Required scenarios:

- create Developer Hermes and Tester Hermes
- create IT Lead Hermes and assign Developer/Tester executors
- verify `agent1` and `agent2` folder layout
- ensure distinct `HERMES_HOME` values
- reject path traversal
- reject absolute paths outside the configured agents root
- start/stop/restart Hermes through a fake runtime command
- reconcile a tracked Hermes process that exits unexpectedly
- keep Java Agent operations typed as phase 2
- edit one agent's skills without changing another
- enforce `admin`, `operator` and `user` RBAC at backend routes
- direct executor session is private by default
- direct leader session selects itself as leader
- child executor session from a leader chat records parent and leader
- default session API filter returns the current user's sessions
- admin/operator multi-user filter can expand to all users
- normal users cannot read all users or expand session filters
- selecting a leader for an executor session requires `leader_executors`
- session and message idempotency replay returns the original row
- session and message idempotency conflict returns `409`
- create a mirrored session message and dispatch through the runtime boundary
- create a session and hand it off to another agent
- create a leader delegation and verify parent/child linkage
- list session participants from `/sessions/{id}/participants`
- create/list/cancel deployment jobs
- load/update runtime, ports, integrations and auth settings with redaction
- mutating runtime, leader, session, skill and config actions create redacted
  audit entries
- logs UI separates process logs, events and audit trail
- settings UI supports user role changes
- screenshot manifest is generated and contains the required viewports/routes
