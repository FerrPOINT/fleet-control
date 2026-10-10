# Runtime And Configuration Source Integration

Normal candidate merge of integration6881c040 and configuration PR64 at820a1afe.
Both histories are retained. This is a source candidate, not a release,
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

## Generated Artifacts: Verified Codegen

The merge initially retained the exact688 artifact, without manual JSON edits.
Genuine Rust generation for frozen8c93f43 now passes in
[run38015043570](https://github.com/FerrPOINT/fleet-control/actions/runs/38015043570),
workflow5e57d5b. Parent authenticates artifact11656020208 and ZIP SHA256
9e46aadcaf9de57d91d8389b0b460234ef1e0e6f02a72db81eea8af2d3d05ff8.
The integrated schema SHA256 is
1167220ea9f3d65ddca4cce1112a26d53c77f8c1684ef958859f737f20210953.
All earlier operations/DTOs remain structurally identical; the generator adds
only the two configuration paths and ConfigurationObservation/SdlcWorkflowBinding.
The generated TypeScript client, typecheck, OpenAPI drift and compatibility with
mainb750e7b pass, including eight compatibility regressions. The generated client
remains ignored/reproducible through the existing postinstall convention.
This proves contract generation, not all-target test compilation, PostgreSQL,
native execution or model admission. Final combined-source Rust parity is still
mandatory in the backend gate; a successful generator alone is not acceptance.

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
