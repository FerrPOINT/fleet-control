# Chats PM consumer integration: 8 October 2026

This package extends the production consumer already published in Fleet
`0f8d07836e70dd5258f85e604b83ccbbca04d270`; consumer
`f58db4eb0a4e73dd1eeb8275953c9cb0c596180b` is an ancestor of that baseline.
It reuses PR59's tab-scoped digest marker approach. PR59 remains an independent
main package and is not modified by this integration change.

The consumer branch is `feat/chats-pm-consumer-20261008`. Its PR targets the
published `feat/hermes-runtime-integration-20261004` dependency. This is a
consumer-only stacked diff, not an integration-to-main release. First integrate
and validate the gateway/runtime foundation; then apply this consumer package
and repeat combined gates before installation. No main merge readiness or
installed PM acceptance is asserted.

The dependency advanced to `d1b9d96215145f33100578e992dac4f954549778`
during verification. Its delta changes backend and shared runtime documentation;
the frontend, OpenAPI and Base pin remain identical to the tested baseline.

## Implemented production behavior

The inherited directory uses server agent counts, task search, concrete session
cursors, current-owner defaults and authorized multi-user chips. Agent, search,
users and cursor survive the detail return link. Legacy `/sessions` remains
unchanged. The detail has URL tabs/question IDs, complete task context,
server-order paginated transcript, retained tab drafts and reading position,
mobile/tablet context focus restoration and authoritative control projections.

Clarification supports single, multiple, text and explicit custom options using
stable option IDs. Recommendations are visible but never selected automatically.
Version conflicts retain the original draft and option labels until explicit
review. Other owners have read-only access. An answer receipt confirms saving;
the consumer separately states that this API does not confirm PM delivery or
consumption. It never equates saving an answer with publishing requirements.

Requirements show every returned document section, author, timestamp, exact
content hash and revision selection. The comparison highlights changed fields
and also preserves complete previous/current documents. It is a read-only
comparison of actual returned revisions, not an invented server diff endpoint.
Owner-only consent targets the exact current revision/hash. A disabled consent
shows the actual owner/context/document/Tracker waiting reason or identifies the
missing prerequisite projection. No fake Backlog transition or PM success is
created by the consumer.

Unknown prompt, answer, confirmation and private-chat creation keep the original
key/payload in memory. Tab-scoped sessionStorage stores only an allowlisted
actor/agent/service/key/digest marker. Reload retains a hold while losing private
text; new commands stay disabled even if fresh server reads permit them. Replays
within the original mounted form use exactly the original request after fresh
same-owner/service authorization. A late ACK followed by a rejected replay does
not erase the durable hold. Stop/steer continue using the existing exact-key
runtime receipt lookup and capability/projection guards. No leader controls are
added. Tracker outage preserves transcript and previously received documents;
dependent structured commands remain disabled.

## Contract findings and release blockers

Fresh published authorities: Tracker PR114
`357caa7a60a717eb7b0ac72f286b793326992931`, Workflow PR90
`11398711aa04605bc1a618622a84ae648e6de0c8`. Seven Fleet DTOs match the published
Tracker wire schemas, including closed nested objects and numeric constraints.
Namespace reads, schema parity and healthy runtime are not PM admission. The
published Fleet reservation declares dispatch false; bound chat controls deny
new prompt/steer. Only producer projections authorize consumer controls.

- CF-01: task context contains `can_confirm` and `waiting_reason`, but no
  structured prerequisite completion/routing-policy projection. Missing details
  are shown as unavailable; confirmation never guesses completed prerequisites.
- CF-02: answer ACK has no PM delivery/consumption receipt. Saving and unverified
  delivery are reported separately.
- CF-03: answer/confirmation have no original-key lookup endpoint. After reload,
  private payload cannot be replayed; metadata retains the hold. This package
  does not add a manual discard or infer settlement from a newer document.
- CF-04: requirements expose revision lists, without server detail/diff commands.
  The consumer compares the returned complete documents locally.

Backend, runtime/admission, credentials, migrations, OpenAPI, SDK pin, package
metadata, lockfiles, sibling repositories and shared verification ledgers are
unchanged relative to the published baseline. The exact Base dependency remains
`cbb4e99230420dc2659431b1c9fb5090e5c940f0`.

## Verification boundary

[The task manifest](assets/screens/chats-pm-consumer-20261008/validation.json)
binds canonical Git source bytes, gate logs, contract authorities and reviewed
PNG evidence to the tested package. Tests exercise production components with
signed test OIDC and a disposable real HTTP/SSE fixture. These are fixture gates,
not an installed owner PM/Workflow roundtrip. External live tests are skipped
when their environment is absent. No user browser login or password is needed
for these independent source checks. No Docker containers or runtime images are
changed by this consumer task.

The unit gate passes 455 tests in 35 files. Build, lint, formatting, UI contract,
OpenAPI compatibility, strict published Tracker parity, pinned Base, packed UI
consumer and effective theme gates pass. All 135 standard fixture screens were
captured and verified; `stock-manifest.json` preserves that capture's hashes,
while the task directory retains selected Chats/PM screens. Shared baseline
screens and their manifest are unchanged by this package.

Earlier full browser runs had isolated failures in a legacy delegation toast,
a WebKit controls fetch during reload, and Chromium `ERR_NETWORK_CHANGED`
during navigation. No assertion was removed or weakened. The final full run
uses three workers and zero retries; its exact result is recorded in the manifest.
The WebKit access-control failure repeated in the serial run. Fixture JSON
responses now return the concrete origin and allow credentials, matching the
shared client's credentialed transport. The affected test passes three
consecutive diagnostic runs with its original page-error assertion intact.
CI currently selects pull requests targeting main, so the stacked base does
not trigger those jobs. Local fixture gates do not replace combined runtime
acceptance after integrating the dependency.
