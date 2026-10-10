# Local Setup

The sibling `services-base` checkout is required for Cargo path dependencies
and the shared frontend package. Use the same parent directory for both repos.
Use [`.namespace-base-revision`](../.namespace-base-revision) when present;
[`.base-revision`](../.base-revision) retains the legacy SDK pin. CI and
standalone builds use this same selection. Runtime producer source and SDK
publication are separate; do not replace either pin with an arbitrary integration
head. See [Base integration](BASE_INTEGRATION.md).
SDLC is not enabled by running these services; see
[implementation and acceptance](SDLC_IMPLEMENTATION.md).

```bash
cp .env.example .env
# Replace FLEET_CONTROL_JWT_SECRET and runtime token secrets before backend run.
pnpm --dir ../services-base/frontend install --frozen-lockfile
cd frontend && pnpm install --frozen-lockfile
cd ../backend && cargo fetch --locked
```

Start the accepted local workspace from its root with PowerShell 7 and
`./start-local.ps1`; `-Workspace` selects `sdlc1` or `sdlc2`. Infrastructure belongs
to `sdlc-common`. Do not create a standalone permanent Fleet Compose project.
Disposable tests require an owned, labelled `sdlc-qa-*` project and exact cleanup.

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

## Configuration Activation Storage

Before activating a revision, the operator must provision persistent private
Linux storage for `FLEET_CONTROL_FLEET__CONTROLLER_ROOT`, outside the agents root
and every runtime mount. It must belong to the Fleet process UID with mode `0700`.
Do not assume a Windows bind mount enforces Linux ownership/permissions.
Empty, missing or unsafe storage holds activation before file/runtime effects;
Windows ACL support is not certified. Preserve legacy journals instead of
deleting them to unblock setup. See [environment](ENV.md#private-controller-storage)
and [recovery operations](OPERATIONS.md#private-activation-recovery-storage).
This prerequisite does not enable SDLC or certify runtime containment.

Local URLs:

- frontend: http://localhost:23802
- backend health: http://127.0.0.1:23801/api/v1/health
- API docs: http://127.0.0.1:23801/swagger-ui/

The frontend uses Central Auth SSO on `http://localhost:7701` by default. Its
`fleet-control` client must allow `http://localhost:23802/sso/callback`.
Central users need no local role assignment to operate Fleet. Their stored role
remains unchanged; the former bootstrap-admin subject setting is ignored. PATs
still require matching Fleet read/write scopes, and private chats remain
owner-only. Never match accounts by email.
