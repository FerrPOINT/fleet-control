# Deployment

Docker Compose starts PostgreSQL, Redis, backend and frontend.

```bash
POSTGRES_PASSWORD=change-me \
FLEET_CONTROL_JWT_SECRET=change-me-change-me-change-me \
docker compose up -d --build
```

Published defaults:

- backend: `127.0.0.1:23801`
- frontend: `0.0.0.0:23802`

Agent data is stored in the `agents_data` volume at
`/var/lib/fleet-control/agents` inside the backend container.
