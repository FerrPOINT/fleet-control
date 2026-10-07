# Java Agent Adapter Contract

The Hermes [SDLC skill discovery policy](../RUNTIME.md#sdlc-skill-discovery-policy)
does not apply to Java configuration, skills or readiness. Java lifecycle and
capability-gated SDLC limitations are unchanged.

Hermes signed-v3 configuration recovery adds no Java configuration/recovery
capability. Java runtime lifecycle and phase-2 chat limits remain unchanged;
the new default-off flag skips Java agents and creates no journal for them.

The private [controller recovery storage](CONTROLLER_RECOVERY_V1.md) is limited
to the mapped container Hermes path. It does not adopt Java processes or change
their existing lifecycle/readiness contract. Java SDLC chat/control admission
remains capability-gated independently.
Migration000021's native delivery/ACK flow is also Hermes-only; it introduces no
Java recovery worker, lifecycle change, chat capability or installed opt-in.

The optional Base container-control consumer currently accepts only Hermes
bindings. It neither containerizes Java nor adds Java chat/control/SDLC
capabilities. Existing Java local jar lifecycle/readiness remains unchanged.
Java Docker lifecycle will need its own image/launch and acceptance contract;
Hermes original container receipts cannot authorize a Java launch. Start with
Docker configuration now rejects Java before files/process/DB effects with the
existing Unavailable503 error, not a new public501/not_implemented DTO. There is
no native fallback; native Java remains unchanged only without Docker mode.
The automatic generation preparation/restart path is likewise Hermes-only;
Java never obtains its image/process/config bindings or preparation receipt.
Named-volume protocol2/policy3 is also Hermes-only and does not grant Java
Docker, chat or SDLC capabilities; historical Java local lifecycle is unchanged.
The000018 pre-create reservation accepts only Hermes. It introduces no Java
container or SDLC capability; an unresolved Hermes reservation cannot be bypassed
by changing launch kind or requesting a native launch.

The original approval journal (000016) likewise adds no Java chat/control or
machine admission. Its claim requires an accepted concrete Hermes free chat;
Java lifecycle and typed unsupported execution remain unchanged.

The additive000015 original control-outcome journal is Hermes-only. It adds no
Java dispatch permit, recovery witness or chat/control capability; Java cannot
claim its context or finish an outcome. Existing Java lifecycle remains unchanged.

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
Hermes managed-file directory barriers do not enable Java configuration activation
or change Java lifecycle capabilities.

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

Hermes current-approval snapshot recovery also does not supply Java approval or
chat recovery capabilities. Java jar lifecycle remains unchanged; its chat and
control implementation still requires independent phase-2 acceptance.
