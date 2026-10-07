# Fleet Control Documentation

Fleet Control is the runtime fleet layer in the SDLC suite. It complements
`task-tracker`, `project-workflow`, `wiki`, `CI-CD`, `hermes` and `java-agent`.

## Product And State

- [Chat clarification implementation plan](CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md)
- [Chat clarification verification](CHAT_CLARIFICATION_VERIFICATION.md)
- [Chats/PM consumer handoff](CHATS_PM_CONSUMER_HANDOFF_20261006.md)
- [Chats/PM preview evidence, not live acceptance](assets/design/chats-pm-consumer/manifest.json)
- [Chat clarification contract](contracts/CHAT_CLARIFICATION_CONTRACT.md)
- [PM clarification owner gateway ADR](adr/0013-pm-clarification-owner-gateway.md)
- [Persisted PM credential preparation ADR](adr/0014-persisted-pm-credential-preparation.md)
- [Accepted run session readback ADR](adr/0015-accepted-run-session-readback.md)
- [Hermes original request journal ADR](adr/0016-hermes-original-request-journal.md)
- [Versioned native Hermes renderer ADR](adr/0017-versioned-native-hermes-renderer.md)
- [Atomic terminal and pinned recovery ADR](adr/0018-atomic-terminal-pinned-recovery.md)
- [Native original-key recovery ADR](adr/0019-native-original-key-recovery.md)
- [Prepared dispatch restart recovery ADR](adr/0020-prepared-dispatch-restart-recovery.md)
- [Hermes recovery extension v1](contracts/HERMES_RECOVERY_V1.md)
- [Bounded Hermes event-stream profile v1](contracts/HERMES_EVENT_STREAM_V1.md)
- [Hermes run-control consumer profile v1](contracts/HERMES_RUN_CONTROL_V1.md)
- [Hermes control-outcome producer v1](contracts/HERMES_CONTROL_OUTCOME_V1.md)
- [ADR: native control outcome witness](adr/0024-native-control-outcome-witness.md)
- [ADR: original approval outcome journal](adr/0025-original-approval-outcome-journal.md)
- [Pre-spawn runtime launch journal](contracts/RUNTIME_LAUNCH_JOURNAL_V1.md)
- [Fenced controller recovery storage](contracts/CONTROLLER_RECOVERY_V1.md)
- [ADR: fenced controller recovery epochs](adr/0032-fenced-controller-recovery-epochs.md)
- [Private container lifecycle client](contracts/CONTAINER_CONTROL_V1.md)
- [ADR: pre-spawn runtime launch binding](adr/0026-pre-spawn-runtime-launch-journal.md)
- [Durable runtime command ADR](adr/0021-durable-runtime-control-commands.md)
- [Current native approval snapshot ADR](adr/0022-current-native-approval-snapshot.md)
- [Logical journal progress time ADR](adr/0023-logical-journal-progress-time.md)
- [Runtime command UI fixture manifest](assets/design/runtime-controls/manifest.json)
- [Actual credential interoperability harness](../scripts/pm_credentials_live/README.md)
- [Native Hermes protocol acceptance harness](../scripts/hermes_protocol_live/README.md)
- [Managed native supervisor acceptance harness](../scripts/native_supervisor_live/README.md)
- [Actual Docker supervisor and loaded-config acceptance](CONTAINER_SUPERVISOR_ACCEPTANCE.md)
- [SDLC_IMPLEMENTATION.md](SDLC_IMPLEMENTATION.md) — реализация нового SDLC, границы и оставшаяся приёмка.

- [TZ.md](TZ.md)
- [PRODUCT_REQUIREMENTS.md](PRODUCT_REQUIREMENTS.md)
- [CURRENT_STATE.md](CURRENT_STATE.md)
- [ROADMAP.md](ROADMAP.md)
- [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md)
- [GAP_REGISTER.md](GAP_REGISTER.md)
- [PRE_DEVELOPMENT_GATE.md](PRE_DEVELOPMENT_GATE.md)

## Architecture And Domain

