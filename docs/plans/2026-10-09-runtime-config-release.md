# Runtime Configuration Foundation Release Unit

Status: exact-source Linux backend gates PASS; configuration-only Draft
publication authorized, with foundation47 as a blocking dependency.
No new migration, deployment, runtime readiness or completion is claimed.

## Current Integration Verification 2026-10-11

Candidate `f1b08acff5ba434e4d46477ad149224d01b9dd87` integrates current
Fleet47 `219f94ae04a352fcaec6483ac199b220341b6ced`, including current main
`c39ff84d82277004bf8170fbac2f3b122ea6bcad`. The dependency merge is clean;
the configuration-owned diff against Fleet47 remains 32 files. Frontend,
migrations, Cargo lockfile and build SDK pin are identical to that parent.

On Git-exported sources with Rust 1.88, pinned Base SDK `19a7a381` and real
package objects `4b9b4c9`, workspace fmt and strict Clippy passed. Workspace
tests passed 222 cases; 15 opt-in cases remain ignored. All 48 foundation
integration cases used disposable PostgreSQL 17.6; all seven package roles
used real Git objects. Eight machine configuration API cases passed again.
The real `gen-openapi` output matches the checked-in schema. The first QA
harness named that binary incorrectly after successful tests; only generator
and drift were repeated. Both QA projects removed their own containers,
networks and disposable volumes through journal v2; permanent runtime
identities were unchanged and the external Cargo cache was preserved.

These results qualify backend integration only. Fleet47 native upstream
acceptance, applicable frontend/browser gates, live Workflow readback,
package installation and physical runtime admission remain required.
This PR stays Draft; `runtime_ready=false` is unchanged.

## Initial Parent And Scope

The release branch `feat/runtime-config-release-20261009` was initially based on
foundation47 `8befcb6ba34c58d2d146403cdd683dbf1dafbce3`, not bare main or `4cc9a8a`.
The parent includes normal merge `87c5d08` of foundation and accepted main
`2fad13115f3cb8341cd46679691438b24a1c8ee8`. Foundation47's evidence and reported
five successful CI jobs apply to that parent only, not this configuration delta.
No parent source/history, private-session behavior, PM callback guard, migration,
schema, lockfile, SDK pin, UI or runtime image is rewritten here.

This is the first ordered configuration release unit: verified Base package
drafts, frozen Workflow mapping, exact revision/effective-head reads, identity
fences, validation/activation preflight, managed-file observation and regressions.
It reuses `000009` configuration tables/JSON and the existing activation lifecycle.
The only supervisor change is a fresh Workflow read before apply. The parent owns
`000010_task_chats`; this unit owns zero additional migrations. Later migrations
`000011` through `000022`, container/native-context/admission/controller/observer
modules and the large integration tail are excluded.

## Selective Source Provenance

Read-only donor HEAD: `f97cb604044b3302f31ff6549658cfb9f1ec69d6`.
Production modules were reconstructed from bounded historical changes, not copied
from the donor's whole current tree or cherry-picked with unrelated parentage:

| Source commit | Selected purpose |
| --- | --- |
| `3b1e68e16000059b377d4d0adc48e758e4f6f627` | Verified exact-Git role-package draft |
| `691357f57c75d19418d4ca093206e3221c7b1d9e` | Scoped machine configuration observation and closed HOME readback |
| `dfe1ae74fce8e9f212343092b88b82dbf9abb453` | Direct exact revision/effective-head lookup beyond latest-100 history |
| `b8f188b70189f3bfcd0c7491870ae3fd410275f1` | Bounded Git IO, timeout/kill/reap and strict binary batch parsing |
| `1bbe76ab1261f91d90b5c99309bc679b6ceddc07` | Agent-row-lock identity/rebind fences for drain/run/outbox work |
| `f40a9ad0ae956b4aea682910d6c051dd0d3789b9` | Fresh owner mapping and frozen Workflow v3 identity |
| `d520deb58b335868993cc8b91244200f8920a1f5` | Only closed-skill support/Unix-hardlink refusal and associated tests |

The current donor's pending duplicate-Authorization cardinality guard and focused
zero-introspection regression are selected only for the new configuration route.
Foundation47's existing PM guard is unchanged. New integration `CurrentUser`
fixtures retain foundation47's `central_write: None`. CI wiring supplies real
package objects without replacing any parent lineage/profile/migration gate.
OpenAPI adds only two paths and two schemas, preserving existing PM/Tracker DTOs;
actual Rust-generated OpenAPI byte parity passed in the backend packet below.

## Dependencies (Distinct Pins And Authorities)

