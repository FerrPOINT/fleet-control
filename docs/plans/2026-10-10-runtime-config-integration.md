# Runtime And Configuration Source Integration

Normal candidate merge of integration6881c040 and configuration PR64 at820a1afe.
Both histories are retained. This is a private source candidate, not a release,
contract acceptance, runtime admission or native execution result. The historical
PR64 and runtime-unit evidence below their respective plans does not accept this
combined tree.

## Semantic Boundary

- Keep package Git verification, frozen Workflow identity, exact revision and
  effective-head lookup, owner-first machine readback and false runtime authority.
- Keep original Docker commands/plans/credentials, generation/custody/lease fences,
  per-agent workers, drain/unknown holds, readiness, exact rollback and F6 lineage.
- Process activation retains Workflow preflight and rechecks the pinned package.
  Docker fresh-claim and pending/restart discovery share the same guarded entry:
  fresh owner/package readback precedes private plan/lock writes or native effects.
- Every Docker phase rechecks the immutable revision/hash. Forward continuation
  uses the original target; stopping a failed candidate and rollback use the exact
  previous working revision, including a recovered effective child, not the root's
  old revision or a newly saved desired draft. Missing/foreign proof stays held.
- Readiness brackets HTTP/physical/file proof with fresh Workflow/package checks
  and the existing closed skills-directory readback. These checks do not fabricate
  active/effective state before the original atomic publication.
- Owner HTTP failure does not finish a Docker activation through the process
  completion path, remove its plan, mint custody or grant another native permit.
  Pre-plan read-only transient retry remains bounded per attempt; recorded native
  phases stay held with their original durable recovery evidence.
  The binding-specific classifier retries only fresh Unavailable, not authoritative
  Conflict/Forbidden/Validation or unexpected errors; those enter existing HOLD
  diagnostics. Existing custody/observe retry classification is unchanged. The
  Workflow client's original non-200 HTTP classification remains Unavailable,
  including401/403; this classifier does not reinterpret that owner contract.
- SDK19a, Base package4b9 and executable utility9b remain three distinct pins.
  No migration, lockfile, producer protocol or accepted runtime image is changed.

## Generated Artifacts: Pending

`openapi/openapi.json` conflicted. The candidate mechanically retains the exact
committed688 artifact, without manual JSON edits. Rust routes/DTO registrations
from BOTH parents remain. The retained schema therefore intentionally lacks the
two PR64 paths until genuine Rust union generation. Existing client bytes are not
regenerated or declared compatible. Parent must authenticate exact-candidate
codegen, regenerate the client and run parity/compatibility before public source
publication. A byte-identical historical artifact is not a combined contract PASS.

## Regression Gates

Four new authored Rust tests cover forward-phase immutable target selection,
all rollback phases selecting current effective rather than root/desired, and
foreign agent/revision rejection, and genuine loopback owner503 readback with no
file/package/native effects or private diagnostics. They are mandatory by name
and increase `runtime::container_activation::tests::` from14 to18. Rust/PG
execution is pending. A narrow successor adds two classifier/retry regressions:
permanent binding failures return Held on their first attempt; Unavailable can
retry only before a recorded activation. The mandatory intent count is now20;
all six new Rust tests remain unexecuted locally. The loopback regression also
returns a valid owner200 with changed catalog hash after its two503 responses:
that authoritative Conflict takes one attempt and exits Held without IO.
All prior runtime/config tests and migration guards remain, including the original
accepted-runtime approval fixture. The Python lineage selector is synchronized
with688's existing exact-latest ledger subtraction; no migration SQL changes.

Required acceptance remains exact-candidate Rust1.88 locked check/Clippy/tests,
focused configuration/activation/Auth/HTTP/PG cases, both-lineage roundtrip,
genuine union OpenAPI/client, frontend checks and separate native qualification.
Live Workflow/package/producer readiness and model admission are not granted.
Parent reports run38011797295 terminal FAILURE for frozen688, not this candidate;
its diagnosis and any separate source correction remain parent-owned. It is not
cancelled, rerun or reinterpreted here. No local heavy run is permitted by this
source merge.
