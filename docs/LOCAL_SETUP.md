# Local Setup

```bash
cp .env.example .env
# Replace FLEET_CONTROL_JWT_SECRET and runtime token secrets before backend run.
docker compose up -d postgres redis
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
pnpm dev
```

Local URLs:

- frontend: http://127.0.0.1:5173
- backend health: http://127.0.0.1:23801/api/v1/health
- API docs: http://127.0.0.1:23801/swagger-ui/
