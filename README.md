# Fleet Control

Fleet Control is the SDLC control plane for managed agent runtimes. It starts
with isolated Hermes deployments and models Java Agent as the second runtime
type from day one.

## Snapshot

| Area | Value |
| --- | --- |
| Product | `fleet-control` |
| Backend | Rust 2024, Axum, SeaORM, PostgreSQL 17, Redis 8 |
| Frontend | React 19, Vite, Tailwind 4, shadcn-style primitives |
| API | REST + SSE, OpenAPI generated from Rust |
| Default ports | backend `23801`, frontend `23802` |
| Runtime types | `hermes`, `java_agent` |

## Purpose

Fleet Control manages a fleet of isolated agents:

- sequential identities: `agent1`, `agent2`, `agent3`
- per-agent runtime, config, workspace and logs folders
- runtime process lifecycle: provision, start, stop, restart, health
- per-agent skills and SOUL/config editing
- sessions/chats treated as task records
- workflow namespace bindings owned by `project-workflow`

The first implemented runtime is Hermes. Java Agent is present in the model,
API, create wizard and capability matrix, but launch/provision operations return
a phase 2 response until the adapter is implemented.

## Agent Layout

```text
data/agents/
  agent1/
    runtime/
    config/
    workspace/
    logs/
  agent2/
    runtime/
    config/
    workspace/
    logs/
```

For Hermes, Fleet Control launches the process with:

```text
HERMES_HOME=data/agents/agentN/config
cwd=data/agents/agentN/workspace
```

## Local Development

```bash
cp .env.example .env
docker compose up -d postgres redis
cd frontend && pnpm install && pnpm dev
cd backend && cargo run -p server
```

Open:

- frontend: http://127.0.0.1:5173
- backend: http://127.0.0.1:23801/api/v1/health
- API docs: http://127.0.0.1:23801/swagger-ui/

The first registered user becomes system admin.

## Quality

```bash
cd frontend && pnpm typecheck && pnpm build
cd backend && cargo fmt --check
```

Full backend compile/test requires a Rust Windows toolchain with a working C/C++
linker, or the Linux CI environment.

## Documentation

Start with:

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [docs/DATA_MODEL.md](docs/DATA_MODEL.md)
- [docs/API.md](docs/API.md)
- [docs/OPERATIONS.md](docs/OPERATIONS.md)
- [docs/TESTING.md](docs/TESTING.md)
- [docs/UI_UX.md](docs/UI_UX.md)
- [docs/contracts/AGENT_RUNTIME_CONTRACT.md](docs/contracts/AGENT_RUNTIME_CONTRACT.md)
