# Implementation Plan

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
