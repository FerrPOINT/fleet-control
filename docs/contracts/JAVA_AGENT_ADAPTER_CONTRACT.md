# Java Agent Adapter Contract

The [Hermes control profile](HERMES_RUN_CONTROL_V1.md) introduces no Java
chat/control capability. Java lifecycle remains available; its run steer/stop
remain typed phase-2 failures. The retired run-wide approval path is rejected
for every runtime; only separately supported exact request decisions apply.

Fleet's migration 000013 and command receipts apply only to accepted Hermes
free-chat runs. Java cannot reserve/claim these permits or obtain an ACK through
the Hermes control history. Its existing lifecycle and typed unsupported chat/
control behavior remain unchanged; an empty history is not a Java capability.

Hermes prepared-dispatch restart recovery is exclusive to its exact free-chat
journals and native protocol. It adds no Java admission, chat/control, config
activation or recovery operations; existing Java lifecycle remains unchanged.

The [Hermes native original-key extension](HERMES_RECOVERY_V1.md) does not
extend Java lifecycle/chat capabilities or bypass its phase-2 execution gate.

Hermes-only atomic terminal/pinned recovery does not extend Java chat/control
capabilities. Existing Java lifecycle and phase-2 boundaries remain unchanged;
see [the common runtime contract](AGENT_RUNTIME_CONTRACT.md).

Existing Java process lifecycle shares bounded tracked-process stop/wait. An
untracked active runtime is not labelled stopped or restarted without proof;
this does not implement Java chat/control or automatic SDLC admission.

The actuator readiness request and body share a three-second deadline; JSON is
limited to 16 KiB with identity encoding. Hung headers/bodies cannot indefinitely
hold the agent lifecycle mutex. Startup has a 60-second wall-clock readiness
deadline. Health observation preserves desired state and unconfirmed recorded PID.

The protected activation journal introduced for Hermes does not add Java
configuration activation or chat admission. Java keeps its existing typed
unsupported configuration behavior; no Hermes journal is written for Java.
The Hermes database dispatch journal and single-send permit also confer no Java
chat/control capability; Java cannot use that free-chat path.

Java Agent is modeled alongside Hermes. Existing local jar provisioning layout,
start/stop/restart and readiness checks are implemented and retained. Chat/control,
configuration activation and automatic SDLC admission remain phase 2.

Current launch: `java -jar agentN/runtime/backend.jar --spring.profiles.active=noop`.
The jar is supplied externally; a missing jar is a validation error. Readiness
uses `/actuator/health/readiness` with the Java service's db-only readiness contract.

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

The pinned Base/Hermes configuration observation does not admit Java. A Java
lifecycle health result cannot supply Hermes package/filesystem proof or grant
automatic assignment rights; the existing lifecycle remains unchanged.
The new [Base Workflow mapping](SDLC_WORKFLOW_BINDING_V1.md) is metadata for
Hermes package revisions, not a Java chat/capability implementation or a reason
to disable its existing jar lifecycle.

Hermes terminal-event/EOF readback checks described in the
[runtime contract](AGENT_RUNTIME_CONTRACT.md) do not supply Java session identity,
completion or safe-stop evidence. The Java lifecycle and phase-2 boundary remain
unchanged; no Hermes success alias is a Java capability.