1. Release depends on [Fleet47](https://github.com/FerrPOINT/fleet-control/pull/47).
   The original parent was `8befcb6ba34c58d2d146403cdd683dbf1dafbce3`; current
   integration uses `219f94ae04a352fcaec6483ac199b220341b6ced`, then the
   usual main compatibility and PR review. The separate Draft targets `main`,
   not the dependency branch. Until Fleet47 merges, the main comparison includes
   its inherited foundation changes; the owned configuration diff is measured
   against `8befcb6` and adds no migration or UI change. No automatic merge or
   main acceptance of the integration tail is authorized.
2. Build SDK stays `19a7a381ae6dbea61a643bb96189e483fa64df5c` in `.base-revision`.
   A fresh isolated sibling checkout is clean at that SHA; no frozen Forge SDK or
   shared checkout/cache is changed. SDK verification is a lightweight check only.
3. Role content is separately pinned to Base
   `4b9b4c9297a13fb28a6ba2039af2f7cb719f2f58`. That exact object was fetched into
   the candidate's own Git cache without switching the SDK worktree. Runtime reads
   only Git blobs, never fetches or falls back to HEAD/worktree/donor content. CI
   checks out this exact package separately. Availability is not production
   package acceptance: reconcile the accepted package commit/hashes before any
   installation; a repin requires its own reviewed compatibility gate.
4. Workflow must provide authenticated
   `GET /internal/runtime/base/namespace-bindings/{namespace_id}` with the strict
   [v1 binding](../contracts/SDLC_WORKFLOW_BINDING_V1.md), persisted catalog v3,
   canonical numeric IDs, matching namespace/profile/role and skills revision
   equal to the exact package pin. Fleet uses a dedicated server-only Base PAT;
   Workflow must freshly authorize exactly `project-workflow:read`. No installed
   v3, live credential, current Workflow PR compatibility or distributed lease is
   inferred from controlled fixtures or source exports. Absent/mismatched owner
   support blocks package actions; it never falls back to the legacy catalog token.
5. Machine observation is disabled by default. Deployment must register a
   canonical Base reader subject/agent allowlist and current PAT with exactly
   `fleet-control:read`. Fresh introspection precedes repository/filesystem work;
   duplicate Authorization is denied before token parsing/HTTP. Human/admin/local
   tokens and PM callback secrets confer no authority here.
6. Native Hermes discovery/settings/tools/limits/provenance, endpoint attachment,
   frozen assignment ACK/receipts, seven-agent physical acceptance and autonomous
   business completion remain separate later units. HOME readback is not complete
   native inventory. `runtime_ready=false` remains unconditional; observation
   UUIDs cannot authorize scheduling, dispatch or SDLC completion.

## Verified Backend Evidence

Linux Rust 1.88.0/PostgreSQL packet `sdlc-qa-fleet-config-ba43ca138e39`
completed with exit 0 on 2026-10-09 at 11:09:15 UTC. Tested product code is exactly
`011afd9151c828279976aec5e6cf0a28b78c1f69`, build SDK is `19a7a381`, and the
separate real Git package input is `4b9b4c9` with canonical Base origin metadata.
Publication commit `b7ecfbb` changes documentation only; it is not a new
execution of the code gates. The CI follow-up canonicalizes the package origin
created by `actions/checkout` (HTTPS without `.git`) to the exact URL accepted
by the unchanged Rust guard, and asserts the package SHA and manifest object.
Rust/product code remains identical to the tested SHA above. Integration CI
and Fleet47 evidence are not used as acceptance of this delta.

All 18 named gates passed: preflight, fmt, workspace/all-target check, strict
Clippy (`-D warnings`), API2 (2), configuration API (8), Base package (17),
effective configuration (7), foundation (46), workspace tests, lineage (10),
central profile (3), message order (1), chats directory (1), runtime approval
events (1), migration up/status/down-one/up/status, Rust OpenAPI byte parity
and compiled-source parity. All 15 default-ignored DB cases were explicitly
executed in the supplemental gates, not accepted as skips.

The retained private terminal report records 223 source/control files and
source-manifest SHA-256
`288537cad76962de119935f6603e4dfa80cf1b04b62eea313cdab85fa3d9049c`.
All nine parity checks passed. Verified maintenance Base ComposeHelper journal
v2 reached `cleaned`; independent inventory found no owned containers, networks
or disposable volumes, including the exact four volume names. Permanent
`sdlc1`/`sdlc2`/`sdlc-common` images and runtime state remained unchanged. Evidence
and previous failed packets are retained privately; none is an admission receipt.

## Remaining Release Gates

The backend packet does not accept pinned Node22 frontend strict/compatibility,
browser or visual gates, published-head CI/main compatibility, live Workflow v3
and machine credentials, accepted package installation/repin, or native physical
runtime/assignment/SDLC completion. These remain separate requirements. The Draft
must not merge before Fleet47 and the remaining applicable release checks.
Machine `runtime_ready=false` remains unconditional.

## Lightweight Preparation Checks

- Direct installed Rust 1.88.0 `rustfmt --check` parses/formats the 19 changed Rust
  files successfully; no Cargo invocation, compilation or Rust test is implied.
- Pinned SDK verifier passes; sibling SDK stays clean/detached at `19a7a381`.
- Read-only real Git-object/hash verification at package pin `4b9b4c9` passes:
  7 roles, 14 skills, all 21 content blobs match manifest hashes and inventory.
  Manifest SHA-256: `c8634b4767e83fcd09e69805a3b5c68e61788858ccf6b4145b11cf16f4f09112`.
  The Linux packet above also passed the Rust consumer's real-object regressions.
- Structural comparisons preserve parent PM/session/central-auth source and every
  existing OpenAPI path/schema; only the two new paths/schemas are added. New and
  existing `CurrentUser` fixtures preserve `central_write`; migrations, entity
  schema, lockfiles, SDK pin and frontend source are unchanged.
- README validator (3 unit tests), Markdown links and `git diff --check` pass.
  Contract-wrapper tests: 3 PASS, 1 blocked because `yaml` is not installed in the
  isolated candidate; no install was attempted. This is not a compatibility PASS.
  Host Node is 25, not the pinned Node 22 release toolchain; none is frontend,
  browser, compilation or runtime acceptance.
