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
- Add production auth/session hardening.
- Continue shared `services-base` adoption after telemetry by migrating local
  auth toward `sdlc-auth-core` HMAC/OIDC validation.

Phase 2: Java Agent runtime.

- Implement Spring Boot launch/provision adapter.
- Wire health, capabilities, sessions and chat stream.
- Add Java Agent runtime tests and screenshots.

Phase 3: fleet operations.

- Extend the explicit folder purge flow with optional disk usage previews and
  retention reporting.
- Add richer monitoring and alerts.
- Add bulk runtime updates and rollback.
- Add cross-project workflow health integration.
