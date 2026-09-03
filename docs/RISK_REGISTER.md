# Risk Register

| Risk                                                | Impact                      | Mitigation                                                               |
| --------------------------------------------------- | --------------------------- | ------------------------------------------------------------------------ |
| Hermes CLI command differs by environment           | start fails                 | command is configurable through `FLEET_CONTROL_FLEET__HERMES_COMMAND`    |
| Backend restart loses child process handles         | stale status                | health action reconciles untracked Hermes as stopped                     |
| Java Agent model drifts before implementation       | rework                      | keep adapter contract documented and visible in UI                       |
| Agent folders are manually edited                   | inconsistent state          | `.fleet-agent.json` marker and idempotent provisioning                   |
| Secrets leak in logs                                | credential exposure         | redact before persistence and API response                               |
| User filter is enforced only in UI                  | private data exposure       | backend defaults omitted `user_id` to the current user                   |
| Operator all-session access missing                 | support gap                 | `SystemRole` grants `sessions:read_all` to admin/operator                |
| Leader sees private executor task                   | privacy break               | private sessions have `leader_agent_id = NULL` and are not leader-scoped |
| Leader writes outside its team                      | incorrect control authority | validate `leader_executors` before assignment and message authoring      |
| Browser retry sends prompt twice                    | duplicate runtime action    | idempotent message create skips runtime dispatch on replay               |
| Cross-service token is accepted                     | auth boundary break         | strict `aud`/`iss` validation for fleet-claim access tokens              |
| One Fleet session maps to multiple runtime sessions | lost runtime state          | store per-agent links in `session_agent_runs`                            |
| Concurrent agent creation reuses an ordinal         | folder/port collision       | allocate `agentN` with PostgreSQL sequence, not `count + 1`              |
| Audit log stores secrets                            | credential exposure         | redact audit payloads and avoid copying full message bodies into audit   |
| Deployment job is stuck                             | stale runtime state         | job list/detail/cancel UI and audit trail                                |
| Rust checks blocked on Windows                      | false gate status           | require MSVC `link.exe` or Linux/CI before declaring green               |
| OpenAPI generation drifts                           | frontend/backend mismatch   | regenerate from Rust source and diff before release                      |
