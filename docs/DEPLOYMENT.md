# Deployment

## Standalone Fleet Foundation

Requires Docker with BuildKit and Compose 2.17+ (named build contexts). Check out
`fleet-control` and a compatible `services-base` as adjacent directories. The
current foundation requires the Base revision recorded in
[CURRENT_STATE.md](CURRENT_STATE.md); merge Base before Fleet for default-branch
installations. `additional_contexts.services_base` supplies the Rust crates and
UI source without sending the entire SDLC workspace to Docker.

From `fleet-control`:

```bash
cp .env.example .env
# Replace POSTGRES_PASSWORD, FLEET_CONTROL_JWT_SECRET and
# FLEET_CONTROL_FLEET__RUNTIME_TOKEN_SECRET with separate random values.
# On Linux, restrict .env permissions with chmod 600 .env.
docker compose up -d --build --wait
curl -fsS http://127.0.0.1:23802/api/v1/health
```

Both published endpoints default to loopback: backend `127.0.0.1:23801`, frontend
`127.0.0.1:23802`. Explicit exposure via `BACKEND_HOST`/`FRONTEND_HOST` requires
TLS, correct cookie/CORS configuration and an access policy. PostgreSQL/Redis
are not published. Agent data is in `agents_data` at
`/var/lib/fleet-control/agents`; archive does not delete it.

This standalone recipe uses legacy HMAC authentication for private evaluation;
the first registered user on a clean database is the administrator. Complete
bootstrap on loopback before exposing the service. It is not the Central Auth
deployment: use the platform's authenticated umbrella deployment for SSO. Do not
infer shared identity or production isolation from standalone smoke success.

Hermes, its dependencies and model credentials are not bundled in these images.
Running real runtimes requires separately provisioned binaries, guarded agent
folders and verified runtime capabilities. Java jar lifecycle also requires an
external JDK/jar. Automatic SDLC is still blocked by the cross-service gates in
[SDLC_IMPLEMENTATION.md](SDLC_IMPLEMENTATION.md).

Nginx forwards both global and session-scoped SSE without buffering. Frontend
health includes a backend probe; backend startup applies database migrations.

## Disposable Container Acceptance

CI builds both release images with locked dependencies on a fresh PostgreSQL
database, then checks authentication/RBAC, private chat access, idempotency
replay/conflict, immediate session SSE through Nginx and persistence after backend
restart:

```bash
# Only against a disposable project with a fresh database and generated secrets.
docker compose -p fleet-ci up -d --build --wait
python3 scripts/compose_smoke.py --url http://127.0.0.1:23802 --compose-project fleet-ci
docker compose -p fleet-ci down --volumes --remove-orphans
```

The last command destroys only that disposable project's volumes. Never run it
against a deployment containing user data. Use alternate host ports when defaults
are occupied. Smoke does not start a provider-backed Hermes run or prove the
seven-agent SDLC acceptance flow. Migration, API drift, browser and secret gates
remain mandatory; see [QUALITY_GATE.md](QUALITY_GATE.md).
