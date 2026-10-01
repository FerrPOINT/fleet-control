# ADR 0012: Configuration Revisions And Drain

## Status

Accepted and implemented for tracked Hermes runtimes. Crash recovery and atomic
lifecycle command coordination remain gaps before production SDLC rollout.

## Context

Editing configuration files under an active agent can change behavior midway
through a run. Desired settings and settings proven effective are different
facts. Failed activation must not silently expose a partly applied revision.

## Decision

- Save immutable config, SOUL, env references and skill snapshots as revisions.
- Use draft, validated, activating, active and failed states.
- Keep desired and effective revision identifiers separate.
- Activation drains new dispatch and waits for active runs and unknown
  submissions. It applies files, reads them back and verifies runtime health
  when restarting a previously tracked process.
- On failure, restore the previous files and runtime. If rollback cannot be
  verified, keep the agent drained and retain the previous effective revision.
- Resolve secret references only into managed runtime files. Do not return raw
  secrets or inherit Fleet credentials into the agent environment.
- Repeated provisioning preserves existing effective `.env`; new managed files
  are created exclusively and with mode `0600` on Unix. Provisioning rejects
  active/unreconciled runtimes and foreign unmarked directories.

Runtime health is not SDLC readiness. Workflow assignment, skills, model and
project access must be verified before automatic assignments can be enabled.

## Consequences

- Saving settings does not mutate a running Hermes immediately.
- Operators can inspect desired/effective drift and failed activation.
- A failed rollback blocks new work instead of claiming readiness.
- A disk activation journal and reconciliation API are still needed for crashes.
- Skill edits require a new snapshot; automatic skill revision UX remains pending.
- Java config activation is phase 2; existing Java lifecycle is preserved.

## Alternatives

- Immediate file edits: rejected because they change active execution behavior.
- Replace files without rollback: rejected because failed readiness leaves drift.
- Force-cancel every active run on save: rejected because it discards work without
  explicit operator authorization and audit.
