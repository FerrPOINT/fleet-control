# Architecture

Current SDLC scope and unimplemented gates are tracked in
[SDLC implementation](SDLC_IMPLEMENTATION.md). Automatic SDLC remains disabled;
legacy leaders are preserved, not part of the current delivery scope.

Fleet Control keeps a small control-plane core:

```text
frontend -> api -> app services -> infra repository/provisioner/runtime
                              -> PostgreSQL
                              -> agent folders
                              -> Hermes process
```

## Backend

- `domain`: public types shared by API and OpenAPI.
- `app`: service contracts, auth and orchestration context.
- `infra`: SeaORM entities, PostgreSQL repository, filesystem provisioner and
  local runtime supervisor.
- `api`: Axum routes, auth middleware, REST/SSE and OpenAPI.
- `server`: migrations, dependency wiring, seed agents and process startup.

## Shared Fleet Base

Fleet Control is aligned with shared fleet building blocks from
`FerrPOINT/services-base`. Cargo consumes `sdlc-telemetry`, `sdlc-shared` and
`sdlc-auth-core` through sibling checkout path dependencies. The frontend consumes
`@sdlc/ui`, including shell, controls, API utilities and bearer SSE reconnect.

`server` initializes logging through `shared::telemetry::init_tracing`, and
`api` wraps HTTP routes with `shared::telemetry::request_id_mw`. Every API response
carries `x-request-id`, and application logs include request method, path,
status and latency in the same format as the rest of the SDLC fleet.

Central ES256 validation uses `sdlc-auth-core` with JWKS. Local users are linked
only by verified central subject and retain their stored role. Central mode
disables local credential fallback; there is no effective-admin bypass. Standalone
legacy HMAC mode retains strict issuer/audience validation for fleet claims.

Central profile names come from Base's live verified profile metadata, not the
email prefix or a stale access-token claim. Missing verified names fail closed
before provisioning. The repository updates only the name and timestamp of an
active matching `central_sub`; identity, stored role and historical same-email
profiles remain unchanged. An unchanged profile uses a read path, avoiding a
user-row write lock on every protected request. Inactive profiles are not
reactivated as a side effect of authentication.

## Migration History Compatibility

Fresh installations use nine common migrations and the combined foundation
migration. Existing split SDLC histories retain all 13 applied versions. Startup
selects the original registry from the applied versions without rewriting the
ledger, timestamps or historical SQL. Unknown and mixed histories are rejected;
product deployments remain part of both histories. The profile integration
candidate is tested with this compatibility fix, not with an incompatible
fresh-install-only migrator.

## Security Boundary

`SystemRole = admin | operator | user` is enforced in the backend. Frontend
permission gates are convenience only.

- `admin` can manage users, roles, settings, RBAC, sessions and runtime
  infrastructure.
- `operator` can manage fleet infrastructure and all sessions, but cannot assign
  roles.
- `user` can create and use own sessions and read the safe agent directory.

`is_system_admin` remains a legacy alias derived from `system_role = admin`.

## Runtime Adapter Boundary

Runtime-specific behavior stays behind the supervisor/provisioner contracts.
Hermes and Java Agent differ in env vars, health checks, session APIs and launch
commands, but share the same agent/session/skill model.

Fleet Control separates three axes that must not be conflated:

- `AgentKind` is the runtime implementation: `hermes` or `java_agent`.
- `AgentProductRole` is the product role: `leader` or `executor`.
- `SdlcRole` is the real agent specialization: Project Manager, Analyst,
  Architect, Developer, Reviewer, Tester or DevOps.

Profiles such as `developer`, `tester` and `it_lead` define prompts, skills and
workflow bindings. They are not runtime kinds.

## Chats And Legacy Leaders

`/chats` groups sessions under the concrete agent and does not expose legacy
leader/handoff controls. User filtering is enforced by the backend. Immutable
Tracker task/agent binding and machine assignment scope remain required work;
a free private chat never advances a business stage.

Prompt and dispatch intent are committed atomically in PostgreSQL. Agent-row
locking serializes claims; running/waiting runs and unknown acceptance hold
capacity. Durable per-session events provide cursor replay, not a lossy global
broadcast. Config activation drains assignments and separates desired/effective
revision, with verified readback and rollback before releasing the drain.

The following leader behavior remains only on the compatible `/sessions` routes.

Leaders and executors are stored in the same `agents` table. A leader manages
executors through `leader_executors`; a task session can optionally select one
leader through `agent_sessions.leader_agent_id`.

Sessions are private by default. A direct executor chat has no leader. A direct
leader chat uses the leader as both primary agent and selected leader. Tasks
created from a leader chat become child sessions with `parent_session_id` and
inherit that leader.

Fleet stores messages in `session_messages` as a control-plane mirror. It does
not write into Hermes SQLite directly. Message delivery crosses the runtime
supervisor boundary and is represented by `session_agent_runs`, because one
Fleet session can involve a primary executor and a selected leader.

When a selected leader writes into an executor session, the message author is
the leader but the delivery target remains the primary executor runtime. Direct
leader chats are the natural exception because the leader is also the primary
agent.

Session and message creation are idempotent. The repository stores payload
hashes next to idempotency keys. Replay returns the original row, while changed
payloads return `409 conflict`.

## Hermes Launch

Managed Hermes agents run with isolated `HERMES_HOME` and cwd. Fleet starts the
headless control-plane surface with:

```text
hermes serve --host 127.0.0.1 --port <agent.api_port>
```

The dashboard port remains metadata for UI links, while programmatic chat
control is attached to the Hermes serve/JSON-RPC contract.

## Jobs And Settings

Provisioning and runtime updates are represented as deployment jobs so operators
can inspect, cancel and audit long-running actions. Runtime roots, runtime
sources, port ranges, integrations and auth settings are stored as typed
redacted control settings.

Agent storage reporting is computed on demand by the filesystem provisioner. It
reuses the same `agents_root/agentN` guard as purge/provisioning, scans only the
managed runtime/config/workspace/logs areas, and reports marker validity before
operators run a physical purge.

Fleet-wide storage review aggregates those read-only per-agent reports for the
technical inventory and does not persist or repair filesystem state.

## Evidence Gate

The shared-header ownership and acceptance boundary are documented in
[Fleet Platform Header](plan/2026-10-01-platform-header.md).

OpenAPI is generated from Rust source, frontend types are generated from
OpenAPI, and screenshots are generated by Playwright. The repository is not
considered green while any generated contract or screenshot manifest is stale.

## External Project Boundaries

`project-workflow` owns workflows and namespaces. Fleet Control stores bindings
only. `wiki` owns docs/evidence. `CI-CD` owns build/deployment pipelines.

## Общая база

Подключение версий, границы контрактов и проверки описаны в [BASE_INTEGRATION](BASE_INTEGRATION.md).
