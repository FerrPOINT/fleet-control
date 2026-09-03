# Gap Register

Open gaps:

| Gap | Severity | Owner | Exit criteria |
| --- | --- | --- | --- |
| Native Windows Rust linker missing: `link.exe` | Local tooling limitation | Environment | Install MSVC Build Tools for native Windows cargo commands |
| Local Docker engine/compose hangs during final migration smoke rerun | Local tooling limitation | Environment | `docker compose ps` returns promptly and clean DB migration `up/status` reruns |
| Java Agent runtime operations are phase 2 | Accepted MVP scope | Runtime | Adapter provision/start/chat implemented and tests pass |
| Physical agent folder purge is not implemented | Accepted safety scope | Product | Separate explicit admin purge flow with confirmation and audit |

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
