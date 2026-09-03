# Gap Register

Open gaps:

| Gap | Severity | Owner | Exit criteria |
| --- | --- | --- | --- |
| Native Windows Rust linker missing: `link.exe` | Local tooling limitation | Environment | Install MSVC Build Tools for native Windows cargo commands |
| Local Docker engine/compose hangs during final migration smoke rerun | Local tooling limitation | Environment | `docker compose ps` returns promptly and clean DB migration `up/status` reruns |
| Java Agent runtime operations are phase 2 | Accepted MVP scope | Runtime | Adapter provision/start/chat implemented and tests pass |
| Direct Cargo dependency on private `services-base` is blocked in WSL/CI | Local/CI integration limitation | Platform | WSL and GitHub Actions can fetch `FerrPOINT/services-base`, then the local telemetry bridge is replaced by `sdlc-telemetry` |
| Shared `sdlc-auth-core` validator adoption is still pending | Accepted fleet standardization scope | Backend | Local HMAC validation is replaced by `sdlc-auth-core::Validator::hmac`, OIDC/JWKS mode is added, and legacy compact-token fallback is removed after the transition window |

Closed gaps:

- leaders/executors product split
- user-owned private-by-default sessions
- selected leader and delegation model
- session participants/runs
- idempotent session/message creation
- admin/operator/user RBAC model
- deployment job UI/API
- settings UI/API
- generated screenshot manifest
- WSL/Linux backend check, clippy and tests
- OpenAPI source regeneration from Rust
- `services-base` telemetry-compatible local bridge
- fleet-compatible local JWT claims with strict issuer/audience validation and
  legacy compact-token fallback
- explicit archived-agent folder purge with confirmation, marker guard, event
  and audit entry
- agent storage and retention preview before physical purge
