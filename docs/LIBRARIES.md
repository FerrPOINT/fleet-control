# Libraries

Backend:

- Axum for HTTP routing.
- SeaORM for PostgreSQL persistence.
- Utoipa for OpenAPI source generation.
- Tokio for async runtime and process supervision.
- Serde for JSON DTOs.
- UUID and time crates for ids/timestamps.
- SHA-2 and hex for idempotency payload hashes.

Frontend:

- React and React Router for SPA routes.
- TanStack Query for server state.
- Vite for build/dev server.
- Tailwind CSS for styling.
- Lucide React for icons.
- Playwright and Vitest for e2e/unit tests.

Do not add new major dependencies without documenting the reason in this file
or an ADR.
