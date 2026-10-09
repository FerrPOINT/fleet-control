# Runtime Controls UI Consumer

Status: stable identity and unacknowledged-response UI verified with fixtures;
generated receipt integration and live runtime acceptance pending.

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
- Generate OpenAPI from Rust and frontend types from that output; do not invent
  handwritten receipt DTOs or patch the generated files manually.
- Show reserved/submitted/uncertain/acknowledged/terminal_observed separately.
  ACK is not completion or proof of physical stop.
- Poll the original scoped command receipt, preserving uncertainty after a lost
  HTTP response; absent list entries do not authorize a fresh command key.
- Verify owner-only mutations, read-only receipt access, run changes and reload
  recovery in browser tests, then capture any changed visible states.

Task-bound controls remain denied until verified Workflow admission. Private
chat controls do not grant machine authority or complete a business stage.
