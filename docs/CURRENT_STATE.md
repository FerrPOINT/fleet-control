# Current State

## Current Integration Snapshot: 10 October 2026

Status: implementation and qualification in progress; **not merge-ready or live
accepted**. This snapshot is not a job monitor. Linked CI runs are authoritative.

## Source And Scope

Integration input `e00b73070c327d28883c3f7a2a523b759df6662b` normally
merges the Chats foundation and runtime assembly, original-command recovery,
canonical clarification answers and accepted-PM-run following while new dispatch
is disabled. Migration024 repairs the ACK constraint additively; original022 is
unchanged. Canonical/split lineages are25/28. These fixes are source-reviewed;
current-source Rust/PostgreSQL and native qualification remain required.
Frontend candidate `dc7d1ee2574c1a05b55d794f4252b6ba37916a82` retains the final
live-region update barrier and corrects the browser fixture to read the existing
`returnTo` parameter. Its filter/cursor/owner/navigation assertions stay intact.
The integration also includes exact-run PM instruction receipts in the existing
tool journal. New question/revision claims require a validated matching receipt;
continuation cannot reuse the old run's proof. This is not proof that the model
read the instructions or completed the workflow. The actualaa11 backend gate
passes workspace/all-target check, strict Clippy and the preceding credential
gates before failing foundation. API generation alone was not runtime compilation.
The current candidate repairs three test fixtures: owner identity, a valid config
revision before drain, and a PM draft chat without an extra ordinary pending run.
The original022 downgrade refusal is tested directly with repaired024 installed;
it does not claim a successful historical whole-lineage downgrade. Existing
denial, custody, ledger and no-redispatch assertions remain. Independent source
review and Rust1.88 formatting pass; the corrected source still needs full PG/HTTP
and native qualification.

The alternative unused chat controller/storage helper/CSS were removed.
Legacy sessions and leaders are retained outside current Chats development.
The remaining legacy session controller and new chat controller are separate;
their consolidation must preserve history and compatibility.

Hermes stays unchanged. Fleet checks ordinary owner/project/current assignment
and workflow state before dispatch. No custom pre-model hook, reserved-run
handshake, second scheduler or host-controller service is required.
Tracker and Workflow are read-only dependencies; full non-PM execution authority
remains a producer dependency. Java lifecycle exists, but automatic Java SDLC
requires separate compatible chat/control evidence.

## Current Evidence

| Scope                    | Verified evidence                                                                                                                                                                                                                                                                                                                                                | Still open                                                                                                                               |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Rust OpenAPI, sourceaa11 | [38066257094](https://github.com/FerrPOINT/fleet-control/actions/runs/38066257094) SUCCESS; artifact11674599663; original strict readback verified; schema SHA256 `afa46ac37b726232eda73df46c24d1d42c796f8873eefb68454fbe0f243df501`. Schema bytes equal the actuala5f schema compared against seven immutable Tracker357 DTOs; eight verifier units pass        | Complete backend/DB/native checks remain separate from schema generation                                                                 |
| Backend, sourceaa11      | [38067316691](https://github.com/FerrPOINT/fleet-control/actions/runs/38067316691) FAIL at foundation, exit101; original strict readback verifies artifact11676356004, eight failed test names/four source frames and cleanup. Earlier workspace/check/strict Clippy/credentials gates complete; the MCP400 correction reaches the intended boundary             | Qualify the combined fixture fixes, instruction receipt regressions,024 upgrade, disabled-dispatch following and all remaining gates     |
| Frontend, source6dd      | [38067840624](https://github.com/FerrPOINT/fleet-control/actions/runs/38067840624) completes all23 gates, then FAIL at final artifact validation; original strict readback verifies artifact11675842096 and cleanup. Fixed public hint is finish_fixture_path. Source proof identifies Playwright attachment copies; the exact rejected filename is not retained | Qualify reviewedcc90 exact-path/byte duplicate handling without widening the artifact allowlist; then visually review qualified captures |
| PM execution             | Structured tools, checkpoint continuation, controls, stream recovery and exact-run instruction receipts are integrated; actualaa11 compiles runtime/test targets before foundation failure                                                                                                                                                                       | Complete PG/HTTP receipt and recovery regressions; real compatible service calls and owner flow remain required                          |
| Forge                    | [38065153549](https://github.com/FerrPOINT/CI-CD/actions/runs/38065153549) FAIL at native OCI, exit101. Authenticated A/B/C receipts verify first five stages, PostgreSQL3/3,24 negatives and cleanup. The closed80ff projection retains source25be oci_delivery.rs:540, the first CLI success assertion; the underlying CLI result is not retained              | Diagnose the existing closed CLI failure observation; qualify the original full12 pipeline and actual task delivery/rollback             |
| Base                     | [PR183](https://github.com/FerrPOINT/services-base/pull/183) merged externally as66b7faf; three maintenance helper blobs match the previously qualified payload and the Forge successor pins that merged object                                                                                                                                                  | Consumer/native checks remain separate; no silent SDK or installed-packet promotion                                                      |

Backend controls retain all existing checks. The new024 ignored migration test
must be explicitly selected and its own disposable database cleaned.
Source/pure checks do not execute PostgreSQL, HTTP or native runtimes.

Frontend diagnostics preserve the original unit command, pool and assertions.
The latest authenticated failure receipt records all23 completed gates, but
its browser counters are unavailable: zero projection values are not passing
test counts. Qualified image upload was skipped after final validation failed;
existing screenshots remain historical. Frontend/OpenAPI/Base bytes are equal
between source6dd and sourceaa11; this scoped parity does not qualify the newer
backend or prove live acceptance.

Idle task-bound PM prompts already have an explicit capability restriction,
including a disabled composer. They do not fabricate a waiting checkpoint or
fall back to a private-chat run. Server/live qualification remains separate.

The PM creation API and typed client exist. The form is still a separate preview
awaiting design approval, not a production Chats entrypoint.

## Release Decision

Qualify the reviewed fixes on their final relevant inputs before release.
Do not repeat unchanged failed attempts or replace actual execution with fixtures,
a healthy process, runtime ACK or a completed run. The real owner PM clarification
and exact-revision confirmation flow and the broader seven-agent delivery/
integration scenario are both still required.

See [active gaps](GAP_REGISTER.md), [delivery order](REMAINING_DELIVERY_WORK.md),
[approved plan](CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md),
[runtime scope decision](contracts/CHAT_CLARIFICATION_CONTRACT.md#runtime-scope-decision-2026-10-10)
and [full SDLC scope](SDLC_IMPLEMENTATION.md). Superseded receipts remain in
[state history](CURRENT_STATE_HISTORY_2026-10-10.md),
[gap history](GAP_REGISTER_HISTORY_2026-10-10.md) and the
[previous snapshot](https://github.com/FerrPOINT/fleet-control/blob/ce4153f453e730dad1e315131d65ca030243264c/docs/CURRENT_STATE.md).
