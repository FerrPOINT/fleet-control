# Agent Runtime Contract

Hermes steer/stop use the original accepted dispatch context, fresh capabilities
and pinned native status under the [control profile](HERMES_RUN_CONTROL_V1.md).
Only an exact bounded ACK is accepted. Guidance never resets local run state;
interrupt ACK can mark stopping, not terminal success or capacity release.
Free-chat controls now reserve a durable actor/key/payload-hash command before
native IO and claim its single-send permit before POST. Public stop/steer also
require independently verified human-session proof before session/run lookup;
an authenticated machine or admin role alone does not authorize these routes.
Readback and replay never
resend submitted/uncertain controls. ACK persistence, stopping state, audit and
Fleet events commit atomically. Independent terminal mirror proof can retire an
unresolved command as `terminal_observed`, not fabricate its ACK. Task-bound
control admission remains a separate gate. Legacy run-wide approval is denied inside the adapter as well
as HTTP; exact human request decisions remain the only approval path.

Hermes stream parsing has a separate bounded
[consumer profile](HERMES_EVENT_STREAM_V1.md). Failure retains accepted identity
and capacity; partial EOF does not dispatch a terminal event. This adds no Java
capability, cross-service lease or stage receipt. Decoder limits are not safe-stop
or native durable event-replay guarantees.

Prepared initial-delivery recovery is separate from accepted/unknown readback.
It can consume the original free-chat permit once after current identity/drain/
protocol/deadline checks; it cannot repeat submitted, failed or legacy work.
No task/PM admission or Java capability is added; see
[ADR 0020](../adr/0020-prepared-dispatch-restart-recovery.md).

Hermes original-key recovery is an explicit source-pinned
[extension](HERMES_RECOVERY_V1.md), not a common runtime capability. Original
scope/store facts are frozen before admission; non-dispatch lookup can restore
only that original run ID within the DB-clock horizon. Native status remains
required for session pin/terminal outcome. No legacy backfill, reset/negative
lookup redispatch, task/PM authority or Java behavior is introduced.

Hermes configuration activation persists an exclusive local backup journal before
stop/file effects. Only the exact journal can be acknowledged after the database
activation result commits and rollback/application is verified. Existing, partial
or changed journals hold further activation; no automatic crash takeover follows.
Journal content is sensitive local recovery data, never a public runtime receipt.
Linux apply/rollback persist parent directory entries after file rename/unlink,
including every new skill-directory ancestor. A failed barrier holds journal/drain
instead of acknowledging effective state. This does not certify Windows directory
durability, host power-loss behavior or native loaded configuration.
See [operations](../OPERATIONS.md#sdlc-foundation-recovery).

Every runtime adapter must provide:

- runtime kind
- capability metadata
- provisioning behavior
- command preview
- start, stop, restart and health behavior
- session metadata mapping
- log capture policy
- secret redaction policy

Configuration snapshots freeze their renderer semantics: absent/1 retains the
legacy output; server-created Hermes revisions use 2 and Java uses 1. V2 seals
native listener/HOME/key/CORS and rejects alternative API aliases or malformed
platform objects. Explicit draft/validate/drain/activation is required to upgrade
an existing agent. File/hash verification alone does not attest loaded runtime
settings. See [ADR 0017](../adr/0017-versioned-native-hermes-renderer.md).

Hermes free-chat dispatch reserves an immutable request journal and consumes a
single-send permit before HTTP. Verified ACK progress commits atomically with
run/message/outbox. Unknown acceptance holds capacity, without POST replay.
Only journal-backed original-context ACKs can recover effective-session GET;
legacy history is preserved without retrospective credential attestation.
Known pinned free-chat runs also recover by authenticated GET, without another
POST or SSE consumer. Validated terminal state, optional assistant mirror and
prompt delivery commit atomically with durable events; exact replay changes no
timestamps/cursor. Late progress cannot reopen terminal state. Missing/invalid
proof retains capacity. See [ADR 0018](../adr/0018-atomic-terminal-pinned-recovery.md).
This adds no task/PM admission authority or Java capability. See the
[journal model](../DATA_MODEL.md#hermes-dispatch-journal).

Supervisor stop acknowledges a tracked process only after bounded kill/wait or
confirmed natural exit. Kill/wait failure preserves the tracked handle and does
not publish stopped/PID-null state. Untracked starting/running/degraded state,
a recorded PID or desired running state returns dependency-unavailable; no
PID-only signal or guessed success is performed. Restart fails at that same gate.
This is parent-process termination, not descendant/container quiescence or a
Tracker assignment-release receipt.

Within one supervisor, a per-agent lifecycle mutex serializes start/stop/restart,
health readback and the entire configuration apply/rollback phase. Each operation
reloads current agent state; public mutations recheck drain after acquiring the
mutex. Activation uses locked internal helpers so restarting a revised runtime
does not recursively acquire the mutex. Failed-start cleanup verifies the owned
Child PID before kill/wait. Cross-instance ownership fencing is still required.
Command validation precedes publishing starting state: a missing Java jar or
invalid Hermes token configuration is a known pre-spawn failure, not an unknown
process that may be replaced or marked running.
An HTTP probe failure degrades an unconfirmed active runtime without clearing its
PID or desired state; an old request snapshot cannot overwrite a newer stop.
Readiness uses an absolute 60-second deadline including probes and sleeps, not
poll-count accounting. Individual HTTP probes are bounded to three seconds.

All adapters use the common agent layout:

```text
agentN/runtime
agentN/config
agentN/workspace
agentN/logs
```

SDLC configuration observation is a separate, read-only owner interface;
see [the API contract](../API.md#sdlc-configuration-observation).
Fresh Base PAT authorization and concrete-agent allowlisting do not authorize
dispatch. Managed-file verification, loaded-runtime admission and terminal
execution evidence are distinct. The current observation is not runtime-ready.
Effective heads and exact agent/revision lookups are independent of paged history.
Pinned package reads have fixed IO/time bounds and no HEAD/network fallback;
neither a successful read nor an older validated draft authorizes activation or
assignment dispatch.

Base-marked configuration revisions also freeze a separate
[Workflow binding](SDLC_WORKFLOW_BINDING_V1.md). Fresh owner ID/name/profile
verification precedes supervisor side effects. An unavailable preflight fails
the revision without retaining drain; an unknown apply/rollback keeps drain.

Terminal runtime evidence must identify the accepted run; EOF, nested child
completion and a requested stop are not completion. The implemented Hermes
adapter validates exact terminal names, native success flags and bounded status
readback before ending a run. Runtime completion is separate from process-tree
quiescence and owner-authorized business transitions. See the
[Hermes wire contract](HERMES_ADAPTER_CONTRACT.md); this does not grant phase-2
Java chat/control capabilities.

Hermes known-ID recovery can also restore the current pending native approval
with its original accepted identity. It commits request/Waiting atomically and
keeps capacity on invalid readback. This does not resume PM, replay a historical
approval queue, authorize wider grants or attest safe OS-descendant termination.
