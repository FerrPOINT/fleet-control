# SDLC Workflow Binding V1

Status: source contract, not installed runtime acceptance or execution admission.

## Owner And Authority

Workflow owns namespace IDs/names, workflow IDs/keys, catalog version and its
declared Hermes profiles. Base owns the exact role package, namespace symbol,
profile declaration and skill/instruction hashes. Fleet does not equate a
namespace symbol with a database ID or let a caller select a mapping.

`GET /internal/runtime/base/namespace-bindings/{namespace_id}` is a Workflow
machine read. The credential is a freshly introspected Base PAT with exactly
`project-workflow:read` and a subject registered as its Base catalog reader.
Legacy catalog tokens, local browser/admin auth and role-reader credentials
do not authorize this observation. The owner must validate its persisted v3
candidate; source export alone or an installed v2 catalog cannot satisfy it.

## Wire Shape

The response is `{ "ok": true, "binding": { ... } }`. Binding fields:

| Field | Meaning |
| --- | --- |
| `schema` | `base-sdlc/workflow-binding/v1` |
| `namespace_id` | Positive canonical signed-64-bit decimal ID string |
| `namespace_name` | Persisted symbolic name matching the Base role |
| `workflow_id` | Namespace's persisted workflow ID, same decimal format |
| `workflow_key` | Canonical `hermes-sdlc:<role_key>` |
| `role_key` | Base role; `devops`, not legacy Fleet `dev_ops` |
| `profile` | Workflow-declared profile, not a native runtime observation |
| `catalog_version` | Exactly `3` |
| `catalog_sha256` | Lowercase SHA-256 of the canonical candidate source |
| `skills_revision` | Exact Base package commit |
| `runtime_ready` | Always `false` |

Fleet rejects unknown/missing/duplicate DTO fields, noncanonical IDs and changed
role/workflow/version/readiness claims. Transport is bounded to 16 KiB and five
seconds, with no proxy, redirect, retry or credential fallback. Only HTTP 200
and identity-encoded replies are consumed. Raw dependency errors/content never
enter public error responses or audit.

## Frozen Configuration

Preparation compares the observation to the concrete agent IDs and the verified
Base package namespace/profile/commit. The mapping is frozen separately as
`config_json.fleet_sdlc_workflow_binding`; `namespace_id` and `workflow_id` in the
configuration carry the real IDs. Proof remains `fleet_sdlc_package`, verified
against exact Git blobs, not against a client-editable JSON claim.

Revision creation and activation recheck current agent identity under its row
lock. Validate, activation request, supervisor apply, readiness and configuration
observation also perform a fresh owner read and compare all frozen fields. Drift
blocks the action. Existing revisions without Base package metadata remain legacy;
old package drafts without a frozen mapping must be prepared again, not converted
by matching names. Preparing a draft does not install v3 or mutate active files.

This is not a distributed lease: Workflow may change after a readback. Trusted
assignment binding, native Hermes model/profile/tools/inventory attestation and
receipts remain required before work. The observation cannot waive Workflow's
owner-evidence gate or cause Tracker scheduling.

## Evidence

Candidate tests require real private Git objects and PostgreSQL plus
controlled owner HTTP metadata. They are not yet executed on this release candidate.
They cover ID/name separation, drift, row-lock
identity changes, credential nonserialization, API validation/activation denial
and legacy fallback refusal. These fixtures are not live cross-service auth,
native profile loading or autonomous acceptance. See [testing](../TESTING.md)
and [runtime configuration](../SDLC_RUNTIME_CONFIGURATION_V1.md).
