# Quality Gate

Commands:

```bash
cd backend
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --test-threads=1
```

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
pnpm chat:evidence:verify
pnpm controls:evidence:verify
pnpm markdown:check
```

Additional gates:

- clean DB migration up/status
- additive control-ledger 000013 upgrade, empty down/re-up and nonempty refusal
  on its own database; release only after the 000012 prerequisite
- OpenAPI regenerate and diff
- opt-in recovery source gate: original journal/scope/epoch and DB-lock expiry
  races; Base plugin Linux SQLite/auth/boundary suite; actual pinned native
  `scripts/hermes_protocol_live/run.py --scenario recovery` with an explicitly
  supplied compatible plugin source and existing image. These component gates
  do not enable installed runtime or replace managed Fleet/native acceptance.
- standalone release Docker builds with sibling Base named contexts and locked
  dependencies; fresh Compose health plus `scripts/compose_smoke.py` acceptance
  (auth/RBAC, privacy, idempotency, proxied session SSE and restart persistence)
- markdown link check through `pnpm markdown:check`
- visual review of desktop screenshots for leaders, sessions, settings,
  deployments and logs; narrow-viewport behavior is verified by UI tests.
- generated JSON/Markdown screenshot proof equality and PNG hashes/dimensions;
  capture rejects missing fixture routes, while visual review checks rendered content.

Native Windows cargo commands require MSVC `link.exe`. The backend gate may be
run through WSL/Linux when the Windows-native linker is not installed.
