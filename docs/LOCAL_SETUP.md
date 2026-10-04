# Local Setup

The sibling `services-base` checkout is required for Cargo path dependencies
and the shared frontend package. Use the same parent directory for both repos.
For this foundation branch, use Base revision
`d03096d4f21cff5231e23d7a8744c51413d25569`
([dependency PR](https://github.com/FerrPOINT/services-base/pull/121));
an older `main` lacks the shared AppShell and durable-stream helpers.
SDLC is not enabled by running these services; see
[implementation and acceptance](SDLC_IMPLEMENTATION.md).

```bash
cp .env.example .env
# Replace FLEET_CONTROL_JWT_SECRET and runtime token secrets before backend run.
docker compose up -d postgres redis
pnpm --dir ../services-base/frontend install --frozen-lockfile
cd frontend && pnpm install
cd backend && cargo fetch
```

Run backend:

```bash
cd backend
cargo run -p server
```

Run frontend:

```bash
cd frontend
pnpm dev --host 127.0.0.1
```

## Managed Hermes command

Запуск backend/frontend не делает raw Hermes CLI совместимым с Fleet. Для
pinned Hermes `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3` raw `hermes serve`
запускает dashboard/headless web server, не gateway API. Текущий Fleet launch
требует Base `services-base/deploy/fleet-hermes-launch.py`, который переводит
`serve --host ... --port ...` в `gateway run` через `API_SERVER_HOST` /
`API_SERVER_PORT`.

В packaged image managed command задан как
`FLEET_CONTROL_FLEET__HERMES_COMMAND=/opt/fleet-hermes/bin/hermes`.
При host запуске настройка должна указывать на deployment-provisioned
совместимый wrapper; этот container path не означает, что wrapper установлен
на host. Default имя `hermes` в PATH не гарантирует wrapper: raw upstream CLI
не подходит для текущего argv.

Dotenv может переопределить host/port после wrapper; versioned renderer и
native acceptance остаются отдельными gaps. Не переписывать старые effective
snapshots для перехода на новые defaults. См.
[runtime](RUNTIME.md) и [контракт адаптера](contracts/HERMES_ADAPTER_CONTRACT.md).
Эта настройка не включает native-ready admission или автоматический SDLC.

Local URLs:

- frontend: http://localhost:23802
- backend health: http://127.0.0.1:23801/api/v1/health
- API docs: http://127.0.0.1:23801/swagger-ui/

The frontend uses Central Auth SSO on `http://localhost:7701` by default. Its
`fleet-control` client must allow `http://localhost:23802/sso/callback`.
Do not infer an admin role from successful SSO. Configure a verified
`FLEET_CONTROL_AUTH__BOOTSTRAP_ADMIN_SUB` only for initial bootstrap, or assign
the local role through an existing admin. Never match accounts by email.
