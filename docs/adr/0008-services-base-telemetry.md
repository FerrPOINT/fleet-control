# ADR 0008: Align With Services Base Telemetry First

## Status

Accepted.

## Context

`FerrPOINT/services-base` is intended to become the common foundation for SDLC
services. Fleet Control already has a working local auth/session model, runtime
supervisor and UI. The shared base currently offers telemetry helpers and a
broader auth core with fleet-wide HMAC/OIDC claims.

Switching auth and telemetry at the same time would mix a low-risk operational
standardization with a security/session compatibility migration.

## Decision

Fleet Control adopts the `sdlc-telemetry` API shape first through a local
compatibility bridge in `shared::telemetry`.

The backend server initializes tracing through the bridge, and the API router
uses its request id middleware. The implementation is aligned with
`services-base` commit `abd23d59d09d70b79ab638167c25267eee960491`.

Direct Cargo dependency on the private `services-base` repo is deferred until
WSL and CI can authenticate to it reliably.

`sdlc-auth-core` remains a planned follow-up. The Fleet Control JWT claim shape
will be migrated toward the shared HMAC/OIDC validator in a separate change.

## Consequences

- API responses consistently include `x-request-id`.
- Backend request logs align with the SDLC fleet standard.
- The first integration surface stays small and easy to verify.
- Auth remains stable for existing Fleet Control browser sessions.
- The codebase has a narrow future swap point for direct `sdlc-telemetry`
  consumption.
- A known follow-up remains: migrate auth to `sdlc-auth-core` with backward
  compatibility.

## Alternatives

- Migrate auth and telemetry together. Rejected because it increases blast
  radius and makes failures harder to isolate.
- Keep Fleet Control fully standalone. Rejected because the fleet needs one
  common operational substrate for shared services.
