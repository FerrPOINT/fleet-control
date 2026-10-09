# Parallel Remaining Work: 9 October 2026

Status: five implementation/preparation assignments dispatched. This document
records work ownership, not completion or permission to deploy.

## Work Ownership

| Owner    | Independent task                                          | Deliverable and boundary                                                                                                                                                                                                                    |
| -------- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ptolemy  | Forge smoke diagnostics                                   | Private helper with phase/call timings, bounded child waits, cleanup and pure regression tests; retain the 300-second aggregate deadline and all assertions. No product change without a demonstrated cause.                                |
| Pascal   | Steer transcript durability                               | New backend follow-up from controls13: persist exactly one redacted control-message with original author/session/run identity when acknowledged; uncertain/rejected delivery must not become successful history. No UI or approval14 edits. |
| Leibniz  | Approval integration and backend verification preparation | Normal-merge frozen approval14 into recovery/controls in a new checkout; preserve every test/fence and prepare an exact combined inventory/helper. Do not alter generated contracts manually or treat preparation as execution.             |
| Feynman  | Docker mapped-volume/controller recovery                  | Continue the isolated lifecycle successor from `333c06d9`; retain Base custody, original generation/origin and durable receipts. No socket access for agents or unrelated SDK/pin changes.                                                  |
| Anscombe | Hosted Rust OpenAPI generation preparation                | Minimal build-only workflow, pinned source/Base/Rust, generated artifact with provenance and hashes. No push/dispatch until reviewed; no hand-edited schema or weakened existing CI.                                                        |
| Parent   | Integration, UI, documentation and publication            | Review source changes, generate client types from authentic Rust output, implement receipt states/readback/reload recovery, verify UI, run gates and publish scoped PRs.                                                                    |

Each worker owns a separate checkout/write set. Frozen commits and previous
evidence packets remain immutable. Task Tracker and project-workflow remain
read-only. Generated API/client files belong to the parent integration path.

## Existing Results

- Approval14 source is frozen at `68b59625e1b8a57139131a7672d90b6a3f465271`:
  one new migration, 21 added cases prepared, light checks pass. Rust/PG/native
  execution remains pending.
- Recovery/controls QA preparation contains 37 stages and explicitly covers all
  106 ignored cases; 20 pure tests pass. Its original source is `ed798638` and
  it must be retargeted/reviewed for later source and generated contracts.
- Native codegen preparation passes 17 pure tests but cannot execute under
  current host memory/commit reserves. The resource guard is not waived.
- Forge's latest full packet failed at the smoke aggregate timeout: only 2 of
  12 stages accepted. Owned resources were cleaned and independently checked.
  The delay's exact cause is not proven; diagnostic preparation is next.
- Integrated UI source `f32ecb7` retains 267 passing unit tests and three-browser
  fixture evidence with nine screenshots. Native controls/recovery are not
  accepted by those fixture results.
- Base Auth [PR126](https://github.com/FerrPOINT/services-base/pull/126) is
  review-ready at `dc43d0e25d60afa073c201e74d1a2cfe9aab8939`, with all ten
  exact-head CI checks successful at review. It is not merged or installed by
  this work split and does not establish end-to-end PM admission.

## Integration Order

1. Review backend approval integration and steer mirroring as separate changes;
   preserve original recovery/control identities and normal Git history.
2. Generate OpenAPI from the final Rust source, verify provenance, then generate
   frontend types. Retarget strict schema-parity and backend test inventories.
3. Complete receipt-state/readback/reload UI and regression/browser evidence.
4. Execute reviewed backend and Forge gates using explicit immutable source
   heads; retain failures and unknown outcomes instead of relabelling success.
5. Publish scoped PRs with exact-head CI and dependencies. Keep migration units
   separately reviewable; never publish the broad historical runtime tail as
   one release or claim fixture evidence as live SDLC acceptance.

## Resource And Acceptance Rules

Source work runs in parallel. Owned heavy Docker/Rust jobs run one at a time
after review and explicit orchestration ACK. Use genuine temporary Compose
projects, Base cleanup journal v2, exact disposable resource lists and immutable
Git exports. Do not restart Docker/WSL, prune shared resources, lower guards or
change accepted runtime images to make a test pass.

The remaining producer admission/readback dependency in project-workflow is a
separate integration blocker; no worker may introduce a model-admission bypass.
Completion requires actual Linux/PG, runtime and contract evidence, followed by
checked PR publication. Source commits, prepared helpers and healthy processes
alone are insufficient.
