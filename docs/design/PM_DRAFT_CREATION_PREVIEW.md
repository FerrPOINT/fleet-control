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

Preview: <http://127.0.0.1:55498/pm-draft-preview.html#/>
List entry: <http://127.0.0.1:55498/pm-draft-preview.html#/?view=list>
Unknown outcome: <http://127.0.0.1:55498/pm-draft-preview.html#/?state=uncertain>
Partial result: <http://127.0.0.1:55498/pm-draft-preview.html#/?state=incomplete>
Saved chat: <http://127.0.0.1:55498/pm-draft-preview.html#/?state=awaiting&view=chat>
Read-only: <http://127.0.0.1:55498/pm-draft-preview.html#/?state=incomplete&access=readonly>
Denied: <http://127.0.0.1:55498/pm-draft-preview.html#/?state=uncertain&access=denied>
Readback error: <http://127.0.0.1:55498/pm-draft-preview.html#/?state=uncertain&access=unavailable>

State/access query parameters seed fictional examples on document load; view
navigation preserves the current in-memory operation and form. They do not
load real tasks or restore a durable operation after refresh. Unsaved inputs and
captured requests stay in memory, survive list/form/chat navigation and trigger
the browser's leave-page warning. No input text is placed in the URL or storage.

The normal path is Chats → Create task with PM → choose project and PM → submit
the original request → continue saved preparation → awaiting admission → open
the saved chat. No model starts; chat input stays unavailable pending real admission.
For a lost create acknowledgement use `?result=unknown` on the initial form;
`?result=absent` allows the identical original-key retry after explicit lookup.
For a lost continuation acknowledgement use
`?state=incomplete&step=chat&continue=unknown`; readback uses the operation ID.
`?access=empty-page` demonstrates an empty rollout-filtered page that still has
a next cursor. No project or agent is selected when that page advances.

| Actual consumer contract                | Proposed presentation                                                                                                                                                                                                                                          |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `CreatePmDraftRequest`                  | Exact `agent_id`, title, description and original idempotency key; project in the route. Explicit concrete fictional UUIDs are typed against generated DTOs; nothing is sent.                                                                                  |
| `PmDraftCreationResponse`               | All eight fields; `incomplete` keeps `session_id=null`, and `next_step=draft` also keeps `task_id=null`. Never claim a saved task at that step. Other partial steps retain the task; `awaiting_admission` requires task/chat IDs and `dispatch_allowed=false`. |
| Create unknown / `findPmDraftCreation`  | Lookup by original project/key. Only authoritative 404 allows identical replay; permission/dependency failure retains uncertainty and blocks creation.                                                                                                         |
| Continue unknown / `getPmDraftCreation` | Continue and reconcile the same operation ID, without submitting title/description again.                                                                                                                                                                      |
| Project directory                       | `enabled`, strict projects and nullable cursor; empty page is not an empty whole directory or a total. Read-only/denied examples do not fabricate live owner permission.                                                                                       |

Eleven component cases cover exact original-key replay, same-operation continuation,
partial state, input retention, empty-page pagination and denied/read-only holds.
Twelve browser cases in Chromium, Firefox and WebKit cover the clickable path,
keyboard navigation, Back/Forward, all listed screens and Axe accessibility.
Their route guard rejects every API or external request. Twenty fresh captures
cover 375/768/1920/2560 pixels and are recorded in the
[review evidence](../assets/design/pm-draft-review-20261006/manifest.json).
The [original proposal evidence](../assets/design/pm-draft/manifest.json)
remains historical. Approval is still pending; this packet adds no production
form, auth client, API call, runtime, model or producer change.

Reproduce from `frontend` with pinned dependencies:

```powershell
pnpm exec vite build --config vite.pm-draft-preview.config.ts
pnpm exec playwright test --config playwright.pm-draft-preview.config.ts
pnpm exec vite preview --config vite.pm-draft-preview.config.ts
```

The browser suite starts and stops its own preview; the last command serves the
result for human review. Production `vite.config.ts` keeps its existing entry.
Explicit approval is required before connecting this new form to mutations.
Approval also leaves the runtime/producer prerequisites in the
[compact PM handoff](../CHATS_PM_FINAL_HANDOFF_20261006.md) open: first dispatch,
event delivery, accepted checkpoint and resume/rebind cannot be inferred here.
