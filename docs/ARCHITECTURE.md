# Architecture

Fleet Control keeps a small control-plane core:

```text
frontend -> api -> app services -> infra repository/provisioner/runtime
                              -> PostgreSQL
                              -> agent folders
                              -> Hermes process
```

## Backend

- `domain`: public types shared by API and OpenAPI.
- `app`: service contracts, auth and orchestration context.
- `infra`: SeaORM entities, PostgreSQL repository, filesystem provisioner and
  local runtime supervisor.
- `api`: Axum routes, auth middleware, REST/SSE and OpenAPI.
- `server`: migrations, dependency wiring, seed agents and process startup.

## Runtime Adapter Boundary

Runtime-specific behavior stays behind the supervisor/provisioner contracts.
Hermes and Java Agent differ in env vars, health checks, session APIs and launch
commands, but share the same agent/session/skill model.

## External Project Boundaries

`project-workflow` owns workflows and namespaces. Fleet Control stores bindings
only. `wiki` owns docs/evidence. `CI-CD` owns build/deployment pipelines.
