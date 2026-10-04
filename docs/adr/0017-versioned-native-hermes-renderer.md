# ADR 0017: Versioned Native Hermes Renderer

## Status

Accepted source design. Native loader evidence and installed-runtime acceptance
are separate gates; this does not enable SDLC admission.

## Context

Pinned Hermes loads its HOME dotenv with override, can clear inherited keys and
applies gateway configuration after launcher environment. Fleet's historical
renderer did not persist managed API host/port or native platform enablement.
Changing that renderer in place would reinterpret effective historical snapshots
and make their file/hash readback report false drift.

## Decision

Snapshot `renderer_version` defaults to 1 when absent and is omitted when
serializing legacy version 1. The server selects 2 for new Hermes revisions and
1 for Java; the config-edit request cannot choose a version. Unknown versions
fail validation/rendering without fallback. No existing row is rewritten.

Version 1 retains previous config/env/marker output. Version 2 derives loopback
host, assigned port, enabled native `platforms.api_server`, HOME and credentials
from managed identity. It writes native platform host/port and protected dotenv
values and an explicit CORS list (including an empty list), preserving unrelated settings.
Only the canonical `/platforms/api_server` section is supported in v2: root,
`gateway.api_server` and `gateway.platforms.api_server` aliases are rejected.
Platform maps/blocks/extra must be objects, avoiding native YAML merge failures
hidden by dotenv fallback. User API listener/env values cannot
replace managed values; credentials are resolved into private files, not public
snapshots. The marker includes renderer version and actual rendered hashes.

Provisioning uses native output for missing files only; it does not repair or
overwrite existing effective files. Moving an existing agent to version 2 uses
the usual draft/validate/drain/activate/readback/rollback flow.

## Consequences

Legacy history remains reproducible. Rendering and native config-loader tests
can prove that exact fixture bytes load with their intended listener/HOME/key,
with a direct native YAML-layer check before the combined dotenv/config check,
but not that an installed process loaded an effective revision, plugin inventory,
model credentials or frozen assignment. Managed overlays and OS isolation still
need runtime attestation. Existing active/unknown runs must not be bypassed.

The additive snapshot field is generated into OpenAPI/client; no database
migration or dependency/runtime pin change is needed.

## Alternatives

- Set only launcher env: rejected; native dotenv/config precedence can replace it.
- Rewrite the old renderer: rejected; it changes historical file/hash semantics.
- Backfill all snapshots: rejected; no permission to mutate accepted history.
- Treat healthy HTTP as loaded-config proof: rejected; it does not attest revision.
