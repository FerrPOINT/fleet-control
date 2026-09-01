# Contributing - Fleet Control

## Local Setup

```bash
git clone git@github.com:FerrPOINT/fleet-control.git
cd fleet-control
cp .env.example .env
cd frontend && pnpm install
cd ../backend && cargo fetch
```

## Development Rules

- Keep the runtime boundary clean: shared behavior belongs in the common agent
  model, runtime-specific behavior belongs in adapters.
- Do not let two agents share one config root.
- Store workflow namespace bindings only; workflow definitions stay in
  `project-workflow`.
- Use generated OpenAPI for public API changes.
- Update docs when changing runtime behavior, DB schema or screens.

## Quality

```bash
just fmt-check-rust
just typecheck
just build-frontend
```

Before pushing larger backend changes, run:

```bash
just test-backend
just test-frontend
```

## Commit Style

Use Conventional Commits, for example:

```text
feat: add hermes provisioning
docs: describe java agent adapter contract
```
