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
pnpm markdown:check
```

Additional gates:

- clean DB migration up/status
- OpenAPI regenerate and diff
- markdown link check through `pnpm markdown:check`
- visual review of desktop and mobile screenshots for leaders, sessions,
  settings, deployments and logs

Native Windows cargo commands require MSVC `link.exe`. The backend gate may be
run through WSL/Linux when the Windows-native linker is not installed.
