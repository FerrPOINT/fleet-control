# Java Agent Adapter Contract

The Docker config18 [P2 successor](../CONTAINER_ACTIVATION_PREFLIGHT_FIX.md) changes
no Java/process validation, provisioning, lifecycle or db-only readiness behavior.

Docker config activation unit18 is Hermes-only. Java's process lifecycle and
unsupported config activation behavior are unchanged; no Docker or process
fallback is introduced. See [bounded unit18 scope](../CONTAINER_ACTIVATION_RELEASE.md).
The [standalone integration](../CONTAINER_ACTIVATION_INTEGRATION.md) runs periodic
reconcile per agent; Java's command, provisioning and db-only readiness contracts
remain unchanged. Docker activation and recovery authority do not apply to Java.

[Automatic Docker preparation unit17](../AUTOMATIC_CONTAINER_PREPARATION_RELEASE.md)
is Hermes-only and does not change Java provisioning, process paths or readiness.

[Mapped controller recovery unit16](../MAPPED_CONTROLLER_RECOVERY_RELEASE.md) is
Hermes-only; it does not change Java process lifecycle or readiness.
The canonical-source/hash and recovery-loop correctness follow-up is also
Hermes-only and changes no Java lease, path, process or readiness behavior.

The optional [Docker lifecycle source release](../DOCKER_LIFECYCLE_RELEASE.md)
applies to Hermes only. Java continues using its externally provisioned jar,
process supervisor and db-only readiness contract.

## Hermes Journal Release Unit12

The isolated Hermes journal release does not change the Java Agent adapter,
readiness, launch or control contract. Its permit, atomic ACK and GET-only recovery
are Hermes-specific and do not establish Java Agent or SDLC runtime acceptance.

Java Agent is modeled alongside Hermes. Existing local jar provisioning layout,
start/stop/restart and readiness checks are implemented and retained. Chat/control,
configuration activation and automatic SDLC admission remain phase 2.

Current launch: `java -jar agentN/runtime/backend.jar --spring.profiles.active=noop`.
The jar is supplied externally; a missing jar is a validation error. Readiness
uses `/actuator/health/readiness` with the Java service's db-only readiness contract.

Native stdout/stderr uses the shared redacted process-log repository; each write
acknowledges its own persisted row, independently of other writers. See
[Logging Standards](../LOGGING_STANDARDS.md). No Java chat capability is added.

Expected launch inputs:

- `AGENT_SERVER_PORT`
- `SPRING_CONFIG_ADDITIONAL_LOCATION`
- workspace cwd

Expected endpoints:

- `GET /actuator/health/readiness` (current lifecycle)
- `POST /api/v1/agent/chat/stream`
- `GET/POST /api/v2/sessions`
- `GET /v1/capabilities`

Unimplemented chat/control/stream/approval operations return typed
`not_implemented`. The generic phase-2 label must not disable the working jar
lifecycle. Java is not ready for automatic SDLC until required capabilities and
workflow bindings have been verified.
