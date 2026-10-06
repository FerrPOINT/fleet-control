# Local Setup

The sibling `services-base` checkout is required for Cargo path dependencies
and the shared frontend package. Use the same parent directory for both repos.
Use the exact published Base revision recorded in `.base-revision`;
do not replace it with an arbitrary older checkout.
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
