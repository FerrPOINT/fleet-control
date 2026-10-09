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
allows only pinned Python blob reads through one pre-opened owned Git process,
denies all new subprocesses during source execution, and denies writes outside its own
disposable scratch. This is an operational guard, not a hostile-code sandbox.
The Python host environment is measured, not claimed to be a locked Hermes venv.

## Runner validation successor

`dc6ab80091eb4d7ac8d8f5c02d233e49b2f51a65` remains immutable. Its parent rerun
`run-e35178825a60` is FAIL: 90-second child timeout after 16 oracle plus four
Hermes probes, during the real native-context case. Scratch was absent. Earlier
successful runs are separate evidence, not a relabeling of that failure.

The log cannot distinguish the lazy `model_tools` import/discovery from later
middleware execution; there was no inner stack/progress trace. A bounded
read-only diagnostic measured three per-object Git calls at 49.82/44.01/44.38 ms
versus 56.76 ms for a batch of the same three canonical blobs, with byte/hash
parity. One Git launch per imported blob was a measured overhead (457 imports
in earlier successful runs), NOT a proven complete cause of the historical
90-second stall. Concurrent host I/O is also not established as the cause.

The successor uses one owned `cat-file --batch` process, exact pinned object
allowlisting, Git object-hash checks and memory-only reuse within that process.
It closes/waits its exact process handle in finally. Parent independently
revalidates every receipt import against pinned Git bytes, sizes and SHA-256.
Exit0 without a receipt, FAIL, malformed/ambiguous JSON, wrong closed fields,
inventory/selectors/counts/pins, unsafe flags or incomplete provenance is FAIL.
No new endpoint, producer capability or runtime authority is added.

The original 90-second execution budget is NOT increased. One absolute deadline
covers child execution and all subsequent verification. Each verification Git
call, including `ls-tree` and final donor status, recomputes its remaining timeout;
readback/hash completion and the final PASS decision recheck that deadline.
Import progress and 30-second repeating diagnostic stack dumps identify the pending phase on a future
authorized run. Existing sealed evidence is never overwritten.

Runner unit tests are a separate synthetic boundary family, not additional
Hermes probes or live acceptance:

```bash
python -B -X utf8 -m unittest discover -s scripts/pm_tools_conformance/runner_tests -v
```

This family has 28 cases, including missing/FAIL/wrong-inventory/forged-exit0
negative cases, full parent run() failure paths, canonical batch/hash/parity
checks and exact-handle cleanup failure. Ten deadline cases include the review
counterexample (child89s + tree2s must FAIL at virtual t=91), exhaustion before
launch, fresh batch timeouts, late readback/hashing and denial before PASS.
The 24 conformance cases remain unchanged and separate.

Frozen `e05e77e7c41803a597b07ef669d915e7cae44d15` has a separately retained actual
parent run `run-c74568ddf359`: 24 cases, receipt validated, scratch absent,
PASS_OFFLINE_ONLY. That fast execution is not invalidated by the independent
deadline finding; nor does it validate this deadline successor. Successor
conformance execution awaits parent review. Runner-unit PASS does not accept
the historical failed or a future live run.

The absolute-deadline successor `2ee879f269b1c4bc84818f20a37fc20e8a5decf4`
has since passed independent source review and all28 runner units, including
the original virtual91s counterexample. Parent normal integration
`379b48e27c3306e5d818d971e59c680a4bf75697` separately executes the24-case packet:
`run-32a21e5cfeba`, receipt validated,457 canonical imports, owned reader exit0,
scratch absent and PASS_OFFLINE_ONLY. Its seal is
`fc1e620d8ddf58580e33814e7dd0f225d12cd3ac6ea18bb5f5731b1b8fbe6442`.
This evidence belongs to that exact integration HEAD and packet bytes, before
this documentation update; it does not qualify later source or enable admission.

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
