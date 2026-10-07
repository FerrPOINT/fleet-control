# Production Chats consumer release packet: 7 October 2026

Task context failure previously appeared as absent requirements, an unassigned
execution or an unbound workspace. The production consumer now shows loading,
refreshing and failed context reads independently. Both initial and previously
cached unbound contexts show the actual read error after access denial. A single
error is shown in the selected tab. Previously received bound documents and
answer drafts remain readable; failed or pending authority keeps actions disabled.
Fresh authorized reads restore the form without submitting its draft.

## Source history and scope

The owned branch is `fix/chats-session-recovery-20261007`. Clean checkout
`94e889d2632517936597af5835e45cb607d78c4b` fast-forward merged published integration
`54a4e1458386dcf640a93b2e7a75a05009756d4e`. Original-key recovery
`16b751679db3a32e921603acc64e9e9aa4f7c350`, tab reload `b466a67` and PM actor/service
ACK protection `94e889d` remain ancestors. Integrated login isolation in `6d2cd40`
clears private forms/caches on a new login; digest-only unresolved control metadata
survives in the original tab and requires fresh original actor/service access.

During verification, integration advanced to
`03c26d20c51b1c5562295c0a6e5975a64da4bc01`. It was also fast-forward merged while
preserving the task changes. Its diff from `54a4e14` is empty for frontend,
public API/OpenAPI and Base pin. Current consumer gates test those unchanged
contracts; the additional activation-ownership backend work is runtime-owned.

This follow-up changes the production chat component, its tests, one browser
fixture and task-owned documentation/evidence. Backend, migrations, public API,
Base SDK, lockfiles, CI-CD and installed runtime are unchanged relative to `03c26d2`.
Base checkout/pin is `cbb4e99230420dc2659431b1c9fb5090e5c940f0`; the exact checkout
verification passed. Requested `base-pdlc-source-status` was not found in the
available Base sources, local skills/plugins or tool metadata. Its broader
status is not claimed from the pin check.

## Verification and installed-system boundary

The [validation manifest](assets/screens/chats-context-source-20261007/validation.json)
records final source/log/PNG hashes and nine individually reviewed Chromium
captures at 375x812, 1920x1080 and 2560x1440. The unit/browser gates include
existing Chats, PM clarification, control recovery, auth isolation and exact
approval regressions. Browser fixture tests exercise real production controllers
and the shared SSE SDK against an owned long-lived HTTP fixture. They do not
constitute live PM acceptance. Development locator/mount failures and the earlier
green run before the final cached-unbound/error-display adjustment are retained.

The [read-only release record](assets/screens/chats-context-source-20261007/read-only-release.json)
rechecks four exact installed image IDs and Fleet's public schema. They match the
earlier installed record; this candidate is not installed. The earlier SSO
browser audit is preserved as historical and is not a fresh native execution.
No credentials, permissions, tasks, images or runtime snapshots were changed.

## Current producer contracts

Fresh remote reads find Tracker PR114 Draft/main at
`357caa7a60a717eb7b0ac72f286b793326992931` and Workflow PR90 Draft/master at
`44e718358d3a8fd339f9ccaa8a26c17d52454623`, both open/clean with no reported checks.
Tracker now publishes Analysis and the optional routing-policy version; the
earlier missing-field finding no longer applies to this head. Workflow now
publishes admission/binding modules; publication does not establish live PM
first-call authority, answer consumption or continuation.

The current Tracker OpenAPI was fetched directly from that exact Git head,
without modifying the Tracker checkout. A comparison retaining nested closed
objects and numeric constraints matches four of seven schemas. Differences are:

- `TrackerTaskContext.assignment`: Tracker declares `additionalProperties: false`.
- `TrackerQuestion.options[]`: Tracker declares `additionalProperties: false`.
- `TrackerRequirementsRevision`: Tracker closes the object and bounds revision
  to 1..9007199254740991; Fleet's published response omits those restrictions.

These restrictions are distinct from missing fields or a demonstrated runtime
incompatibility. The existing snapshot checker omits these constraints; passing
that checker cannot prove full published schema equality. Producers must agree
on the compatibility policy before a combined release claims exact parity.

## Minimal safe delivery against main

Current main is `c8093aace07e54436893c5f7e35df1f968690266`. Read-only merge preview
with integration `03c26d2` reports eight conflicts: `CHANGELOG.md`,
`backend/api/src/routes/sessions.rs`, `backend/app/src/lib.rs`, `docs/API.md`,
`docs/DATA_MODEL.md`, `docs/LOCAL_SETUP.md`, `docs/SECURITY.md`, `docs/TESTING.md`.
They belong to the runtime/main integration owner; no conflicted tree is built
or installed by this packet. The minimal consumer patch is the follow-up commit
relative to `03c26d2`, with the manifest's exact task paths. It depends on the
existing production chat/gateway/control contracts and integrated auth boundary.
Applying it directly to current main is not a verified independent release.

Deliver prerequisites into main, resolve and validate that combined source, then
apply this task-owned consumer patch and repeat combined gates. No non-main PR
or whole-product merge readiness is claimed. After compatible installation,
execute the [real owner PM acceptance sequence](CHATS_PM_API_ACCEPTANCE_20261007.md).
Native first-step admission, structured publication, delivery/consumption,
checkpoint/one-successor rebind and authorized Workflow projection still require
their own evidence. OS restart/lost tab identity discovery and direct SSE
authorization callbacks remain the precise gaps in the
[acceptance boundary register](CHATS_PM_BLOCKERS_20261007.md).

## Publication result

Implementation commit `5c273b3a95df77485aff7bd277b75b12ba3374ea` and evidence commit
`f0c51f863fb8424cdfaeda167a1330f3d3d617fa` are now published.
Four Git push attempts, including HTTP/1.1, were rejected by GitHub with
`Internal Server Error`; the separate Git object API also returned HTTP500.
Those failures remain historical evidence. A subsequent ordinary Git push
fast-forwarded the consumer branch from `94e889d` to the exact local
`f0c51f863fb8424cdfaeda167a1330f3d3d617fa`; independent remote readback confirmed it.
No force push or ref replacement was performed, and history did not diverge.

Fresh GitHub readback reports no PR for this branch, zero check runs and zero
commit statuses. The API's aggregate `pending` value with an empty statuses list
is not a successful CI result. The existing 446 unit and 96 three-engine browser
fixture results remain tied to the manifest's unchanged production source hashes.
This publication follow-up updates documentation/evidence only; it does not
claim new component gates or live PM/Workflow acceptance.

Read-only merge preview of the published consumer against exact main `c8093aace`
still reports the same eight runtime/shared-document conflicts listed above.
The prerequisite for a separate main PR remains integration of the production
chat/gateway/control/auth foundation and runtime-owned conflict resolution. The
consumer source patch is published; the combined product is not merge-ready.
