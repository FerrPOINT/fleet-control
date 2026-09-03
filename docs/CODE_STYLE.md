# Code Style

Backend:

- Rust 2024.
- `cargo fmt --all` is mandatory.
- Prefer domain DTOs at route boundaries.
- Keep repository code responsible for database mapping.
- Do not leak SeaORM entities into frontend contracts.
- Use typed enums in domain and checked text values in database.

Frontend:

- TypeScript strict mode.
- Generated API types must stay aligned with OpenAPI.
- Keep page-level data loading explicit with TanStack Query.
- Prefer small shared helpers over broad abstractions.
- Avoid visible instructional copy that explains controls already visible in the
  UI.

Docs:

- Update docs in the same change as API/data model/UI behavior changes.
