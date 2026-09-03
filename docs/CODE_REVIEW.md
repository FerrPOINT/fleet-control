# Code Review

Review checklist:

- Does backend RBAC enforce the behavior, not only frontend gates?
- Are session defaults private unless a leader is explicitly selected or the
  primary agent is a leader?
- Are leader/executor bindings validated server-side?
- Are create/send operations idempotent?
- Are secrets redacted in API responses, logs and audit payloads?
- Are runtime writes routed through adapters?
- Are database indexes present for new list/filter patterns?
- Are OpenAPI and generated frontend types updated?
- Are screenshots regenerated for changed routes?
- Are migrations reversible and tested on a clean DB?
- Are Java Agent phase 2 failures typed and visible?
