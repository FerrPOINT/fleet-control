# PM tools conformance handoff

This is an offline test/contract packet, not a runtime patch or a live acceptance
receipt. No Hermes plugin, SDK, source pin, database or runtime is changed.
The producer contract is in
[PM_TOOLS_HANDOFF_REQUIREMENTS.md](../../docs/contracts/PM_TOOLS_HANDOFF_REQUIREMENTS.md).

## Pins and command

- Fleet: `ede1e41e843b3992757d4e16c051db6667da7add`.
- Hermes: `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3`.
- Host runner: Python 3.11 with existing Hermes import dependencies; actual
  Python and measured host dependency versions are recorded per run. These are
  NOT a qualified/locked Hermes venv. Missing dependencies fail, never skip.
  No automatic installation, network access, model execution or activation.

From this Fleet repository, with an existing clean Hermes checkout:

```bash
python -B -X utf8 scripts/pm_tools_conformance/qa/run.py --hermes-repo /path/to/hermes-checkout
```

`--hermes-repo` is required; the runner has no donor hostpath or env fallback.
Fleet is derived from this checkout. Optional `--fleet-pin` must be a full
immutable 40-character SHA and an ancestor of this checkout's HEAD. Default
evidence remains qualified only to reviewed `ede1e41`, not newer product code;
an explicit alternate pin is recorded, not silently granted the same review.
Hermes remains fixed to `bbaf7af` and must be clean at that exact HEAD.

Each invocation retains a new ignored `qa/evidence/run-<unique>/tests.log` and
`terminal-report.json`. The latter includes exact pins, the runner's Fleet HEAD, Fleet source hashes,
canonical Hermes import blob IDs/SHA-256, all selectors/counts, source-file seal,
donor status before/after, failure details and disposable scratch absence.
The runner fails for errors, skips, missing tests or unsealed Hermes imports.
It blocks network and non-Git subprocess side effects during source execution,
allows only exact pinned Git blob reads, and denies writes outside its own
disposable scratch. This is an operational guard, not a hostile-code sandbox.
The Python host environment is measured, not claimed to be a locked Hermes venv.

## Scope and interpretation

- `probes/test_hermes.py`: eight executable probes of real Hermes Git modules.
  The profile adapter, process-ownership callbacks and agent conversation body
  are boundary doubles; the last is a test function, NEVER an actual model.
  Real `_run_agent_sync`, plugin registration, registry, tool middleware,
  observability binding and pre-LLM context collector execute.
- `qa/contract.py`, `qa/test_contract.py`: proposed gateway/admission oracle and
  sixteen tests. All peers/credentials/bindings/clock/receipts are synthetic.
  Its journal is in-memory and is not a durable implementation.
- `docs/contracts/PM_TOOLS_HANDOFF_REQUIREMENTS.md` in the Fleet repository:
  pinned-source evidence, bounded vertical contract, exact
  missing producer/custody/readback primitives and future native acceptance.
- `qa/run.py`: offline isolation, Git-blob import/export, retained evidence and
  fail-closed test inventory. No full source/context export or cache reuse.

The expected result is 24 offline tests, zero skips, `PASS_OFFLINE_ONLY` and
independently `producer_admission=BLOCKED`, `live_evidence=false`.
Authoritative observed results and seal are in each retained terminal report,
not inferred from this expected inventory. Evidence/scratch, full Hermes source,
venv and caches are not tracked or shipped in this packet.

The probes distinguish native run ID from conversation/task kwargs, demonstrate
legacy env fallback without granting authority, test concurrent/cleared/stale
contexts, and demonstrate that the real pre-LLM hook collector is not a veto.
Negative oracle tests hold unknown writes across changed call/run IDs, not just
same-key retries. No unknown/rejected write is represented as delivered.

## Explicit blockers

The producer has no qualified fail-closed first-model admission barrier or
supported authenticated PM run-scoped identity handoff. Fleet's prepared child
credential is not yet retained/delivered by a gateway custody implementation.
Exact Tracker write readback and crash-durable operation recovery are also not
qualified here. No endpoint/capability has been fabricated to cover these gaps.

No Docker, Cargo, PostgreSQL, model, server listener, native runtime, external
mutation, push or dispatch was run. Task-bound admission remains unchanged and
held. Parent integration may use this contract/test packet without enabling an
unsafe runtime path; native producer/custody acceptance is a separate gate.
