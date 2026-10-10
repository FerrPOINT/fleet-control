# Local Setup

The sibling `services-base` checkout is required for Cargo path dependencies
and the shared frontend package. Use the same parent directory for both repos.
Use the exact published Base revision recorded in `.base-revision`;
do not replace it with an arbitrary older checkout.
SDLC is not enabled by running these services; see
[implementation and acceptance](SDLC_IMPLEMENTATION.md).

## Installed Workspace

Follow the installation root's `AGENTS.md` and Base's
`services-base/deploy/LOCAL_GROUPS.md`. Permanent Compose groups are `sdlc1`,
`sdlc2` and `sdlc-common`; do not start a standalone Fleet/PostgreSQL group next
to them. From the prepared workspace root, use PowerShell 7:

```powershell
./start-local.ps1 -Workspace sdlc1 -Status
./start-local.ps1 -Workspace sdlc1
python services-base/scripts/audit_docker_groups.py
```

The launcher uses accepted snapshots, not this candidate source. Do not replace
missing snapshots/secrets/volumes with empty resources or stop another workspace
to free a development port. An unprepared installation must follow Base's
workspace setup instructions first.

## Source Checks

Use an actual clean sibling Base checkout at the pinned revision. A Windows
junction is not equivalent: Tailwind's explicit Base source scan can omit shared
utilities even when package imports and the build succeed. See
[Base integration](BASE_INTEGRATION.md).

From the Fleet repository root:

```bash
pnpm --dir ../services-base/frontend install --frozen-lockfile
pnpm --dir frontend install --frozen-lockfile
cargo fetch --manifest-path backend/Cargo.toml --locked
```

Create a private, ignored `.env` only if one does not already exist; use
[ENV.md](ENV.md) for database, auth and runtime credentials. Source dependencies
do not provision an accepted runtime or grant access to its database.

Runtime/DB QA must use a task-owned temporary Compose project
`sdlc-qa-<task>-<unique>` or `sdlc-build-<task>-<unique>` through Base's qualified
helpers, with `sdlc.task`/`sdlc.purpose` labels and exact-project cleanup in
`finally`/`trap`. Direct Docker run/create and global prune are prohibited.

For isolated source development only, after allocating an authorized database
and unused ports, run backend from the repository root:

```bash
cargo run --manifest-path backend/Cargo.toml -p server
```

Run frontend from the same root:

```bash
pnpm --dir frontend dev --host 127.0.0.1
```

The URLs below are source-development defaults. If an installed workspace owns
those ports, configure separate ports and matching CORS/SSO redirects before
starting a candidate; never stop or silently reuse its servers.

Default local URLs:

- frontend: http://localhost:23802
- backend health: http://127.0.0.1:23801/api/v1/health
- API docs: http://127.0.0.1:23801/swagger-ui/

The frontend uses Central Auth SSO on `http://localhost:7701` by default. Its
`fleet-control` client must allow `http://localhost:23802/sso/callback`.
Central users need no local role assignment to operate Fleet. Their stored role
remains unchanged; the former bootstrap-admin subject setting is ignored. PATs
still require matching Fleet read/write scopes, and private chats remain
owner-only. Never match accounts by email.
