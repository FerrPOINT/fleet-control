# Runtime Configuration Foundation Release Unit

Status: isolated source candidate; exact-candidate Linux gates pending.
Parent scope review permits a scoped local source-freeze commit, not publication.
No new migration, publication, deployment or runtime acceptance is claimed.

## Parent And Scope

The release branch is `feat/runtime-config-release-20261009`, based exactly on
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
actual Rust export parity remains pending.

## Dependencies (Distinct Pins And Authorities)

1. Release/publication depends on foundation47's exact reviewed parent, then the
   usual main compatibility and PR review. This local branch is not permission to
   merge/push, stack a remote PR or claim main acceptance of the integration tail.
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

## Next Gate

Parent reviewed the scope and authorized a task-owned local commit after light
checks. No push/new PR or heavy gate is authorized or performed by this packet.
After the local freeze and frozen-commit review, export immutable
sources from the exact candidate Git SHA and run the Linux Rust 1.88.0/PostgreSQL
and OpenAPI/strict commands in [TESTING](../TESTING.md#configuration-foundation-candidate-gates).
Do not reuse integration CI as proof for this branch. `FLEET_TEST_DATABASE_URL`,
`FLEET_MIGRATION_TEST_DATABASE_URL` and `FLEET_TEST_BASE_PACKAGE_CHECKOUT` must all
be present; skipped real-object/DB tests are not acceptance.

Local QA waits for the exclusive heavy slot. Use verified maintenance Base
ComposeHelper journal v2 from `SDLC_MAINTENANCE_BASE` User environment, source
exports from Git SHA, explicit owner/purpose and exact disposable cleanup with
independent empty inventory evidence. Preserve external caches, immutable sources,
runtime images/volumes, backups and secrets. No custom local QA helper, accepted
runtime mutation or physical completion receipt is introduced by this packet.

## Lightweight Preparation Checks

- Direct installed Rust 1.88.0 `rustfmt --check` parses/formats the 19 changed Rust
  files successfully; no Cargo invocation, compilation or Rust test is implied.
- Pinned SDK verifier passes; sibling SDK stays clean/detached at `19a7a381`.
- Read-only real Git-object/hash verification at package pin `4b9b4c9` passes:
  7 roles, 14 skills, all 21 content blobs match manifest hashes and inventory.
  Manifest SHA-256: `c8634b4767e83fcd09e69805a3b5c68e61788858ccf6b4145b11cf16f4f09112`.
  The Rust consumer's real-object regression still must run in the Linux gate.
- Structural comparisons preserve parent PM/session/central-auth source and every
  existing OpenAPI path/schema; only the two new paths/schemas are added. New and
  existing `CurrentUser` fixtures preserve `central_write`; migrations, entity
  schema, lockfiles, SDK pin and frontend source are unchanged.
- README validator (3 unit tests), Markdown links and `git diff --check` pass.
  Contract-wrapper tests: 3 PASS, 1 blocked because `yaml` is not installed in the
  isolated candidate; no install was attempted. This is not a compatibility PASS.
  Host Node is 25, not the pinned Node 22 release toolchain; none is frontend,
  browser, compilation or runtime acceptance.
