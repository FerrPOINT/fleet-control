# Changelog

## [Unreleased]

### Added
- OIDC/JWKS validation mode: `auth.mode=oidc` — RS256 против JWKS провайдера (кэш + refresh + kid-miss retry), строгие `iss`/`aud`, role-клейм маппинг, HMAC/local login отключены fail-closed.

## [Unreleased]

### Added
- Bulk runtime updates/rollback: `POST /api/v1/deployments/jobs/bulk` — один deployment job на агента (≤100), пропуск archived, `rollback` для runtime_update (IMPLEMENTATION_PLAN Phase 3).

## [Unreleased]

### Added
- Operator retention policy thresholds + scheduled stale-folder review (IMPLEMENTATION_PLAN Phase 3): `fleet.retention.stale_archived_days` (default 30) / `fleet.retention.review_interval_secs` (default 3600); storage report помечает `stale` + `archived_days`; фоновый review-воркер логирует stale-агентов; `POST /api/v1/settings/retention/review` (operator, audited).

## 0.2.0 - 2026-09-01

Initial Fleet Control scaffold.

- Created new Rust/React repository from the shared SDLC application stack.
- Added fleet-control domain model for Hermes and Java Agent runtime kinds.
- Added fresh PostgreSQL schema for agents, runtime state, configs, skills,
  sessions, workflow bindings, events, logs and auth users.
- Implemented Hermes folder provisioning and local process supervisor.
- Added Java Agent runtime template and phase 2 adapter contract.
- Added React pages for dashboard, agents, runtime, skills, config, sessions,
  workflows, deployments, logs and settings.
- Added documentation, contracts, ADRs and screenshot manifest.