- [ARCHITECTURE.md](ARCHITECTURE.md)
- [FRONTEND_ARCHITECTURE.md](FRONTEND_ARCHITECTURE.md)
- [DOMAIN_MODEL.md](DOMAIN_MODEL.md)
- [CHAT.md](CHAT.md) — текущий Chats scope и сохранённый аудит legacy-сессий;
  лиды и автономный SDLC не объявлены завершёнными.
- [DATA_MODEL.md](DATA_MODEL.md)
- [WORKFLOW.md](WORKFLOW.md)
- [RUNTIME.md](RUNTIME.md)
- [Runtime containment proposal](design/RUNTIME_CONTAINMENT_PROPOSAL.md) - proposed boundary, not implemented or accepted.
- [EVENTS.md](EVENTS.md)
- [ROUTING.md](ROUTING.md)
- [LIBRARIES.md](LIBRARIES.md)
- [SERVICES_BASE.md](SERVICES_BASE.md)
- [contracts](contracts)
- [SDLC Workflow binding](contracts/SDLC_WORKFLOW_BINDING_V1.md)
- [ADR.md](ADR.md)
- [Durable container pre-create fence](adr/0028-durable-container-precreate-fence.md)
- [Container configuration generation replacement](adr/0029-container-configuration-generation-replacement.md)
- [Generation-bound container endpoint](adr/0030-generation-bound-container-endpoint.md)
- [Private bounded container log readback](adr/0031-private-bounded-container-log-readback.md)

## API And Database

- [API.md](API.md)
- [API_STANDARDS.md](API_STANDARDS.md)
- [API_EDGE_CASES.md](API_EDGE_CASES.md)
- [API_VERSIONING.md](API_VERSIONING.md)
- [ERROR_HANDLING.md](ERROR_HANDLING.md)
- [MIGRATIONS.md](MIGRATIONS.md)
- [DATABASE_STANDARDS.md](DATABASE_STANDARDS.md)
- [DATABASE_INDEXES.md](DATABASE_INDEXES.md)

## Security And Operations

- [SECURITY.md](SECURITY.md)
- [THREAT_MODEL.md](THREAT_MODEL.md)
- [SYSTEM_ADMIN.md](SYSTEM_ADMIN.md)
- [ENV.md](ENV.md)
- [LOCAL_SETUP.md](LOCAL_SETUP.md)
- [DEPLOYMENT.md](DEPLOYMENT.md)
- [OPERATIONS.md](OPERATIONS.md)
- [OPS_RUNBOOK.md](OPS_RUNBOOK.md)
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
- [BACKUP_RESTORE.md](BACKUP_RESTORE.md)
- [MONITORING.md](MONITORING.md)
- [LOGGING_STANDARDS.md](LOGGING_STANDARDS.md)

## Quality And Delivery

- [TESTING.md](TESTING.md)
- [QUALITY_GATE.md](QUALITY_GATE.md)
- [TRACEABILITY.md](TRACEABILITY.md)
- [RISK_REGISTER.md](RISK_REGISTER.md)
- [PERFORMANCE.md](PERFORMANCE.md)
- [RESILIENCE.md](RESILIENCE.md)
- [UI_UX.md](UI_UX.md)
- [FRONTEND_STANDARDS.md](FRONTEND_STANDARDS.md)
- [CODE_STYLE.md](CODE_STYLE.md)
- [CODE_REVIEW.md](CODE_REVIEW.md)
- [RELEASE.md](RELEASE.md)
- [CI_CD.md](CI_CD.md)

## Evidence

- [Chat and clarification design proposal](design/CHAT_CLARIFICATION_PREVIEW.md)
- [PM Draft creation and recovery proposal](design/PM_DRAFT_CREATION_PREVIEW.md) (approval pending)
  is an isolated clickable proposal with fictional data, not an implemented
  Tracker clarification flow or an approved production design.
- [assets/screens/manifest.md](assets/screens/manifest.md) contains the
  generated route-to-screenshot matrix for 135 fixture screenshots at three
  viewports. It does not prove real runtime execution or Central Auth acceptance.
