# Signed Configuration Recovery

## Status

Implemented in the integration candidate; default-off, not enabled in accepted
deployments. Stopped-agent and transaction regressions are distinct from actual
running Hermes crash acceptance, which remains open. See
[verification](../CHAT_CLARIFICATION_VERIFICATION.md) and
[gap register](../GAP_REGISTER.md#interrupted-configuration-activation).

## Context

A released OS lock establishes exclusion but not provenance or safe continuation.
A retained activation may have partially applied files, started a candidate or
committed its result before journal acknowledgement. Unsigned v2 identifies a
revision but cannot authenticate its snapshot, prior effective state or launch.
Replaying unknown native commands or treating HTTP health as readiness could
overwrite files used by an executing agent or create another generation.

## Decision

- Write a closed v3 payload authenticated by domain-separated HMAC-SHA256 using
  the existing runtime-token secret. Seal candidate/effective snapshot hashes,
  canonical roots and original controller/launch together with backup bytes and
  expected file hashes/absence. HMAC is authentication, not encryption.
- Reopen only in protected controller storage under the original per-agent OS
  lock. Verify private regular-file metadata, bounded bytes, exact HMAC and
  current database/rendered plan before effects. Keep unsigned v1/v2, rotated
  secrets, changed snapshots and partial documents held; never auto-upgrade.
- Keep recovery opt-in and independent of controller-custody recovery. Reuse the
  existing activation worker rather than adding a business scheduler or public
  force-release endpoint. Stopped agents remain stopped.
- A committed candidate is acknowledged only after exact file/runtime readback.
  An interrupted candidate restores prior bytes only after confirmed quiescence.
  A previously running container requires original namespace-exit proof and a
  fresh rollback generation with readiness. Pending preparation or uncertain
  native command acceptance remains held without resubmission.
- Recheck claim, current desired/effective hashes and absence of active runs or
  unresolved delivery inside the rollback transaction. Preserve the effective
  revision, mark the candidate failed and append one digest-only audit. Preserve
  drain/journal on failure; replay cannot create another settlement/audit.

## Consequences

No new migration, public API, Java capability or model/SDLC permission is added.
Recovery needs the original protected root and secret in backups. Secret rotation
while journals remain requires reviewed reconciliation, not re-signing. Windows
activation and native-orphan recovery remain unsupported. A foreign recovered
runtime reporting degraded observational health cannot retire a running journal.
Real running-Hermes crash/readiness and backup-loss acceptance still block rollout.

## Alternatives

Adopting unsigned journals, using RAM backups, clearing drain on health, or
reissuing an unknown command would discard provenance or execution exclusivity.
Automatically promoting a candidate after crash would invent successful readiness.
All are rejected. A new public repair endpoint or distributed scheduler is not
needed for this bounded private reconciliation path.
