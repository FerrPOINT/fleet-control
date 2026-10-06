# WebKit: Chats SSE navigation diagnostic, 6 October 2026

This is an append-only diagnostic after consumer commit
`afbfb59eb144d51e0ffeada6f51906a114bb9422`. It changes no production
frontend, Base code, authentication, producer contract or installed runtime.
The existing browser `pageerror` assertion remains intact. No error is filtered,
suppressed or marked as successful product acceptance.

## Observed failure and repeat coverage

The runtime chat's combined Fleet `c02f60ebdceebaa4255352d27893976fc26c1389`
and Base `cbb4e99230420dc2659431b1c9fb5090e5c940f0` browser gate passed 35/36
cases. WebKit's PM clarification case reached its final assertion with:

```text
/localhost:65308/api/v1/sessions/00000000-0000-4000-8000-000000000201/stream due to access control checks.
```

All preceding form and screenshot assertions passed. The protected source log
is `.local/fleet-chats-browser-checks-4bd94b1e7166/browser-fixtures.log` in
the umbrella workspace; its SHA256 is recorded in the evidence below.

Six separate repeats of this case on the consumer branch, WebKit, one worker,
same-origin fixture APIs and owned `http://localhost:24173` preview passed.
Diagnostics recorded stream requests/responses, request failures and navigation;
all six had zero page errors. The stream fixture returns a finite heartbeat,
so the SDK reconnects after EOF. Reload passes through the fixture OIDC flow.
The repeated passes do not replace or invalidate the combined gate's failure.

## Minimal reproduction with the installed SDK

Run from Fleet's `frontend` directory, with the frozen dependencies, Node
22.20.0 and installed Playwright WebKit:

```powershell
node scripts/diagnose-event-stream-navigation.mjs report.json
```

The [diagnostic script](../frontend/scripts/diagnose-event-stream-navigation.mjs)
reads the installed `@sdlc/ui` authenticated stream implementation and transpiles
TypeScript without semantic edits. It starts an owned HTTP server on a dynamically
allocated localhost port, serves a same-origin finite SSE response, and closes
the browser and server in `finally`. It uses only a fixture token; no installed
application, user credential, container or external identity service is accessed.

It runs 50 iterations of each condition:

| Condition                                                                       | Browser page errors | Unhandled Promise rejections |
| ------------------------------------------------------------------------------- | ------------------- | ---------------------------- |
| Ordinary stream, followed by disposal                                           | 0                   | 0                            |
| `location.assign('/done')`, then opening the stream while navigation is pending | 5                   | 0                            |
| Open and dispose the stream before assigning navigation                         | 0                   | 0                            |

The failing condition emits the same message and its stack points to the SDK
`fetch` call. This occurs with the transport's `try/catch` still present and no
`preventDefault` or error allowlist. Cancelled requests are recorded separately;
they are not themselves page errors. The report explicitly sets
`productAcceptance: false`. Exit 0 means the diagnostic reproduced the boundary
and its control assertions passed; it is not a green browser acceptance gate.
Exit 2 means that run did not reproduce the intermittent message.

The installed SDK source's normalized LF SHA256 is
`9e964b76c7f12a91e81259b20caa05ff5685ba0380933d08e29a21893ef01807`, matching
the pinned sibling source. Playwright is 1.61.1. The reproduction establishes
that WebKit can emit this page error for a caught, same-origin fetch that starts
with navigation pending. It does not establish the exact interleaving in the
earlier combined test, which had no corresponding request/navigation trace.

## Base owner follow-up

The pinned SDK catches fetch/reader failures and exposes a disposer that clears
the reconnect timer and aborts the request. It has no automatic document-navigation
disposal. The reproduction identifies the lifecycle boundary for the Base owner
to review: pending EOF reconnects and new connections must not survive a page
leaving or start during an already pending authentication/navigation transition.
Do not add a page-error allowlist or change CORS/credentials to hide this result.
The zero-error disposal control is evidence for review, not a proof that an
automatic `pagehide` handler alone closes every possible interleaving.

Any shared fix requires its own browser regression and fresh exact-head combined
gate, including the PM case's existing unfiltered assertion. Consumer ownership
does not permit changing the pinned Base sibling or upgrading its pin here.
The original 35/36 combined result therefore remains an unresolved acceptance
finding until that gate is rerun with an assessed lifecycle change or a traced
explanation of the original failure.

Compact source, log and control evidence:
[validation.json](assets/design/chats-pm-webkit/validation.json).
The earlier [consumer context packet](assets/design/chats-pm-context/validation.json)
is preserved unchanged: 269 frontend unit tests, 36 fixture browser cases and 13
backend API/PG/HTTP tests. This diagnostic adds no claim of live PM acceptance.
