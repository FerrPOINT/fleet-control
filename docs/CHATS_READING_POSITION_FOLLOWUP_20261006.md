# Chats reading position follow-up, 6 October 2026

Baseline: `e79ca2b1db591220dc1e280fdf06de88418e2784`, with Base pinned to
`cbb4e99230420dc2659431b1c9fb5090e5c940f0`.

## One reproduced consumer defect

Switching from a long dialogue to Clarification and back remounted the transcript
at scroll position zero. Draft text survived, but the reading position did not.
The initial regression failed in the unit test and in Chromium, Firefox and
WebKit: the returned transcript was at zero instead of the previous middle of
the history. The final browser cases also check tail following after tab return.

The existing workspace now remembers its transcript scroll position. A stable
callback restores it when the dialogue's actual DOM container mounts. A reader
away from the end returns to the stored position; a reader following the end
returns to the current bottom. Existing scroll events keep that position and
following mode current. This does not keep hidden tabs mounted or reset the
position on each render, poll, draft edit or history refresh.

The focused unit regression switches to clarification, edits an answer draft,
returns to the transcript, verifies its position and revisits the retained draft.
The browser fixture uses 40 long messages at 375/1920/2560 pixels, verifies the
middle position and tail following across tabs, and retains the original composer
text without sending it. Its page-error assertion remains unfiltered.
Exact current source hashes, counts, before-fix logs and screenshots are in
[validation.json](assets/screens/chats-reading-20261006/validation.json).

## Verified consumer boundaries and remaining acceptance

| Area                           | Existing behavior checked with this packet                                                                                             | Remaining boundary                                                                                                                                |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| History/cursor                 | Server append order, overlap de-duplication, concrete older-page cursor, and reconnect/message refresh while an older page is pending  | Native SSE replay/reset after expired cursors still needs its owning contract; no synthetic history is introduced                                 |
| Reading/input                  | Tab return preserves the current transcript position or tail; composer and clarification drafts survive tab changes                    | Position is page-memory state, not durable restoration after a full reload or leaving the session; this is not an anchor-based restoration system |
| Stale/conflict/unknown outcome | Existing question-version draft retention, explicit review, readback before exact original retry, and HTTP 408/5xx holds remain tested | Durable server reconciliation after reload, delivery/resume and genuine PM receipts require producer/runtime/live acceptance                      |
| Permissions                    | Existing failed-context/read-only/reassignment and stale stop-controls guards remain tested                                            | Actual owner/operator denial and native PM execution authority need live integration checks                                                       |

This is one consumer position fix. It adds no API, runtime/model behavior,
migration, Base change or producer field. The
[Workflow projection gap](CHATS_PM_PROJECTION_STOP_FOLLOWUP_20261006.md) and
[WebKit navigation finding](CHATS_PM_WEBKIT_STREAM_DIAGNOSTIC_20261006.md)
remain separate. Fresh fixture passes do not declare full release readiness or
native PM/live acceptance.
