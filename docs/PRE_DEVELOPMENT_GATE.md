# Pre-Development Gate

Goal: start main development only after architecture, API, data model, UI,
docs, screenshots, tests, CI and local environment are synchronized.

Required before green:

- RBAC matrix implemented and tested.
- Session privacy/leader/delegation rules implemented and tested.
- Runtime adapter boundary documented.
- Hermes runnable path implemented.
- Java Agent phase 2 contract visible and typed.
- All product and admin pages exist with state handling.
- Screenshot manifest generated for required viewports.
- OpenAPI regenerated from Rust source.
- Frontend generated types updated.
- Backend and frontend quality commands pass.
- Clean DB migration status passes.
- Known product blockers closed or explicitly accepted.

Current result:

- Product/API/UI/docs/screenshot hardening is implemented.
- Rust checks and OpenAPI regeneration pass through WSL/Linux.
- Native Windows Rust commands still require MSVC `link.exe`.
