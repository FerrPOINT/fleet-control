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

Runtime integration history is not a release PR. Publish additive migrations
000010/000011 ->000012 ->000013 ->000014 in ordered task packets, at most one
new migration per PR, preserving applied bytes. Validate the actual release
head and its exact Base dependency, not an older PR's CI or another integration
snapshot. New journal-time CI has an isolated PostgreSQL database and cannot be
counted as executed when its environment variable is absent. Native/source QA
does not install a runtime or enable automatic task assignments.
