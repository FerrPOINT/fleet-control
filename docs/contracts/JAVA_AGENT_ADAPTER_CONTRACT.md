# Java Agent Adapter Contract

Hermes original-key recovery and atomic terminal persistence are Hermes-only.
This candidate adds no Java chat/control/readback capability or task/PM authority.
See the [release boundary](../plans/2026-10-09-hermes-recovery-release.md).

## Hermes Approval Recovery Boundary

Unit14 current exact approval recovery is Hermes-only. It adds no Java runtime
approval endpoint, generic approval fallback, lifecycle permission or readiness
claim. Java behavior and SDK pin are unchanged.

## Durable Runtime Controls Unit13

The new journal/native control protocol is Hermes-only. Java Agent chat/control
remains unavailable; no Java readiness, process lifecycle or SDK contract changes
are introduced or accepted by this source freeze.

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
