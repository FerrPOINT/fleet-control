# Quality Gate

## Latest Reconciled Source Evidence

The b965298/c8093aa normal reconciliation with accepted Base875 SDK passes the
frozen Linux/PostgreSQL workspace gate:661 passed, zero failed,33 ignored;324
source hashes match. Frontend448 and84 three-engine fixture browser cases pass.
Actual PG/HTTP/SSE regressions cover private-owner directory/detail access and
same-stream Tracker project revocation. See the
[exact packet](CHAT_CLARIFICATION_VERIFICATION.md#main-history-reconciliation-8-october-2026).
Ignored special cases, native runtime/PM/live acceptance and ordered release
checks below are not waived by these component/fixture results.

Commands:

```bash
cd backend
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --test-threads=1
```

```bash
cd frontend
pnpm generate:api
pnpm typecheck
pnpm lint
pnpm format:check
pnpm test
pnpm build
pnpm exec playwright test
pnpm screenshots:local
pnpm screenshots:verify
pnpm chat:evidence:verify
pnpm controls:evidence:verify
pnpm markdown:check
```

Additional gates:

- managed observer: explicit renderer3 install/remove and rollback plus bounded
  original-run readback. Run all native harness host tests with discovery pattern
  `test_*.py`; the race and startup-fault tests must not be omitted. The native
  `observer` scenario requires the pinned read-only Git cache, committed recovery/
  control files and exact SDK. Keep the QA executable and literal opt-in read-only,
  production environment clearing and `/tmp` noexec. Fresh original/frozen source
  equality, two actual Hermes agents and owned cleanup are required;647 component
  passes,54 host passes or a reused diagnostic binary do not satisfy this gate.
  Interrupted observer activation and combined Fleet-death recovery remain
  separate acceptance, as specified in the
  [contract](contracts/MANAGED_REQUEST_OBSERVER_V1.md)
- original preparation readback: both uncertain and cached-prepared receipts
  must reject missing DB custody and a foreign controller. Execute the regression
  on its disposable PostgreSQL database; its damaged-restore fixture intentionally
  bypasses immutable-history triggers only in a local superuser transaction.
  Never run this fixture against an installed or shared database
- `scripts/container_supervisor_live/run.py --activation-recovery` crash points
  `candidate-running` and `before-create`: preserve distinct native evidence;
  before-create also loses a genuine prepare ACK and requires same original
  operation/generation readback, not another create. Runtime component tests use
  a separate database from the native agents so ordinals remain uncontaminated.
  Guarded restore/readiness/audit/peer proofs and exact owned cleanup are mandatory;
  none of these component gates is PM/Workflow/SDLC admission
- Linux managed-file directory barriers for rename/unlink/new ancestors;
  injected post-rename failure must retain journal/drain and never advance head
- compare the accepted seven-schema chat snapshot against exact release Tracker
  Git blobs, not a divergent local checkout. PR114 head357caa7 now matches all
  seven wire DTOs; the earlier head8c80a41 failed three schemas. PR90 head44e7183
  still does not authorize the first production Workflow step. Schema parity
  is not dispatch authority; full rollout stays closed
- clean DB migration up/status
- execute opt-in migration cases with `--include-ignored --test-threads=1`;
  `FLEET_MESSAGE_ORDER_TEST_DATABASE_URL` must target its own empty disposable
  database, not the populated heartbeat/runtime database. A green workspace
  run with ignored cases does not establish migration acceptance
- additive control-ledger 000013 upgrade, empty down/re-up and nonempty refusal
  on its own database; release only after the 000012 prerequisite
- additive journal-time 000014 upgrade/down/reapply with original guard/history
  equality and deterministic clock-regression/nonempty-downgrade tests; provide
  `FLEET_HERMES_TIME_MIGRATION_TEST_DATABASE_URL` for its separate empty database
- additive control-outcome 000015 and approval-outcome 000016 migrations in
  separate disposable databases: upgrade, empty down/reapply, legacy-history
  preservation and nonempty downgrade refusal; keep one migration per release PR
- original approval HTTP/PG tests: exact native request and scope, one committed
  claim before POST, concurrent replay, lost HTTP/DB ACK, atomic audit settlement,
  late terminal history and default-off/no-legacy-fallback behavior
- isolated `control_outcome_http_keyset` and `approval_outcome_http_keyset`
  ignored targets with their dedicated empty PostgreSQL databases; invalid
  historical contexts must not starve a later original GET witness
- actual managed native `scripts/native_supervisor_live/run.py` scenarios
  `lifecycle`, `recovery`, `controls`, `control-outcomes`, `control-restart`,
  `approvals`, `approval-recovery`, `approval-outcomes`, `approval-restart`,
  `combined-controls`, `combined-recovery`; preserve
  exact source/binary hashes and verify owned Compose cleanup. A GET snapshot
  is not complete historical replay or unknown decision acceptance proof.
  Original approval component tests do not replace the separate native
  decision ACK acceptance, separate approval Fleet-process-death gate or both
  mixed-plugin gates. Both mixed cases require both committed inventories;
  read-only POST run lookup and GET command lookup must never repeat the effect
- OpenAPI regenerate and diff
- opt-in recovery source gate: original journal/scope/epoch and DB-lock expiry
  races; Base plugin Linux SQLite/auth/boundary suite; actual pinned native
  `scripts/hermes_protocol_live/run.py --scenario recovery` with an explicitly
  supplied compatible plugin source and existing image. These component gates
  do not enable installed runtime or replace managed Fleet/native acceptance.
- standalone release Docker builds with sibling Base named contexts and locked
  dependencies; fresh Compose health plus `scripts/compose_smoke.py` acceptance
  (auth/RBAC, privacy, idempotency, proxied session SSE and restart persistence)
- markdown link check through `pnpm markdown:check`
- visual review of desktop screenshots for leaders, sessions, settings,
  deployments and logs; narrow-viewport behavior is verified by UI tests.
- generated JSON/Markdown screenshot proof equality and PNG hashes/dimensions;
  capture rejects missing fixture routes, while visual review checks rendered content.

Native Windows cargo commands require MSVC `link.exe`. The backend gate may be
run through WSL/Linux when the Windows-native linker is not installed.

Current combined heartbeat evidence is in the
[verification ledger](CHAT_CLARIFICATION_VERIFICATION.md#combined-heartbeat-closure-8-october-2026).
It distinguishes the passing workspace/static checks and final migration/export
and browser gates from failed combined QA helpers. Do not mark a helper fully
passing from successful tests before a later isolation/readiness failure, or
substitute fixture screenshots for live PM acceptance.
