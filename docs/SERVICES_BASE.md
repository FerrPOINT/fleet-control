# Services Base Integration

`FerrPOINT/services-base` is the shared foundation for SDLC Rust/React
services. Fleet Control is being aligned with it incrementally so the control
plane stays compatible with the existing application while moving toward common
fleet contracts.

## Current Integration

Fleet Control currently uses a local telemetry bridge at
`backend/shared/src/telemetry.rs`. It intentionally mirrors the
`sdlc-telemetry` API shape from `services-base` commit
`abd23d59d09d70b79ab638167c25267eee960491`.

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

`sdlc-auth-core` is not enabled yet. Fleet Control currently issues local
browser-session JWTs with a compact claim set. The shared auth crate expects
fleet-wide `aud`, `iss`, `scopes` and optional `sid` claims and also supports
OIDC/JWKS. That migration needs a compatibility step so existing local sessions
and RBAC checks keep working.

The expected path is:

1. Extend Fleet Control access tokens with fleet-compatible claims while still
   accepting the current local token shape during a transition window.
2. Add `SDLC_AUTH_MODE=hmac|oidc` configuration.
3. Validate local HMAC tokens through `sdlc-auth-core`.
4. Add OIDC/JWKS mode backed by the shared Rauthy deployment.
5. Remove the legacy validator after all local sessions have expired.

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
