# Release

Release checklist:

1. Update version/release notes.
2. Run backend format, check, clippy and tests.
3. Run clean DB migration up/status.
4. Regenerate OpenAPI from Rust source.
5. Run `pnpm generate:api`.
6. Run frontend typecheck, lint, format check, tests and build.
7. Run Playwright e2e.
8. Regenerate screenshots and verify manifest.
9. Run markdown link check.
10. Review `GAP_REGISTER.md` and `RISK_REGISTER.md`.
11. Confirm no product blockers remain.

No release can be marked green while the Rust toolchain cannot compile or while
OpenAPI generation is blocked.
