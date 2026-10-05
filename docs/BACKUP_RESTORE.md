# Backup And Restore

Back up:

- PostgreSQL database.
- `agents_root`, including `runtime`, `config`, `workspace`, `logs` and marker
  files.
- Environment files and deployment secret source, stored outside the repository.
- OpenAPI and docs/screens evidence through normal git history.

Restore order:

1. Restore database.
2. Restore `agents_root` to the configured path.
3. Restore secrets/configuration.
4. Run migration status.
5. Start Fleet Control.
6. Let desired-state reconciliation inspect runtime processes.
7. Reconcile ownership and confirm old-process quiescence before explicitly
   starting a replacement. Missing process handles or HTTP failure are not proof
   that the old runtime stopped.

Never restore agent folders without matching database rows unless an explicit
adoption flow validates marker ownership.

Preserve dispatch/control journals and their original request/key/context when
restoring. Never renew submitted permits or edit deadlines to make recovery pass.
Migration000014 refuses down when any Hermes journal exists; do not remove its
guard/function or historical rows to force rollback. Restore a compatible backup
or retain the additive schema with a compatible binary after explicit review.
Logical timestamp ordering does not certify wall-clock retention safety.
