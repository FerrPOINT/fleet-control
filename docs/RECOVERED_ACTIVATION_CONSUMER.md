# Recovered Activation Consumer

Source successor of exact `30f0993d65bd6435769f34f43fa7db30a21845a7`, in an
independent normal clone/branch. Not native acceptance or production readiness.
Frozen units452/906/15/16/17/18, Base9b and parent integration are unchanged.

## Exact Dependencies

- SDK stays `19a7a381ae6dbea61a643bb96189e483fa64df5c`.
- Executable utilities are separately sealed at Base
  `9b53de7b23593949a9e6c05bd5a4f94b930e50a0`, all FOUR canonical Git/LF modules.
  PR180's `815982b648652b2665be9442bf395fc56c7e11d3` has identical module bytes;
  no automatic substitution or SDK update is made. Full Base CI remains external.
- Four-module captured-source loading preloads control/replacement before main,
  with no checkout/package initializer/pyc imports. Native CRLF bytes are rejected.
  Three-field historical receipt provenance is retained byte-for-byte. Only exact
  known ae8/9b provenance is accepted; executable authority is the separate current
  four-hash seal, not the old receipt array.
- New migration19 is necessary:16 forbids lease ACK/renewal after anchor exit,
  while18 refuses all recovered activation. Historical files remain exact.
  Migration19 preserves16's guard OID/all other predicates and adds immutable
  activation/recovery/plan/claim/controller authority, not a new business API.
  Child origin admission and launch mutations inherit the latest ORIGINAL anchor
  lease under SQL locks. Missing ACK/unknown heartbeat/expiry/newer epoch fences
  the child even though no recovery row exists for the child itself. A child
  also requires authority bound to the latest recovered ID/controller: a new
  epoch's native ACK alone cannot bypass original activation authorization.
  A child cannot acquire a separate recovery namespace. Empty-history downgrade restores
  the original custody function body/OID as well as the16 state guard/ledger.

Enable only with protected operator configuration
`fleet.container_control.recovered_activation = true` and a canonical utility9b
export in `base_root`. Default is false. No agent setting or HTTP DTO grants it.
No Base/Tracker/Workflow/UI/generated/lock/pin19a mutation is included.

## Authority And Execution

The original18 machine still owns desired/effective transitions. The original
claim/controller, both reserved recipes/generation/operation/stop IDs, exact
config/credentials, mappings and marker remain immutable. A current Fleet UUID
is recorded separately against the original native recovered anchor/lease.

Before each continuation Fleet reloads the complete private plan/recipes, validates
marker/layout/config/effective revision, gets original Base observation with exact
current native lease, and checks full original Activation record under agent lock.
All active run states, prepared/submitted dispatch and uncertain outbox work remain
fenced. Normal `checked()` still rejects recovery history; only the proved recovered
branch can pass, and unrelated-generation recovery history blocks it.

Native work uses Base4 `prepare/reconcile_replacement`, attachment, start/readback,
endpoint and stop/readback with original commands. A create-new fsynced0600 private
command derived from the original plan precedes preparation. Native replacement
journal scope and operation payload hash are stable across heartbeats/restarts.
Transport checks exact command/intent/anchor hashes and closed responses; neither
an arbitrary registration, endpoint, credential nor a different stop ID is sent.

Only a worker's successful original phase CAS enables its next first delivery.
Re-entry has readback-only actions; a missing native claim does not gain a new
permit. Base's durable pre-effect claims prevent repeat native effects even when
the worker loses ACK/CAS. The permanent per-agent OS lock and operation lock remain.

Renewal stays independent of long readiness polling and selects the immutable
previous anchor, not newest candidate. Recovery ACK lookup is by exact original
generation even after candidate insertion. After publication, exact target files,
published launch, current recovered lease and original anchor still guard health
readback/renewal; a subsequent proven physical restart uses that same anchor.
No process-only restart manufactures takeover; foreign/unapplied heartbeat holds.
Saving another desired draft does not revoke the effective published generation.
Historical owner lookup is by exact launch, not current desired revision. Terminal
custody still requires unchanged effective revision, published prepared/snapshot/
origin, exact managed readback and latest original native lease. Its running,
stopping and exited states remain bound to the same original authority; only an
own successful running-to-stopping CAS permits the first regular child stop.
Re-entry only reconciles the existing original stop; other actions gain no permit.

## Sequential Revisions

A next desired revision can now activate over an effective recovered committed
OR rolled-back child. Fleet seals a NEW immutable plan before effects, retaining
the original controller/anchor, family journal, exact predecessor activation ID/
hash and published launch. Old claims, commands, credentials and receipts are
not rewritten. Optional lineage is absent from old serialized plans/claims, so
their canonical hashes do not change. Private ancestry is bounded, cycle-checked
and read back against every original PG claim and sealed plan.

