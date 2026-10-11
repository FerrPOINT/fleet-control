# PM Draft Creation Proposal

Status: proposed, not yet approved. This is an addition to the approved
[chat and clarification design](CHAT_CLARIFICATION_PREVIEW.md), not a replacement.
Entry: `frontend/pm-draft-preview.html#/`. Production does not import this entry.

## Proposed Interaction

The PM agent group in Chats offers **Create task with PM**, distinct from a free
private chat. The compact form asks for an accessible rollout-enabled Tracker
project, one concrete Hermes Project Manager, title and original request.
No project or agent is selected automatically. Creation is owner-only; it does
not confer management privileges, authorize a runtime launch or confirm requirements.

Unknown POST acceptance locks the submitted input and offers explicit state
readback. An absent readback permits the identical command to be retried with
the original key; edits require a separate new operation. A saved incomplete
operation offers continuation by its ID, without resubmitting prompt text.
A saved task/chat displays **awaiting PM admission**, not running or complete.
The existing chat/clarification/requirements workbench remains unchanged.

Project names come from Tracker's strict central-subject project directory,
not the legacy projects endpoint or a manually entered UUID. Directory pages
may become empty after Fleet's rollout filter while still containing a next
cursor. The controller must keep pagination available and cannot interpret
page length as a total or readiness proof.

## State And Privacy

Form, unknown acceptance, saved incomplete operation, awaiting admission,
access denied and unavailable dependency are separate states. Recovery must
not erase the typed request or mistake a permission/dependency error for 404.
Input stays in memory; leaving an unsaved form requires confirmation. No
real prompt, credential or secret enters a URL, storage, logs or screenshot fixtures.
Only a nonsecret operation ID/key may be navigation context in the later live
controller; readback always rechecks human ownership and project authorization.

This preview has fictional UX-102, projects and agents. All actions affect
memory only. It imports neither auth stores nor API clients and its CSP forbids
form submission. No fixture success is live acceptance.

## Review And Integration Gate

Preview: <http://127.0.0.1:55496/pm-draft-preview.html#/>
List entry: <http://127.0.0.1:55496/pm-draft-preview.html#/?view=list>
Unknown outcome: <http://127.0.0.1:55496/pm-draft-preview.html#/?state=uncertain>
Partial result: <http://127.0.0.1:55496/pm-draft-preview.html#/?state=incomplete>

Four focused component cases cover required explicit selection, recovery,
dependency failure, absent readback and access denial. Generated
[proposal screenshot manifest](../assets/design/pm-draft/manifest.json) records
form/recovery states at 375x812, 768x1024, 1920x1080 and 2560x1440.
It is separate from production screenshots and records `approved=false` and
`liveAcceptance=false`. Explicit user approval is required before connecting
this new form to production mutations. The backend contract can be verified
independently while approval is pending.
