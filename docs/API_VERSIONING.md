# API Versioning

Current version: `/api/v1`.

Rules:

- Backward-compatible fields may be added to responses.
- Request fields may be added only when optional or guarded by explicit feature
  negotiation.
- Removing fields requires a new API version.
- Renaming public fields requires an alias period. Example:
  `agent_id` remains a legacy alias while `primary_agent_id` is the public name.
- Runtime adapter contracts version independently from HTTP API versions.
- OpenAPI regeneration is required before release.
- Frontend generated types must be updated in the same change as API source
  changes.

Breaking-change checklist:

- Update OpenAPI and generated TypeScript.
- Update docs/API.md and docs/TRACEABILITY.md.
- Add migration and compatibility tests.
- Capture affected UI screenshots.

## Explicit Runtime Security Migrations

Two deliberately incompatible v1 security retirements are recorded, not claimed
as backwards-compatible changes:

- Run-wide approval no longer returns success: use an exact approval request
  decision; the legacy operation returns409 without a runtime effect.
- Free-chat `POST .../runs/{run_id}/steer` and `.../stop` require the exact
  `Idempotency-Key` header. Missing/invalid keys fail before dispatch. A server
  generated fallback would make a lost reply unsafe to retry, so there is none.

These human stop/steer operations additionally require middleware-verified human
session proof, matching targeted approvals. An authenticated sessionless token
is no longer sufficient, even for an administrator or with an idempotency key.
Clients must use a verified human login/session; custom headers cannot assert it.
There is no unscoped machine-token fallback. Assignment-scoped machine control
remains a separate blocked contract until admission is implemented. This is an
authorization security retirement without a DTO change, not a claim that every
existing token/client remains compatible or that schema checks prove identity.

Exact approval decisions also retire unjournaled/legacy runtime effects. They
require the original accepted free-chat dispatch context, fresh native approval
capability and the currently waiting exact request in the pinned native session.
Task/PM records alone cannot grant that permission; fenced admission is still
required. A preflight failure retains the reserved uncertain receipt without
POST, never a later implicit retry. Historical reads/replays remain supported.
This is a runtime authorization/protocol security change without a new DTO or
schema-compatibility exemption. Native loaded configuration generation is not
attested by an origin or credential fingerprint; that remains a separate gate.

Clients must freeze one key with the original run/operation/payload and read the
durable receipt after an unknown result. A changed payload needs a distinct new
intent only when no unresolved control holds the run; never rotate the key to
retry an unknown effect. Task controls remain blocked until fenced admission.

The additive original-key GET lookup resolves a lost command ID without another
mutation. It requires the original key in exactly one header, current verified
human/session/project authorization and the canonical normalized payload hash.
It searches only that actor's key and returns the existing receipt. No client
may turn404 or an unacknowledged receipt into permission to retry POST. Existing
ID/list routes remain available; the mandatory key and duplicate-header refusal
are not waived for legacy mutation clients. See [API](API.md#durable-free-chat-controls).

Fleet's compatibility wrapper checks these closed security migrations before
running the unchanged Base compatibility checker. Only the exact required string
header on the two operations is added to the comparison baseline; other headers,
responses, request bodies, routes and types are still checked. Dedicated tests
reject missing/optional/duplicate/renamed/typed headers and unrelated breaking
changes. This policy is explicit migration evidence, not ordinary compatibility.
Release requires migration13 after12, updated clients and exact-head CI; legacy
clients without keys are not supported for these unsafe control operations.
