# Runtime Controls UI Consumer

Status: stable identity, receipt readback, explicit original-key reload recovery,
lookup backend and authentic updated OpenAPI integrated. Combined Linux/PG and
live runtime acceptance remain pending.

## Integrated Lookup Producer And Generated Contract

Normal merge `82b7c8e` preserves UI `ae027dd` and producer `cfa30f39`.
The producer's required-header literal GET returns one actor/session/run-scoped
receipt or 404 without mutation/native I/O. Hosted
[run37953053154](https://github.com/FerrPOINT/fleet-control/actions/runs/37953053154)
at `93a2d9d24229e1bbf9a1dceabeecf9eab5f94419` succeeds for sourcecfa30f39
and unchanged Base19a. Authenticated artifact11626597579 ZIP SHA256 is
`da1449df287ae850b29fccb76f39aa742a3bc06a2503863aad1ba83bf52efca9`;
schema SHA256 is
`b074c77295f7ad89912e3667124545ab66f6727184257d1f72030b4417c87f82`.
The parent independently verifies the original run/attempt/artifact plus full
source inventory, tree/lock hashes and unchanged receipt DTO. The only schema
delta is the new single-receipt GET. Actual output is retained verbatim; real
openapi-typescript generation, Base schema equality and fresh-main compatibility
pass. Production source compilation is generator evidence; three new API tests,
five PG/HTTP/concurrency cases and combined backend acceptance remain pending.

## Original-Key Recovery Successor

The browser journal stores only operation/run/key metadata under actor/session
identity. No message input, token or receipt body is persisted. Reload performs
GET `/api/v1/sessions/{session_id}/runs/{run_id}/controls/lookup` with the original
Idempotency-Key header; it never automatically sends POST. This distinct route
is required: an old collection endpoint could ignore an optional header and
return another command. The producer/path are now integrated as described above;
their native authorization/PG evidence remains separate from this UI slice.

Recovery validates actor/session/run/operation and permits explicit local
settlement only after acknowledged, rejected or terminal-observed readback.
Missing/mismatched/legacy replies, 404 and failed fresh reads keep the handle.
Settlement does not resend guidance or prove successful business execution.
The same view can show old stop and new steer targets independently; submitted
means processing started, not proven delivery. Browser storage failure blocks
control dispatch before POST instead of silently losing the original key.

Independent review reproduced two defects: a late original POST callback after
lookup settlement could poison a successor handle, and pre-reservation input
rejection could leave an unresolvable steer handle. Current-key callback fencing
and pre-journal UTF-8 byte validation fix them. Eight regressions cover both
stop/steer accepted/unknown late replies and ASCII/Unicode exact/over-limit input.
All328 frontend tests across36 files and typecheck pass. Original history and
generated receipt DTOs are retained; no migration or hand-written DTO is added.
The independent closure proof passes both reproduced scenarios; the parent
final Chromium/Firefox/WebKit fixture also passes all three cases, including
over-limit UTF-8 input, without relaxing timeouts. Its result directory is
`frontend/test-results/runtime-controls-css-final`. Final lint, production build,
111 Markdown link checks and the regenerated nine-image manifest pass; the
existing large-main-chunk warning remains. Native/backend acceptance is not
inferred from these frontend results.

The failed first browser packet is retained separately from the successful
three-engine rerun: Chromium exceeded the unchanged 120-second beforeAll build
limit, while Firefox/WebKit passed. The rerun passes all three. The publisher
accepts `--input=<result-directory>` and still refuses failed Playwright results,
allowing evidence publication without overwriting failed packets. Nine fixture
screenshots were regenerated, verified and inspected on mobile/desktop. They
are not native authorization, safe-stop, runtime dispatch or SDLC evidence.

## Frozen Authentic Contract And Readback Panel

The isolated hosted [codegen run37945042300](https://github.com/FerrPOINT/fleet-control/actions/runs/37945042300)
completed successfully at workflow `db2bb4bd2b757afd652a01a4d537c19db20d6abb`,
source `fc6ef12df757a0858f0931cfd88a66fc7f133ed4` and Base
`19a7a381ae6dbea61a643bb96189e483fa64df5c`. The parent independently verified
the authenticated run/artifact and downloaded it through the reviewed verifier.
Artifact11622004476 ZIP SHA256 is
`beca6b90dc9855462f18f889987c7b5357990bc7ad210b50c20de480675aeeb9`;
OpenAPI SHA256 is
`57f1e0985ae35c7c2dad8bc903d75e44bca122409e612bb61c67005e8c1478b4`.
The API/domain Git diff between that source and integration `7f9ae892` is empty.
The actual schema output is retained verbatim; TypeScript is generated with
openapi-typescript, not edited by hand. This generator run compiled its pinned
source but does not certify the newer combined backend or its database gates.

Chats now reads the existing scoped runtime-control collection using GET only,
refreshes on events/commands and polls every five seconds. It displays reserved,
submitted, acknowledged, uncertain, rejected and terminal-observed independently.
An acknowledged stop is explicitly not proof of run completion or physical stop.
Wrong session/run receipts and failed reads are not shown as successful actions.
The current panel shows the latest 100 commands for its concrete selected run;
it is not a paginated global command history. No receipt list entry implicitly
clears a frozen composer or authorizes a new command key after an unknown POST.

All 286 frontend tests across 34 files pass, including the existing identity
regressions and 16 new panel/transport cases. Typecheck, targeted ESLint and
production build pass; the existing large-main-chunk warning is retained.
Generated-schema equality and the seven accepted clarification DTO checks pass.
Compatibility passes against actual main `b750e7b` with only the documented
run-wide approval retirement and required stop/steer identity migrations;
eight compatibility regression tests enforce those narrow boundaries. Missing
`origin/main` initially prevented the check; fetching the actual remote main
resolved that setup issue. The newly generated required headers then correctly
failed the old gate before the explicit, tested security migration was added.
The updated Chromium/Firefox/WebKit fixture passes and captures all three
required viewports, explicitly checking that the readback text is in view.
Nine regenerated screenshots are hashed by the manifest; mobile and desktop
were inspected. None of these fixture results proves native control delivery,
owner authorization, safe process stop or PM/SDLC acceptance.

## Scope

This source branch starts at journalcb720d7, including main34aaec0 and
foundation8faea62. Controls13fc6ef12 is normal-merged atdfb528f; its Linux/PG
acceptance is a separate prerequisite.
No migration, runtime configuration, deployment pin or generated API schema
is manually changed by this UI slice.

Stop and steer clients require a caller-owned Idempotency-Key. Chats retains
the original run/key after a lost stop response, even if the active run changes.
Steer preserves its original input and identity; an uncertain steer never becomes
a new prompt. Legacy session controls also pass explicit keys and retain the
original command arguments on retry. While acceptance is unknown, the original
input remains locked rather than silently becoming a new intent. HTTP200 with
accepted=false neither clears the composer nor reports successful delivery.
Legacy stop uncertainty survives closing/reopening the confirmation dialog.
The legacy composer also freezes the original session identity.

## Verification

Pinned Node22.20.0/pnpm10.28.1, frozen offline dependency installation and Base
SDK19a7. Three focused suites pass30 tests: API headers, production chat and
legacy session controls. The complete frontend unit suite passes267 tests across
33 files. Typecheck, production build and targeted lint pass. The build retains
the existing large-main-chunk warning; no warning threshold is relaxed. The production chat
component with mocked HTTP passes Chromium/Firefox/WebKit; each browser covers
375x812,1920x1080,2560x1440 and checks horizontal overflow. Browser readiness
awaits the actual session/control responses, not an arbitrary sleep.

Nine fixture screenshots and generated/verified hashes are stored in
[the fixture manifest](../assets/design/runtime-controls/manifest.json).
Reproduce with the command in that manifest, then run
`node scripts/runtime-controls-evidence.mjs` and
`node scripts/runtime-controls-evidence.mjs --verify` from frontend.
Mobile and desktop images were visually inspected. This is UI fixture evidence,
not live runtime authorization, backend dispatch or SDLC acceptance.

## Remaining Integration

- Source integration is complete at `f32ecb7`: normal merge parents are the UI
  candidate `f96fcdc` and recovery/controls candidate `ed798638`. The frontend
  and Base pin are unchanged by that merge; integrated Rust formatting passes.
  This does not replace Linux compilation, PostgreSQL or runtime acceptance.
- A subsequent normal integration retains review-ready foundation `28c9a5e`
  and accepted main `b750e7b` readiness refresh protection. Only CURRENT_STATE
  needed manual conflict resolution. Foundation's CI evidence does not certify
  the combined runtime source; its control receipt work below remains open.
- Authentic generated OpenAPI and frontend receipt types are now integrated;
  retain strict Rust/codegen drift checks in the combined backend gate.
- Backend successor `37ec604a` is integrated by normal history: approval14 and
  exactly-once acknowledged steer mirror now coexist with recovery/controls.
  Linux/PG/native evidence is still pending; frontend and wire shapes are not
  manually changed by that merge.
- Exact original-key settlement/reload recovery, lookup producer and authentic
  regenerated path are integrated; execute their combined Linux/PG gates.
- Preserve uncertainty after missing/failed lookup; never infer original command
  identity from collection ordering or issue a new key after an unknown POST.
- Complete native owner-only/control recovery acceptance independently of the
  frontend fixture checks and screenshot evidence.

Task-bound controls remain denied until verified Workflow admission. Private
chat controls do not grant machine authority or complete a business stage.
