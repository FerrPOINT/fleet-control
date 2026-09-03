# API Standards

Fleet Control API standards:

- Base path is `/api/v1`.
- Rust source annotations are the source for OpenAPI.
- Frontend generated types must come from `openapi/openapi.json`.
- All mutating endpoints require authentication and write `audit_log` when they
  change control-plane state.
- Idempotent create/send endpoints accept `idempotency_key`.
- Reusing a key with the same payload returns the previous resource.
- Reusing a key with a different payload returns `409 conflict`.
- Secret-like fields must be redacted in responses and audit payloads.
- Public session APIs use `primary_agent_id`; legacy `agent_id` remains only as
  a compatibility field.
- Runtime writes go through `RuntimeAdapter`; API handlers must not write into
  Hermes internal databases.
- New access tokens must include configured `aud` and `iss` claims. Legacy
  compact-token compatibility may only be used for tokens that omit both fields.

Pagination and filtering:

- List endpoints accept bounded `limit`.
- Default session filtering is current user only.
- `user_id=all` and multi-user filters require `sessions:read_all`.
- Date filters use RFC3339 timestamps.
