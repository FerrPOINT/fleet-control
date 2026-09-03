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
  after the transition window.

Phase 2: Java Agent runtime.

- Implement Spring Boot launch/provision adapter.
- Wire health, capabilities, sessions and chat stream.
- Add Java Agent runtime tests and screenshots.

Phase 3: fleet operations.

- Add operator retention policy thresholds and scheduled stale-folder review.
- Add richer monitoring and alerts.
- Add bulk runtime updates and rollback.
- Add cross-project workflow health integration.
