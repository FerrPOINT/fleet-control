# AGENTS.md

## Project

This repository is `fleet-control`, a Rust/React SDLC control plane for managed
agent runtimes.

## Stack

- Backend: Rust 2024, Axum, SeaORM, PostgreSQL 17, Redis 8.
- Frontend: React 19, Vite, Tailwind 4, shadcn-style local primitives.
- API: OpenAPI generated from the Rust `api` crate.
- Env prefix: `FLEET_CONTROL_`.
- Default ports: backend `23801`, frontend `23802`.

## Boundaries

- Do not mutate sibling SDLC repositories while working here.
- `task-tracker` is a reference/template only.
- Hermes is the first implemented runtime.
- Java Agent must remain represented in types, API, docs and UI, but process
  launch/provision is phase 2 until the adapter is implemented.
- `project-workflow` owns namespace/workflow definitions. Fleet Control stores
  bindings and connection status.

## Runtime Layout

Every agent is created from a database ordinal:

```text
agents/agentN/runtime
agents/agentN/config
agents/agentN/workspace
agents/agentN/logs
```

For Hermes, always launch with `HERMES_HOME=agents/agentN/config` and
`cwd=agents/agentN/workspace`. Never point two managed agents at the same
`HERMES_HOME`.

## Development Rules

- Keep public API changes reflected in `docs/API.md`, `docs/DATA_MODEL.md` and
  `openapi/openapi.json`.
- Guard all filesystem paths under the configured agents root.
- Redact secrets before storing logs or returning env data through the API.
- Use existing frontend primitives from `src/shared/ui`.
- Capture UI evidence for new/changed screens under `docs/assets/screens`.
