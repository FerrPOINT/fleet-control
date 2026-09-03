# ADR 0009: Fleet-Compatible Local JWT Claims

## Status

Accepted.

## Context

Fleet Control currently runs in local HMAC auth mode. The shared
`sdlc-auth-core` crate expects fleet-wide JWT claims: `aud`, `iss`, role/scopes
and an optional session id. A direct crate dependency is blocked until WSL and
CI can authenticate to the private `services-base` repository.

The control plane still has active browser sessions issued by the earlier
compact local token shape that contained only user id, email and expiry.

## Decision

Fleet Control will continue validating tokens locally in HMAC mode, but newly
issued access tokens include `aud`, `iss`, `role`, `scopes` and `sid`.

Validation first uses strict HMAC checks for the configured issuer and
audience. A legacy fallback is allowed only when the untrusted token payload has
neither `aud` nor `iss`; tokens that contain fleet claims never fall back to the
legacy path after issuer or audience failure.

`FLEET_CONTROL_AUTH__MODE` is introduced and currently accepts only `hmac`.
OIDC/JWKS remains reserved for the later `sdlc-auth-core` migration.

## Consequences

- New browser-session JWTs are compatible with the future shared HMAC
  validator shape.
- Existing compact local browser sessions keep working during the migration
  window.
- Wrong issuer or audience is rejected for all tokens that claim to be fleet
  tokens.
- Backend authorization remains based on the database user role, so role/scope
  token claims are hints rather than the source of truth.
- The remaining auth gap is narrower: swap local validation for
  `sdlc-auth-core`, then enable OIDC/JWKS and retire the legacy fallback.

## Alternatives

- Switch directly to `sdlc-auth-core`. Rejected for now because the private
  dependency cannot be fetched by WSL/CI.
- Keep compact local tokens until the shared crate is available. Rejected
  because it delays compatibility work and keeps a larger migration step open.
