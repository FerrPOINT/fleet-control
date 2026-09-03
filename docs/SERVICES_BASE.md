# Services Base Integration

`FerrPOINT/services-base` is the shared foundation for SDLC Rust/React
services. Fleet Control is being aligned with it incrementally so the control
plane stays compatible with the existing application while moving toward common
fleet contracts.

## Current Integration

Fleet Control currently uses a local telemetry bridge at
`backend/shared/src/telemetry.rs`. It intentionally mirrors the
`sdlc-telemetry` API shape from `services-base` commit
`5e353e84aa99f459807aba3e31c24b8880eeceff`.

The sibling checkout is expected at `../services-base` during local fleet
development. It now includes the shared `auth-server`, `sdlc-auth-core`,
`sdlc-telemetry`, `sdlc-shared` and the React UI package that future services
should converge on.

Fleet Control currently wires:

- `shared::telemetry::init_tracing("fleet-control")` in the backend server
  entry point;
- `shared::telemetry::request_id_mw` as the outer API middleware.

Every API response receives an `x-request-id` header. The same id is inserted
into request extensions and emitted with method, path, response status and
latency in backend logs.

## Deliberately Deferred

Direct Cargo dependency on `services-base` is deferred because WSL Cargo and
WSL Git cannot currently authenticate to the private GitHub repository. Adding
the git dependency in that state breaks the WSL/Linux quality gate and would
likely break CI unless cross-repository credentials are configured.

`sdlc-auth-core` is not enabled yet. Fleet Control now issues local
browser-session JWTs with the shared crate's HMAC-compatible `aud`, `iss`,
`role`, `scopes` and optional `sid` shape. Legacy compact local tokens without
`aud` and `iss` remain accepted during the transition so existing browser
sessions keep working. Tokens that include fleet claims are validated strictly
against the configured issuer and audience.

The expected path is:

1. Keep `FLEET_CONTROL_AUTH__MODE=hmac` as the only active mode until the
   shared crate is reachable from WSL/CI.
2. Replace the local HMAC validator with `sdlc-auth-core::Validator::hmac`.
3. Add OIDC/JWKS mode backed by the shared `auth-server`/Rauthy deployment.
4. Remove the legacy compact-token validator after all local sessions have
   expired.

## Contract

- Do not import shared base crates through a local filesystem path; CI must be
  able to resolve the same dependency from GitHub.
- Pin git dependencies by commit until `services-base` publishes versioned tags.
- Keep `backend/shared/src/telemetry.rs` source-compatible with
  `sdlc-telemetry` until the direct dependency is available.
- Do not change Fleet Control public API or session ownership semantics as part
  of telemetry adoption.
- Any future shared-base auth migration must update `docs/SECURITY.md`,
  `docs/API.md`, `docs/ENV.md` and OpenAPI in the same change.
