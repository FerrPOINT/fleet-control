# Implementation Plan

Phase 0: pre-development hardening.

- RBAC and permissions endpoint.
- Idempotent sessions/messages.
- Session participants, leader selection, handoff and delegation.
- Deployment jobs and settings surfaces.
- Product pages for leaders and executors.
- Technical pages for agents, deployments, logs and settings.
- Screenshot manifest and evidence capture.
- Documentation and ADR alignment.

Phase 1: Hermes MVP completion.

- Finish real Hermes API session open/send/stream integration.
- Expand fake Hermes lifecycle tests into real adapter contract tests.
- Add runtime reconciler tests for desired-state restart.
- Add clean DB migration and seed workflows.
- Replace the local HMAC token validator with `sdlc-auth-core::Validator::hmac`
  after WSL/CI can fetch `services-base`.
- Add OIDC/JWKS validation mode and retire the compact-token legacy fallback
  after the transition window. — done: `auth.mode=oidc` — RS256/JWKS (кэш+refresh, kid-miss), строгие iss/aud, маппинг ролей, local login и HMAC-токены отклоняются fail-closed (см. docs/ENV.md, docs/OPERATIONS.md).

Phase 2: Java Agent runtime.

- Implement Spring Boot launch/provision adapter.
- Wire health, capabilities, sessions and chat stream.
- Add Java Agent runtime tests and screenshots.

Phase 3: fleet operations.

- Add operator retention policy thresholds and scheduled stale-folder review. — done: `fleet.retention.stale_archived_days` / `fleet.retention.review_interval_secs`, stale flag + archived days in storage report, scheduled review worker, `POST /api/v1/settings/retention/review`.
- Add richer monitoring and alerts.
- Add bulk runtime updates and rollback. — done: `POST /api/v1/deployments/jobs/bulk` (см. docs/API.md); rollback помечает runtime_update jobs через `detail.rollback`.
- Add cross-project workflow health integration.
