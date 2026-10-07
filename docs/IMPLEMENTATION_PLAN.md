# Implementation Plan

## Current Integration Follow-Up

Chats94e889d and runtime46df7aa are combined by normal merge. Tab reload recovery
and late PM ACK authority checks are no longer missing frontend wiring; retain
their original-key and unknown-outcome holds. The four shell regression fixes
are tested with the actual pinned Base SDK, not a cached sibling SDK checkout.
See [evidence](CHAT_CLARIFICATION_VERIFICATION.md#chats-reload-and-identity-integration-7-october-2026).

Next close same-SPA identity isolation for query caches, permissions, in-flight
auth callbacks and private drafts, with negative cross-user tests. Do not discard
an unresolved original command and treat local cache reset as a redispatch permit.
Keep actual producer/PM/runtime/Forge acceptance and ordered main-release work
below; a source-branch push does not enable or deploy automatic SDLC.

## Connect Fenced Controller Recovery To Native Custody

The000020 storage flow reserves immutable recovery epochs under
the agent-row lock, with exact original hashes/PID, versioned30-second DB-clock
lease, idempotent readback and held unknown acceptance. Any epoch fences old
queue/permit/endpoint/lifecycle effects. Candidate000021 now retains the original
native command, commits one dispatch claim and saves a validated original-key ACK
atomically with owner outcome and audit. Its trusted entry point is not connected
to automatic startup. Base handover/private epoch source is published at3facb28;
pins and installed runtime remain unchanged. Complete10-second dual heartbeat,
fresh dual-fence effect admission and interrupted activation settlement before
enabling restored execution.
Prove actual restart, competing owner and interrupted activation; do not rewrite
the original launch or use expiry as a redispatch permit. See
[the exact contract](contracts/CONTROLLER_RECOVERY_V1.md). The fresh562-case
Linux/PostgreSQL source gate passes; see
[evidence](CHAT_CLARIFICATION_VERIFICATION.md#fenced-controller-recovery-storage-7-october-2026).
Candidate000021 historical positive outcome after expiry without renewal passes
the fresh568-case component gate. Next prove actual Fleet/Base/ongoing-Hermes restart, next owner
against physical cessation, and lost native ACK across OS-process crashes.

## After Integrating The Original-Key Chats Consumer

Normal merge49c11f5 combines the independently published Chats16b7516 and
runtime9a11bde without modifying backend bytes. Original-key recovery is now
production frontend behavior, not a remaining adapter wiring task. See
[combined verification](CHAT_CLARIFICATION_VERIFICATION.md#integrated-chats-and-runtime-candidate-7-october-2026).

The separate Chats task next proves browser reload/logout/identity-change
recovery using only opaque command identity and server-authorized GET. It must
not store raw guidance, transcript or credentials, release uncertainty from a
local cache, or duplicate a POST. Resolve the pinned SDK stream-denial finding
separately; successful component/fixture rechecks do not close it.

Runtime work continues below: durable fenced controller transfer and interrupted
activation, original-generation bounded/redacted collection, exact producer
admission/first-step authority, PM checkpoint/delivery and actual Forge/SDLC
acceptance. Keep the compatible published release lineage and one-migration
boundary; no installed opt-in follows from merging source or fixture screenshots.

## Controller Ownership Recovery After Read-Only Witness

The Base same-container restart observer is published at
[9171fe6](https://github.com/FerrPOINT/services-base/commit/9171fe6b1b6b05b9504d33fb881f274c0b5541e0).
Actual native proof retains the original running synthetic agent and byte-exact
journals with lifecycle effects denied. Fleet's typed consumer verifies original
mapping/registration plus fresh immutable DB launch/ACK/PID for degraded health;
this is not adoption. See
[evidence](CHAT_CLARIFICATION_VERIFICATION.md#original-controller-restart-observation-7-october-2026).

Complete recovery with a durable CAS/lease owner record and immutable controller
epoch history, without rewriting original launch identity. Only exact retained
private source/config/launch witnesses and confirmed old controller cessation
may authorize the new owner. Reconcile activation checkpoints and unknown native
commands before resuming; a lost journal, competing owner, new container ID or
source-policy upgrade must remain held until its explicit recovery rule is proven.
Validate real Fleet/Base/Docker recovery and ongoing run before installed opt-in.
Keep release migrations isolated and ordered. Base PR150 remains a prerequisite
and currently conflicts with main; do not fold its unrelated history into a new
release PR or change the installed SDK pin from these utility tests.

## Public Control Recovery Consumer Handoff

The verified-human original-key GET is implemented and covered by the fresh
551-case full gate. The local client is generated from Rust OpenAPI. See
[consumer contract](contracts/HERMES_RUN_CONTROL_V1.md#fleet-command-journal)
and [verification](CHAT_CLARIFICATION_VERIFICATION.md#original-key-control-lookup-7-october-2026).
The published Chats16b7516 consumer, now included in49c11f5, retains original
key/run/input, hashes the same Rust-normalized semantic payload and releases a pending hold only
from a fresh exact accepted receipt. Unknown,404 or terminal-without-ACK must not
become a second POST. Keep actual native reply-loss/restart acceptance separate
from these component tests; no installed runtime flag changes accompany this GET.

## Verified Native Credential And Input Slice

The fresh actual two-agent Rust/Base/Docker gate verifies six native model runs,
static provider rotation after drain, peer/restart isolation and real readiness
rollback. Six retained dotenv creation intents match original PostgreSQL fences.
This advances launch/input acceptance but does not enable a collector or replace
the coherent production flow below. All effective secret sources/reloads, bounded
retention, atomic private checkpoint/range commits, controller recovery and
predispatch/PM/Forge acceptance remain mandatory. See
[native evidence](CHAT_CLARIFICATION_VERIFICATION.md#native-provider-rotation-and-original-input-custody-7-october-2026).
No migration, API, SDK, UI or installed runtime change accompanies this test packet.

## Next Runtime Work After Private Log Transport

Base source [d0eedc1](https://github.com/FerrPOINT/services-base/commit/d0eedc16336386ca1a8827387d806d9c9a89b919)
now implements closed private `log_page`: per-stream byte offset and verified
SHA256 prefix,16 KiB pages, bounded64 MiB scan and double original readback.
All137 Linux runtime cases pass without skips; native Docker CLI reads all5000
synthetic records before/after stop and rejects a real rotating source. This is
source/transport evidence, not a Fleet collector or release. See
[evidence](CHAT_CLARIFICATION_VERIFICATION.md#verified-base-log-source-pages-7-october-2026).
The utility still depends on open Base PR150; SDK pin remainscbb4e99.

Fleet's private Rust client now decodes the same closed `log_page` envelope,
validates exact requested range/original receipt and retains raw binary bytes
without Debug/public serialization. This establishes the transport consumer,
not a DB collector or actual Rust-to-Base/Docker ingestion acceptance. The
generation-bound production flow below remains mandatory; do not wire pages
straight into generic `insert_log` or advance cursors in memory only.

Implement the remaining collector as one coherent generation-bound flow:

The original-input prerequisite now freezes guarded dotenv bytes/hash in each
new automatic private creation intent before create. Revision-bound launches
match the rendered file and retry/start checks reject drift. Historical intents
are not backfilled. This is not completion of item1: the pinned native loader
interpolates/sanitizes credentials and can apply external/managed sources. Prove
and freeze its actual effective values without storing delegated child credentials
or relying on the current environment; the collector remains disabled.

1. Freeze actual resolved launch credentials in private controller storage before
   execution; never reconstruct old secrets from the current rotated environment.
2. Validate original launch/controller/registration and retain partial line/UTF8/
   secret chunks privately. A staged raw checkpoint must be durable before the DB
   transaction references it; it must not become a public log or plaintext DB field.
3. Atomically commit redacted source ranges, durable cursor/version and delivery
   events under the original launch fence. Identical content is not dedup identity.
   A failed/unknown commit reconciles the original range; it does not skip forward.
4. Restore the exact referenced private checkpoint after crash. Preserve the old
   checkpoint until the new DB commit is confirmed; missing/tampered state holds
   collection instead of adopting a new generation or discarding backlog.
5. Define bounded disk/retention supervision before enabling this append-only
   source. The utility does not change runtime logging settings; overflow/rotation
   or unsupported effective driver is a hold, never an empty successful poll.
6. Prove actual Rust -> Base -> PostgreSQL ingestion, secret rotation/split handling,
   duplicate replay/crash recovery and authorized API/SSE. Keep migration releases
   in order; this transport packet adds no migration or public API.

The shared repository log-acknowledgement race is fixed and verified on both
main-based and runtime source. See
[the PostgreSQL evidence](CHAT_CLARIFICATION_VERIFICATION.md#atomic-process-log-acknowledgement-7-october-2026).
Do not implement collection by repeatedly inserting the current200-line tail:
timestamps/content hashes are not reliable source offsets, and identical lines
can be distinct real records. Freeze the original resolved per-launch credentials
before rotation, and atomically commit a verified source range with its cursor.
Missing prefix, overflow or rotation needs explicit gap/reconciliation semantics,
not silent deduplication or a successful empty poll.

Private Base Docker log readback now passes on four original exited generations
of two actual Hermes agents. Rust's typed client is implemented and passes the
fresh528-case Linux gate, but this is not production ingestion. Connect it only
with current launch custody, exact resolved per-launch secret redaction and
generation-bound durable cursor/deduplication/overflow/rotation semantics;
then prove actual Rust-to-Base ingestion and authorized API/stream output.
Complete interrupted activation/controller recovery and the unchanged
predispatch/first-step/PM/Forge acceptance next. Do not invent missing producer
authority or use a healthy runtime as SDLC readiness.

Keep Base control source69831aa separate from SDK pin cbb4e99. Its prerequisite
PR150 is ready/MERGEABLE at7d7323a with9 successful exact-head CI jobs, but still
open and not installed. Retain ordered release rather than
publishing a predecessor-heavy log-only main PR. Fleet's historical migrations
still require the agreed one-migration-per-release-PR order. Nothing is installed
or enabled from this packet. See
[verification](CHAT_CLARIFICATION_VERIFICATION.md#actual-private-base-docker-log-readback-6-october-2026).

## Current Actual Docker Proof

Real original-Engine/UID 999 Rust Fleet/Hermes/chat/config acceptance now passes
for two mapped-volume agents and five controlled-model runs. Additive `000019`
binds the actual Base endpoint to original launch/PID and removes the old
localhost-only container-dispatch assumption without an arbitrary endpoint setter.
Fresh broad/migration gates pass526 workspace cases (30 explicit opt-in ignores),
fmt/check/strict Clippy and19 migration cases without ignores. Publish this scoped
candidate and preserve the ordered one-migration-per-release-PR boundary, then
prove actual Docker readiness rollback, controller recovery/private-journal loss,
logs and the unchanged producer/PM/Forge requirements. No installed opt-in or
"full merge-ready" claim follows from this one gate. See
[verification](CHAT_CLARIFICATION_VERIFICATION.md#actual-docker-supervisor-chat-and-configuration-6-october-2026).

## Current SDLC Scope

The current focused-tested candidate implements container configuration replacement
through original namespace stop and fresh activation/rollback generations.
Eight focused regressions cover exact pre-create admission, stopped apply,
pending custody holds, normal replacement, readiness rollback and unknown or
foreign effects.31 focused cases and strict Clippy pass. The full exact-source
Linux gate passes523 cases with29 explicit ignores and fmt/check/Clippy.
Complete real mapped Hermes/model acceptance before claiming release readiness.
This is not controller takeover
or completed PM/Forge/SDLC acceptance.

The current work adds additive000018 pre-create DB authority: immutable
agent/history ordinal/controller/generation/operation/intent hash before both
private intent creation and Docker prepare. Exact replay, missing-file holds,
competing-controller fencing and blocked native fallback have new regressions;
fresh full/migration gates pass515 workspace cases (29 explicit ignores),
fmt/check/strict Clippy and19 migration cases without ignores on ten disposable
databases. Both gates capture the same290 unchanged backend/SDK files and owned
resources are cleaned. See [exact evidence](CHAT_CLARIFICATION_VERIFICATION.md#durable-container-pre-create-fence-6-october-2026).
This is not actual
mapped Hermes/model acceptance or complete private Base journal recovery.
Base PR150's main conflict was normally merged into424ad76 without rewriting
history. All9 exact-head CI37480593149 jobs pass; the candidate is ready/mergeable
after a fresh post-ready check, with no reviews/threads. Utility bytes are
unchanged. Earlier98a5bbd readiness below is historical; installed state and
producer permissions remain unchanged.

The current candidate additionally merges consumer `d592a0d` (HTTP408 remains
unknown). Fresh262 frontend tests and33 fixture browser cases pass, with
regenerated21-image preview evidence. All288 backend/SDK bytes match the
previous successful workspace/migration gates; no backend rerun is claimed.
The live acceptance requirements remain unchanged.

Chats through `0ecee7e` and runtime parent `7564e2e` are normally merged in the
integration candidate. Combined-tree gates pass511 workspace tests with29
explicit ignores,18 fully configured migration tests,259 frontend tests and33
fixture browser cases;21 preview screenshots are regenerated. See the
[combined evidence](CHAT_CLARIFICATION_VERIFICATION.md#combined-chats-and-runtime-candidate-6-october-2026).
This is not main/installed acceptance; later parallel Chats changes need another
scoped integration. The real runtime/producer/PM/Forge requirements below remain.

Fleet source `d924799be5ec77935b71decce20059f22b919e10` now consumes private
protocol2/boundary policy3 named-volume subpaths. Original mapping/file/hash are
bound in creation intent, prepared receipt, registration and DB launch provenance.
Fresh guards preserve local AgentPaths/marker ownership; protocol downgrade,
changed controller/proof, sibling mounts and unknown-start resend are denied.
The fresh Rust1.88/Linux/PG gate passes509 tests with29 explicit ignores and
fmt/check/strict Clippy; see [verification](CHAT_CLARIFICATION_VERIFICATION.md).
No SDK/public API/migration/deployment flag changes or installed rollout.

Next prove actual Rust Fleet -> named-volume Hermes -> model/chat with the real
controller UID, then safe new-generation config drain/activation/rollback, logs,
controller restart reconciliation and the remaining producer admission/PM/Forge
gates. Private-controller document/journal loss and restore must also be tested
before rollout; preserved-file replay is not proof of storage-loss recovery.
Base PR150 remains ready/mergeable on98a5bbd with9 SUCCESS jobs CI37465043730;
it is not merged/installed. The full SDLC goal remains open.

## Historical Runtime Gates

Base follow-up2bcf3d2 now proves read-only named-volume mapping with real UID999
controller files, original snapshot/Engine/volume hashes and drift rejection.
This is not yet a Fleet consumer: persist the original mapping in creation intent,
revalidate before all effects and extend Base guarded lifecycle validation to
use the corresponding local paths. Do not merely replace policy sources and
bypass existing path guards. CI37461023808 passes all nine jobs for this new
ready/mergeable candidate, reread after ready with no reviews/threads;
the ready368cfb8 observation below is historical. See current verification.

The current published runtime integration is cf4b08e, including trusted Fleet
bridge attachment. Its full Rust1.88/Linux/PG gate passes504 tests with29 ignored;
the earlier503/502 gates below are historical. The attachment fixture is not
actual Rust Fleet/container/model/chat acceptance.

Read-only installed-runtime inspection on6 October confirms a remaining blocker:
Fleet agents_root is `/var/lib/fleet-control/agents`, whereas the Docker Engine
bind source is `/var/lib/docker/volumes/sdlc1_fleet_agents/_data` (independently
`sdlc2_fleet_agents/_data` for workspace2). Both Fleet backends run UID/GID999;
the previous Base Hermes startup QA ran UID/GID10001. Do not pass the controller
path to the daemon, guess another volume, change protected ownership or accept
the previous host-bind QA as volume/permission proof. The next runtime packet
must seal the exact controller/Engine mount mapping, preserve local path/marker
guards and prove write/read access for the explicitly selected runtime UID.
See [current verification](CHAT_CLARIFICATION_VERIFICATION.md).

Base PR150 has been normally merged with accepted main475c694 without force push;
candidate368cfb8 is ready/mergeable with nine successful exact-head CI37458583952
jobs, reread after ready. Local377 Python cases contain366 PASS/11 skips;
26 Node runtime-contract tests and README/hub pass.
Utility executable bytes are unchanged by this history reconciliation.

Prepared-generation recipe checking is now implemented: original process/policy,
derived runtime API token and registration are revalidated before claim/start.
The new seven-case regression and fresh503-test Rust1.88/PG workspace gate pass
with29 ignored, fmt/check/strict Clippy and exact cleanup. The502-case source-loader
gate below is retained as prior evidence. Current fixture-backed verification does
not replace the next actual Fleet/container model/chat and config lifecycle gates.

The captured-source Base utility loader follow-up closes the file-hash/import
gap: fixed isolated Python compiles the captured pinned bytes, not cached code
or reread checkout files. Host behavioral coverage is available and added to CI;
all four Linux Rust regressions and the final502-case workspace gate pass.
This cannot replace controller attachment, filesystem mapping or actual Hermes
container/PM acceptance. Previous own QA cleanup is now verified on the original
Engine; the final full gate exits0 with exact own cleanup and288 matching inputs.

Docker consumer is now connected in the integration candidate: immutable
container registration/paths/config/context/source pins precede Base start;
start/health/stop and every Hermes endpoint use original container receipts.
Unknown start stays claimed and cannot resend or fall back to native Hermes.
Automatic first-generation preparation now saves a private original intent
before Base render/create/register and writes its matching prepared document.
Operator-prepared generations remain a compatibility path. Trusted controller
bridge attachment is now implemented; next implement daemon source mapping, UID/file access,
logs and namespace-based config rollback. The follow-up history-ordinal restart
candidate now preserves previous files and creates a new generation only after
confirmed original exit; its new full Rust/PG gate was interrupted by host disk
exhaustion/Docker unavailability. Own project3b96063dfdea is now verified clean;
the final502-case gate passes without relabelling the partial174-unit pass. Real Docker
restart/Fleet/Hermes acceptance remains required.
Do not enable installed Docker mode or label automatic provisioning complete.
Task admission, producer first step, PM tools/resume and real Forge/seven-agent
acceptance remain separate required gates.

The full fresh Rust1.88/PG candidate gate passes494 tests with29 explicitly
ignored profiles, all-target check/strict Clippy and fmt. See
[verification and retained failure](CHAT_CLARIFICATION_VERIFICATION.md).
This gate includes the initial preparation consumer and fake-Base PG fixtures,
not actual container/Hermes or full PM acceptance. Do not relabel the
ignored/live gates as complete. The prior490-case gate remains historical.

Historically, Base PR150 at6041530 was ready/mergeable with nine successful exact-head jobs in
CI37449729086; checks were reread after ready and reviews/threads are empty.
PR144 is merged at main63fff28. This certifies the shared utility, not installed
Fleet/Hermes acceptance. Native real-Hermes startup/stop and synthetic creation
controller exits/v1 regression are recorded separately from Rust fixtures.

## Historical Native Packet

The earlier Base release was d2c8ef6, preserving accepted maina3d6a79 with unchanged
boundary/bootstrap bytes and nine green jobs in CI37409952184. The a0f7044
observation below is historical. Fleet2293862 is published integration source;
its229 component/38 harness cases and fresh native lifecycle are scoped local
evidence, not readiness for PM, host-boundary consumption or automatic SDLC.

The reconciled foundational release PR47 now has five green exact-head CI jobs
at `5f20540`, but stays Draft for missing live PM acceptance. Base PR144 has nine
green jobs at `a0f7044`; its host primitive still is not a Fleet consumer.
The separate integration follow-up rejects dead retained-child dispatch and
foreign-controller runtime overwrites/false HTTP health-transition alerts.
Its fresh compilation, component and native evidence must be accepted separately;
neither foundational CI nor fixture screenshots certify the later runtime tail.

The pre-spawn native launch journal `000017` binds agent/config/controller before
execution and holds unknown outcomes across Fleet replicas. Component validation
is recorded separately; this does not complete the host-boundary integration,
loaded config, safe descendants or task admission. Release after `000016` as its
own one-migration packet, never by rewriting applied migration history.

The packet also pins native launch identity in private dispatch intents and
checks it before preparation, permit consumption and actual HTTP submission.
An old or legacy intent cannot follow a replacement managed gateway. This is
not loaded-generation attestation or full task admission.

Source reconciliation with accepted main `3c6b8ef` now preserves canonical and
legacy split foundations, verified central names and the accepted Base SDK.
The actual PostgreSQL lineage suite verifies both populated histories through
the shared eight-step tail (18 canonical/21 split entries), not just a clean
installation. Historical migration bytes are unchanged. Ordered single-migration
release packets, exact-head CI and review are still prerequisites. Keep PR47
and dependency packets unchanged until their owners perform that process.

The historical phases below are not completion evidence for the October SDLC plan.
Current implementation, boundaries, blockers and acceptance are maintained in
[SDLC_IMPLEMENTATION.md](SDLC_IMPLEMENTATION.md). Leaders are deferred. Java lifecycle
exists, but runtime chat/control still returns phase-2 errors and cannot run SDLC.

Original approval journal000016 and its opt-in consumer are implemented.
Original-mode HTTP reservation, exact-byte approval POST and bounded GET-only
recovery require their recorded component gates. Native positive/unknown decision
outcomes, separate Fleet SIGKILL recovery and combined run/command plugins are
verified in disposable native QA. Installed rollout and complete SDLC remain.
Journal component
checks do not close those gates or enable installed runtime flags. Ordered
one-migration release packets and exact-head CI/reviews still precede rollout.

Phase 0: pre-development hardening.

- RBAC and permissions endpoint. — done: `SystemRole = admin|operator|user` c бэкенд-энфорсментом (middleware + `GET /users/{id}/permissions`, `PATCH /users/{id}/role`), legacy `is_system_admin` alias; см. docs/AUTHORIZATION.md.
- Idempotent sessions/messages. — done: миграция 0004 (`idempotency_key` + `idempotency_payload_hash`, unique `(user_id, idempotency_key)`), replay возвращает исходную сессию/сообщение.
- Session participants, leader selection, handoff and delegation. — done: `leader_agent_id`, роли `leader|executor`, `/sessions/{id}/participants`, handoff/delegation-роуты (docs/API.md §sessions).
- Deployment jobs and settings surfaces. — done: `deployment_jobs` + bulk `POST /deployments/jobs/bulk`, settings API с per-key аудитом (docs/API.md).
- Product pages for leaders and executors. — done: `frontend/src/pages/{leaders,executors}` + карточки агентов/сессий.
- Technical pages for agents, deployments, logs and settings. — done: `frontend/src/pages/{agents,deployments,logs,settings,alerts}`.
- Screenshot manifest and evidence capture. — done: 82 desktop-файла в `docs/assets/screens/` + manifest.md (`1920x1080` и `2560x1440`).
- Documentation and ADR alignment. — done: полный док-паритет с task-tracker (55 файлов), ADR + ADR_INDEX синхронизированы.

Phase 1: Hermes MVP completion.

- Finish real Hermes API session open/send/stream integration. — done: Hermes `/v1/runs` адаптер (open/send/SSE mirroring/stop/steer/approval forwarding) — см. CURRENT_STATE.md.
- Expand fake Hermes lifecycle tests into real adapter contract tests. — done: контрактные тесты адаптера на фикстурах `backend/tests/fixtures` (Run-переходы, SSE, approvals).
- Add runtime reconciler tests for desired-state restart. — done: `reconcile_action(status, desired)` решает Restart/HealthCheck/None (unit-тесты переходов), reconciler-цикл перезапускает failed/stopped агентов с desired=running и health-checkает running.
- Add clean DB migration and seed workflows. — done: SeaORM-миграции 0001+ (idempotent IF NOT EXISTS), seed в тестах через фикстуры.
- Replace the local HMAC token validator with `sdlc-auth-core::Validator::hmac`
  after WSL/CI can fetch `services-base`. — done: `AuthService` валидирует через `sdlc_auth_core::Validator` (hmac-mode), локальный дубликат decode-логики удалён; см. docs/AUTHORIZATION.md.
- Add OIDC/JWKS validation mode and retire the compact-token legacy fallback
  after the transition window. — done: `auth.mode=oidc` — RS256/JWKS (кэш+refresh, kid-miss), строгие iss/aud, маппинг ролей, local login и HMAC-токены отклоняются fail-closed (см. docs/ENV.md, docs/OPERATIONS.md).

Phase 2: Java Agent runtime.

- Implement Spring Boot launch/provision adapter. — done: supervisor поднимает `java -jar <agents_root>/agentN/runtime/backend.jar --spring.profiles.active=noop`, readiness по `/actuator/health/readiness` (db-only).
- Wire health, capabilities, sessions and chat stream. — partial: health/readiness and lifecycle exist; chat/control remain phase 2 in runtime/mod.rs.
- Add Java Agent runtime tests and screenshots. — done: runtime-тесты provision/launch/readiness + скрин-свидетельства java-agent-страниц в evidence-сете.

Phase 3: fleet operations.

- Add operator retention policy thresholds and scheduled stale-folder review. — done: `fleet.retention.stale_archived_days` / `fleet.retention.review_interval_secs`, fleet-wide `GET /api/v1/agents/storage-review`, stale flag + archived days in storage report, scheduled review worker, `POST /api/v1/settings/retention/review`.
- Add richer monitoring and alerts. — done: `fleet_alerts` (миграция 6): авто-алерты переходов здоровья (agent_down/agent_recovered), `GET /fleet-alerts`, acknowledge (Operator+), авто-закрытие открытых и подтверждённых алертов после восстановления, аудит; событийная модель включает restart-loop/heartbeat-stale.
- Add bulk runtime updates and rollback. — done: `POST /api/v1/deployments/jobs/bulk` (см. docs/API.md); rollback помечает runtime_update jobs через `detail.rollback`.
- Add cross-project workflow health integration. — done: `refresh_workflow_bindings` сверяет биндинги с живым project-workflow каталогом (reconciler, runtime/mod.rs:344), `binding_status` в UI/API.
