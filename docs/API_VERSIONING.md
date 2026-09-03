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