The new claim transaction locks agent/root/latest lease, checks exact root native
proof and predecessor authority, current effective revision, desired/draining
target, marker/paths and all active/unknown-run fences. Migration19 guards lineage
inserts AND updates; child SQL origin/mutations still inherit the latest ROOT
lease, never a child recovery row. Missing latest authority/ACK, expiry, stale
epoch or conflicting next plan is held before a new native permit. A complete
sealed next plan without PG claim can retry only this guarded pre-effect claim.

Original mapped volume/controller evidence is retained, not resolved against a
new controller snapshot. Base4 validates fresh same-engine/controller/volume
custody. Only the reserved generation/service/network and mapping input hash
change. Process/image/credential changes remain outside this capability.

Own successful Planned->StoppingPrevious CAS permits the predecessor's original
Base4 stop once. Restored stopping reconciles that exact stop, not root stop and
not a fresh permit. Before config writes, its original start ACK/stop claim and
physical exit are read back. Once a candidate becomes the Base4 journal head,
its existing guard rechecks the exact predecessor stop on every operation; Fleet
does not attempt to make a historical generation head again. Failure restores
the CURRENT effective revision/bytes on the reserved new rollback generation,
only after candidate exit and with full readiness before effective publication.
The terminal predecessor's managed-byte check is handed to that exact in-progress
successor plan; it cannot require obsolete bytes while the successor applies its
new config. Root readback, latest lease and all SQL fences remain required.

## Phase Coverage

| Recorded cut | Behavior |
| --- | --- |
| Claimed pre-plan / partial plan / lost PG claim under a foreign owner | Typed HOLD; no plan/claim reconstruction |
| Planned with complete PG/private plan and valid recovered custody | Original running/readback, drain fence, record stop permit |
| Stopping previous after restart | Read-only original stop readback; advance only with exact durable stop claim, original start ACK and fresh physical exit; otherwise HOLD |
| Previous stopped / applying candidate or rollback | Recheck original exit, then exact managed files; no fork |
| Preparing candidate/rollback | Fresh own phase CAS seals original command and permits Base4 preparation; re-entry only reconciles existing Base4 claim |
| Old protocol2 preparation or missing Base4 creation claim | HOLD; never adopt old native preparation or rekey |
| Candidate/rollback prepared | Fresh own progress attaches once; restored phase reconciles original attachment only |
| Starting candidate/rollback | Fresh own CAS starts once; re-entry observes original ACK; never-started/no ACK holds |
| Running / ready | Original physical endpoint/snapshot + exact bytes + bounded authenticated readiness; recheck before publication |
| Stopping candidate | Fresh own CAS stops original candidate; restored phase reconciles original stop/exit only |
| Candidate stopped / rollback | Exact previous bytes and reserved NEW rollback generation only after original candidate exit |
| Committed / rolled back | Effective/drain publication atomic with proof; exact published generation remains on original anchor |
| Terminal child regular stop | Own running-to-stopping CAS permits Base4 stop once; restored stopping reconciles only; no new prepare/start/attachment permit |
| Next revision on effective recovered child | Exact terminal predecessor + sealed lineage + fresh root proof; same machine, same root journal, no child custody anchor |
| Next candidate failure | Exact current effective bytes/revision on reserved rollback generation, not the root's obsolete config |

Original stop recovery calls the existing sealed Base9b `stop_readback` primitive
with its recovered mount guard, not Base3 `stop()` (which can claim a journal).
Fleet's closed private loader entry is not a new Base protocol action/public API.
The native stop journal stores an immutable command, not a separate ACK row:
original start ACK + exact stop claim + bracketed physical exit produce the same
deterministic stop receipt. Missing claim/start ACK, still-running/unknown exit,
foreign/old lease, corrupt/private/link/path evidence stays held without writes.

Unknown/missing/corrupt/private/link/path/volume/foreign-owner evidence does not
authorize fallback, cleanup, takeover, credential rotation or a second namespace.
Failures keep existing typed audited recovery actions. Utility capability executes
`ResumeOrRollbackOriginalPlanWithCompatibleBase` only where all listed proofs exist.

## Evidence And Gates

Current successor light evidence, not Rust/native acceptance:

