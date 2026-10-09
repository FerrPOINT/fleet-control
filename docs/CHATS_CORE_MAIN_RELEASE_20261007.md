# Production Chats core for main

This Chats package is integrated with Fleet `main` at
`b750e7b69cb359fbe7c7fd13647882c7ae8472bb` and its Base pin
`19a7a381ae6dbea61a643bb96189e483fa64df5c`. It includes the message-receipt
producer fix required by the new consumer and optional POST receipt digest in
OpenAPI. Database migrations, runtime configuration and dependency lockfiles
remain unchanged relative to that main. The PM runtime integration branch is
separate from this package.

## Supported scope

`/chats` uses the public agent directory and protected session list. Agent,
loaded-subset search and permitted user filters survive through the detail's
return link. The list labels its loaded subset instead of claiming full counts.
Load errors suppress empty-state and cached list output.

`/chats/:sessionId` has an independent dialogue workspace, URL tabs, desktop
context panel and mobile context dialog. Protected session, messages and runs
are its authorities. It preserves the server message order, isolates primary
agent runs, retains the draft across tabs, preserves the reading position and
signals new visible content without counting foreign or inactive run deltas.
The legacy `/sessions/:sessionId` workspace remains available.

Sending checks current authentication, ownership, write permission, executor,
session state, runs and pending delivery before dispatch. Other owners and users
without write permission see read-only controls. Active/pending primary runs or
pending/dispatched delivery prevent a new prompt. A response confirms message
storage/delivery only; it does not confirm PM execution or native runtime ACK.

Create and prompt requests keep their original payload/key after an unknown
response. Only the same request can be explicitly replayed in that tab. Scoped,
digest-only dispatch markers survive reload; private text and tokens are not
stored. Reload, a new actor, malformed metadata, run completion or a changed
permission does not release an unknown command. Legacy stop/steer markers stay
held without calling an unsupported lookup. A reload loses the private draft
but retains the hold; this version has no UI for manually discarding that hold.

The prompt receipt binds the session, owner, author kind and original request
digest. A valid redacted body can confirm the same command; missing or mismatched
digests retain its key. Producer acknowledgement and replay return the specific
persisted row independently of the bounded history page. The backend must
provide this POST receipt contract when deploying the new Chats consumer.

## Contract limits and separate PM package

The current producer returns at most 200 sessions and the first 500 messages
(ordered by server `created_at`, then `id`), with no cursor or total count.
Search covers only the loaded sessions. The UI states these limits and does not
invent pagination. At the message cap, late replies remain outside the history
page until the producer adds pagination. POST/replay, dispatch and mirror
receipts remain available independently of that read limit.

Main has no task-context/clarification/requirements gateway, authoritative
chat-controls projection, original-key lookup or public runtime control
capability projection. The Clarification and Requirements tabs explicitly say
PM is not connected; answer/confirmation actions are disabled. No assignments,
execution/checkpoint/attempt state, requirement revision or PM success is
fabricated. Stop and steer are unavailable in this core. Fresh reads are a
conservative UI guard; this legacy API has no atomic projected dispatch gate.
The pinned Base SSE client stops on 401/403 but exposes no HTTP-status callback;
periodic protected reads provide denial detection and dispose the workspace's
stream. This is not proof of immediate revocation notification from SSE.

The richer consumer and prior evidence remain published at
[fix/chats-session-recovery-20261007](https://github.com/FerrPOINT/fleet-control/tree/f58db4eb0a4e73dd1eeb8275953c9cb0c596180b).
The integration producer at
`c5a8831127bc53a063bef8b8a967da4095c357e7` closed the prior nested-schema finding:
source contract parity is 7/7 against Tracker PR 114 at
`357caa7a60a717eb7b0ac72f286b793326992931`. That source parity is separate from
live PM acceptance and does not add those endpoints to current main.

## Initial Frontend Validation Evidence

[The manifest](assets/screens/chats-core-main-20261007/validation.json) binds
source files, validation logs and screenshots by SHA-256. Screenshots cover
375×812, 1920×1080 and 2560×1440: dialogue, unavailable clarification,
unavailable requirements, denied and read-only states. Browser tests use signed
test OIDC tokens, intercepted existing API contracts and a real disposable HTTP
SSE fixture. These are fixture acceptance results, not a live installed runtime
or a PM/Workflow roundtrip. No live mutation, deployment or Docker container was
required.

Passed: 170 unit tests in 24 files and 57 browser tests across all three
engines, including 13 core scenarios per engine. The 27 live-test cases were
explicitly skipped because their external environment was absent. All 15 new
PNGs were visually reviewed. The dialogue uses a section inside the shell's
single main landmark. Stock `pnpm screenshots:local` captured all 135 fixture
screens, and `pnpm screenshots:verify` validated the generated manifest. The six
standard Chats list/detail images were refreshed and visually reviewed; the
task manifest also binds their hashes and the generated stock manifest.
These historical results are recorded in the manifest for the original frontend
revision, not the current integrated backend/consumer candidate. Full frontend build, unit,
lint, format, UI contract, effective theme, OpenAPI generation/compatibility and
Markdown checks use frozen dependencies and this package's exact clean Base
checkout. Browser acceptance covers Chromium, Firefox and WebKit. Existing live
specs requiring external credentials remain explicitly skipped when those
environment variables are absent.

## Receipt Contract Review

The real PostgreSQL boundary regression first reproduced a committed 501st row
with a missing receipt. The new regression preserves the initial system event,
checks exactly 499/500 history rows, and covers replay/conflict/authorization,
raw runtime dispatch and assistant mirror receipts. It requires an explicit
disposable database and fails when that input is absent. See
[the regression procedure](TESTING.md#message-receipt-boundary-regression).

Client regressions first reproduced rejection of a valid redacted receipt and
acceptance of missing/wrong request digests. All three pass after the consumer
change. The cross-language protocol fixture includes explicit nulls and the
original idempotency key in its digest. The final real PostgreSQL regression
passed, including POST-only digest projection and omitted digests in history and
runtime input. Rust regenerated OpenAPI with one optional field; generated client
types, TypeScript and compatibility checks passed. The frontend gate passed 176
unit tests, build, lint, format and UI contract. All 48 Chats cases passed across
Chromium/Firefox/WebKit on the development test server. Final hosted CI and
native application/browser acceptance remain pending for this review revision.
