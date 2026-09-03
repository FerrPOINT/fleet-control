# ADR 0001: Rust and React Stack

## Status

Accepted.

## Context

Fleet Control belongs to the same SDLC suite as `task-tracker` and
`project-workflow`. The team already has working conventions for Rust services,
React shells, Docker Compose, generated OpenAPI clients, Playwright evidence and
documentation layout.

## Decision

Use the same Rust/Axum and React/Vite/Tailwind stack as the existing SDLC
applications.

## Consequences

- New developers can reuse the same local setup and quality gates.
- API contracts can be generated from Rust source and consumed by the frontend.
- UI routes can share the same shell and evidence workflow as neighboring apps.

## Alternatives

- Build a standalone desktop or CLI-only control plane. Rejected because this
  would fragment operator workflows and make screenshots/evidence weaker.
- Use a different backend stack. Rejected because it would add risk before the
  runtime/session architecture is stable.
