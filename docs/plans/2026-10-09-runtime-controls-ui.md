# Runtime Controls UI Consumer

Status: stable command identity implemented; receipt integration pending.

## Scope

This source branch starts at journalcb720d7, including main34aaec0 and
foundation8faea62. The backend controls13 release is a separate dependency.
No migration, runtime configuration, deployment pin or generated API schema
is manually changed by this UI slice.

Stop and steer clients require a caller-owned Idempotency-Key. Chats retains
the original run/key after a lost stop response, even if the active run changes.
Steer preserves its original input and identity; an uncertain steer never becomes
a new prompt. Legacy session controls also pass explicit keys and retain the
original command arguments on retry. A user edit is a new explicit intent.

## Verification

Pinned Node22.20.0/pnpm10.28.1, frozen offline dependency installation and Base
SDK19a7. Three focused suites pass25 tests: API headers, production chat and
legacy session controls. Typecheck passes. This is component evidence, not a
live runtime, browser screenshot or SDLC acceptance claim.

## Remaining Integration

- Merge the exact controls13 source preserving current-main ancestry.
- Generate OpenAPI from Rust and frontend types from that output; do not invent
  handwritten receipt DTOs or patch the generated files manually.
- Show reserved/submitted/uncertain/acknowledged/terminal_observed separately.
  ACK is not completion or proof of physical stop.
- Poll the original scoped command receipt, preserving uncertainty after a lost
  HTTP response; absent list entries do not authorize a fresh command key.
- Do not clear an uncertain command as success merely because HTTP returned200.
- Verify owner-only mutations, read-only receipt access, run changes and reload
  recovery in browser tests, then capture any changed visible states.

Task-bound controls remain denied until verified Workflow admission. Private
chat controls do not grant machine authority or complete a business stage.