- Linux sealed four-module CLI/private fake Engine:21 passed,0 skips,47.785s
  (final repeat; preceding21/21 repeat60.692s).
  Six new cases cover committed->next, rolled-back->next, expired lease, unknown/
  changed epoch, duplicate/conflicting next commands and failed-next rollback.
  Earlier successor attempt:20 passed/1 error,111.955s; a positive fixture lost
  its 20-second lease before start. Repeat uses the supported30-second bound,
  never a fake clock or relaxed guard. Negative expiry checks remain active.
- Authored, NOT locally executed: activation intent14 (two new successor plan/
  command cases), PG13 (four new sequential success/rollback, lineage/drain/
  concurrency/unknown heartbeat and expiry/new-epoch cases). CI requires exact
  counts with no ignored cases; migration19 still owns one roundtrip selector.
- Parent0be22c7 independently fixes pre-existing four-module Rust fixtures and
  lineage single-down. It is not copied here; normal integration must preserve
  those fixes and this file's two new tests. Rust/PG acceptance remains pending.

Historical f326 light evidence (not rerun/reattributed to this successor):

- Linux sealed FOUR-module CLI/private fake Engine gate:15 passed,0 skips,21.861s
  (final mandatory-selector repeat; preceding corrected run48.705s also passed).
  Covers positive prepare/attach/start/stop/rollback, original unknown accepted and
  absent preparation, attachment/start/stop cuts, changed payload/owner/engine/
  volume/process-only guards, and next proven physical restart. It executes exact
  canonical utility bytes through Rust's captured loader, pausing only before main
  to inject the original fake Engine. It does NOT execute Rust business phases,
  PostgreSQL or real Hermes readiness.
  Five added cases exercise original-stop ACK-before-PG-CAS readback with byte-
  unchanged evidence, missing/wrong claim, missing start ACK/no exit, foreign/
  unapplied lease, and terminal child unknown stop/exit without a second kill.
  An earlier run failed (226.093s): two inherited positive cases exceeded the live
  lease during filesystem stalls, plus a new fixture omitted running Status/PID.
  The fixture was corrected; the unchanged real lease guard was not bypassed.
- Existing activation15 Linux fake selectors:15 passed,0 skips,3.168s.
- Parent independent exact9b224/0-skip and PR180224/0-skip results remain separate
  utility evidence, not local consumer/native results.
- Captured-loader7 + canonical utility5 + README3 Windows tests:15 passed,
  12.628s. Includes the private read-only entry/redacted failure (no control main).
- Authored mandatory Rust selectors: app2 unchanged; activation intent12 (two new
  original-command/rollback regressions); replacement transport3 (also redacted
  Debug and stop-only first permit); PG activation9 (four new cases, including
  recovered success/rollback, active/unknown run fences, exact claim/lease/CAS,
  exited-anchor renewal, no readiness bypass, terminal next-draft/ordinary stop
  both outcomes and SQL serialization/unknown heartbeat/expiry/new epoch); migration19
  roundtrip1. Existing migration18 roundtrip and all other focused gates remain.

Parent owns locked compile/Clippy/full tests, these authored Rust/PG cases on both
lineages20/23, canonical Linux export, actual Compose/Engine/Fleet/controller
restart/lost-ACK races and authenticated Hermes readiness. No Docker/native,
Cargo build/test/Clippy, PG, cache preparation, push or default merge was run here.

## Remaining Limits

Not all crash cuts are automatically recoverable: missing plan/claim, old native
prepare/start/attachment without compatible original ACK and unknown original
stop remain held. No journal reconstruction or abort-before-start cleanup exists.
Base4 native durability/races still need acceptance, including lease loss during
effects and second physical restart while a candidate/rollback is unpublished.

Sequential activation uses the existing business planner, not a second scheduler.
Base4 bounds a native replacement family to256 commands (including rollback).
Fleet preflights a conservative128-plan ancestry budget BEFORE sealing/stopping,
reserving two commands per plan, even when earlier plans did not roll back.
Exhaustion holds with current effective child untouched, not journal rotation/
cleanup/new-anchor takeover. Recovered pre-plan proof failures are audited HOLD,
not fresh native retry permits; diagnostics retain original root custody identity.
No in-place upgrade of an already-applied
f326 migration19 is supplied: this source successor extends that same unreleased
migration19, applied from18 in the final clean release lineage. Parent owns native
acceptance, both SQL lineages and source integration. Process-only takeover and
missing original native proof remain unsupported, not successful recovery.
Regular post-publication stop is supported; stop-CAS-before-native-claim crash
remains held when the original stop permit is absent. Generic restart permits
are not added. Health readback is supported;
readiness does not prove loaded provider/model/tools/skills or Workflow provenance.
Image/token rotation, log ingestion and production admission remain separate.
`operator_prepared`, successful file writes or fake tests never close that target.
