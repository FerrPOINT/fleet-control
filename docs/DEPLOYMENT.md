# Deployment

Docker Compose starts PostgreSQL, Redis, backend and frontend.

```bash
POSTGRES_PASSWORD=change-me \
FLEET_CONTROL_JWT_SECRET=change-me-change-me-change-me \
FLEET_CONTROL_AUTH__MODE=hmac \
FLEET_CONTROL_AUTH__JWT_ISSUER=fleet-control \
FLEET_CONTROL_AUTH__JWT_AUDIENCE=sdlc \
docker compose up -d --build
```

Published defaults:

- backend: `127.0.0.1:23801`
- frontend: `0.0.0.0:23802`

Agent data is stored in the `agents_data` volume at
`/var/lib/fleet-control/agents` inside the backend container.

Release gate:

- Rust backend checks must pass with a working linker.
- Clean DB migrations must pass.
- OpenAPI must be regenerated from Rust source.
- Frontend build, tests, Playwright and screenshot manifest must pass.
- No deployment should claim green status while `link.exe` is missing on the
  Windows gate host unless the same backend gate is green in Linux/CI.
