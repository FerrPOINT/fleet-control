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
7. Start agents explicitly if reconciliation cannot prove their previous state.

Never restore agent folders without matching database rows unless an explicit
adoption flow validates marker ownership.
