# Runtime Containment Decision Proposal

Status: Fleet integration proposed, not implemented or live accepted. A user choice between a
per-agent container and delegated Linux cgroup was requested on 5 October 2026.
Recommended candidate: one container per agent in its existing workspace Compose
project. No installed runtime, image pin, mount or Compose group changes here.

## Standalone Base Primitive: 5 October 2026

Base [e6dcb3ca2c3cb4e2563f7bc13b443822ead3df10](https://github.com/FerrPOINT/services-base/commit/e6dcb3ca2c3cb4e2563f7bc13b443822ead3df10)
implements the host-only original-ID utility, not Fleet lifecycle. Its
[v1 contract](https://github.com/FerrPOINT/services-base/blob/e6dcb3ca2c3cb4e2563f7bc13b443822ead3df10/docs/platform/RUNTIME_BOUNDARY.md)
specifies trusted controller policy, sealed real inventory, private Linux PID
namespace, no restart, one durable stop claim and fresh original-ID readback.
Foreign SQLite tables/views/incompatible schema are denied without writes; this
is a separate controller journal, never Hermes SessionDB. Policy/labels cannot
substitute for controller ownership, filesystem guards or authorization.

Final Base native QA `sdlc-qa-runtime-boundary-cb38ff85e243` passes on Linux
kernel6.6.87.2/Engine29.8.1: three distinct namespaces, real descendant/setsid/
TERM-ignore/continuous-fork processes, one actual kill, protected sibling and
old-generation denial after manual restart. Two fresh host controllers recover
the original command after forced controller death before/after effect. On
Windows this host fault is TerminateProcess, not Linux-controller SIGKILL.
18 FakeEngine units are separate evidence. Exact source/report hashes and own
cleanup/independent empty ps are in the
[Base source ledger](https://github.com/FerrPOINT/services-base/blob/e6dcb3ca2c3cb4e2563f7bc13b443822ead3df10/docs/plans/base-pdlc-source-status.md).

The primitive does not create/start boundaries, prove loaded configuration or
integrate Fleet DB/drain/activation/admission. Docker Engine ID/kernel/version
are not host boot identity. Exclusive controller lifecycle and never restarting
the same container ID are mandatory; Docker kill provides no atomic StartedAt
CAS against privileged external lifecycle mutation. No accepted runtime or
dependency pin is changed; safe Fleet descendant stop remains unverified.

## Evidence And Required Boundary

### Pre-Exec Base Release Candidate: 6 October 2026

[Base PR144](https://github.com/FerrPOINT/services-base/pull/144), exact head
`dd2d0755266ef9081528702767006ba08e32d841`, isolates the host utility from
auth delegation/plugins/migrations on main. Its
[contract](https://github.com/FerrPOINT/services-base/blob/dd2d0755266ef9081528702767006ba08e32d841/docs/platform/RUNTIME_BOUNDARY.md)
now includes private pre-exec registration and one committed-claim Compose
start. Unknown start ACK/controller death holds the original resource without
adopting a fresh PID or issuing another start. Single-service specs prohibit
lifecycle hooks/build/external dependencies; original never-started state is
rechecked before the claim. Complete expected first-start inventory is sealed
in advance, not accepted from arbitrary observed post-start configuration.

The [release packet](https://github.com/FerrPOINT/services-base/blob/dd2d0755266ef9081528702767006ba08e32d841/docs/plans/runtime-boundary-bootstrap.md)
records actual four-resource registration/start/controller-crash acceptance,
separate process-tree stop acceptance,44 Linux boundary/bootstrap tests and
main-based Rust/frontend gates. All nine checks in
[exact-head CI](https://github.com/FerrPOINT/services-base/actions/runs/37376418561)
pass; PR144 is ready for review/CLEAN with no review threads. It is not
human-approved, merged or installed. This is offline namespace evidence, not an
online Hermes/model, task admission or SDLC receipt.

Fleet does not yet consume this registry: its DB binding before start,
trusted host transport, immutable effective configuration/boot generation,
atomic drain/receipt/activation/rollback and unknown-start operator recovery
remain required. Existing SDK/runtime pins and accepted Compose are unchanged.

The current `runtime/process_stop` kills and waits for the owned gateway child.
It cannot prove descendant termination. Process-group signals, process snapshots,
run terminal status and an HTTP stop ACK do not fix that gap: a child can daemonize
or call setsid, and processes can fork while a snapshot is being read. Untracked
PIDs remain denied, not adopted or signalled by numeric PID alone.

The target is an enforceable generation-specific resource boundary established
before model/tool execution. It must survive Fleet death and be verifiably empty
before replacement, configuration activation, port/workspace reuse or lease
handover. A lost stop response retains the original boundary ID and drain; it
never creates a replacement boundary as an implicit retry.

## Recommended Container Candidate

- Base owns common structured Compose rendering, container identity/readback and
  boundary stop utilities. Fleet owns agent/runtime/config lifecycle and admission.
  No scheduler, task requirements or workflow transitions move into Base.
- Permanent projects remain only `sdlc1`, `sdlc2`, `sdlc-common`. Agent services
  belong to their existing workspace project, never a new permanent project per
  agent. QA uses its own labelled `sdlc-qa-*` project with exact finally cleanup.
- A trusted controller selects the fixed project/service, exact immutable image,
  validated mounts, port and network policy. User/agent requests cannot provide
  arbitrary Compose, Dockerfile, shell, image tags, host paths or Docker options.
- Containers have private PID namespaces, non-root runtime users, dropped
  capabilities, no-new-privileges, a read-only root and explicit writable state/
  workspace mounts. No host PID/cgroup namespace, Docker socket, privileged mode,
  controller credentials or sibling agent directories are visible to Hermes.
- Preserve logical `agentN/runtime`, `config`, `workspace`, `logs` ownership.
  Separate immutable effective config/SOUL/skills mounts from writable runtime
  state. Seal actual mount/image/argv/env/config inventory; a filesystem hash or
  healthy listener alone is not proof of the loaded generation.
- Persist original container ID, daemon/boot identity, project/service labels,
  concrete agent and config revision, boundary generation and normalized inventory
  fingerprint before execution. A reused service name or image tag is not identity.
- Creation uses real Compose, never docker run/create or forged labels. Stop
  may use one exact-ID kill of the existing registered generation via the trusted
  host utility, followed by original-ID readback; a reused service name cannot
  select its target. Never down the full accepted workspace to stop one agent.
  Preserve snapshots, volumes, secrets, existing image IDs and unrelated resources.

## Stop And Recovery Contract

1. Acquire Fleet lifecycle/config drain and validate the persisted original
   boundary against fresh trusted runtime-host readback. No name-based adoption.
2. Submit one operation scoped to that exact generation. A stale identity, reused
   ID, changed daemon, unresolved response or unavailable host stays held.
3. Require authoritative stopped/empty-boundary evidence under the sealed private
   PID namespace/security policy. Container wait/exit code alone is insufficient
   unless the host contract guarantees and verifies namespace/cgroup quiescence.
   The production host receipt must bind that proof to the original generation.
4. Commit stop evidence and runtime metadata atomically. Only then release drain,
   references, ports and workspace capacity. Failed DB commit retains replayable
   original proof; retries read back the same operation instead of guessing.
5. On Fleet restart, reconcile original IDs and generations through readback.
   A live or unknown boundary is not replaced. Persisted metadata is not evidence
   of process ownership or termination by itself.

Runtime stop differs from run stop. Terminating a whole agent may interrupt its
active sessions and requires the existing explicit runtime action/audit. A native
run ACK/terminal receipt never implicitly permits runtime replacement. External
Forge/remote-tool jobs need their own fenced cancellation/readback receipts;
killing a container does not prove a remote side effect has stopped.

## Alternative: Delegated Linux Cgroup

A preconfigured, controller-owned cgroup v2 subtree can provide kernel-backed
empty-boundary readback and cgroup.kill. It requires atomic pre-exec placement,
protected generation ownership, no writable ancestor/migration paths for agents,
bounded kill/readback and restart reconciliation. A writable fake directory, PID
list, process group or caller-provided populated=0 is not proof. Windows and
nondelegated/read-only container environments must fail closed without fallback.
No host-wide cgroup remount, privileged helper or shared subtree mutation is
authorized by this proposal.

## Acceptance Before Enablement

Host assumptions must be verified against the primary documentation:
[Docker daemon access and container security](https://docs.docker.com/engine/security/),
[Linux PID namespace lifecycle](https://man7.org/linux/man-pages/man7/pid_namespaces.7.html)
and [cgroup v2 delegation, events and kill interfaces](https://docs.kernel.org/admin-guide/cgroup-v2.html).
Those interfaces inform this proposal; documentation is not a host-specific
quiescence receipt or acceptance evidence.

- Real parent, child, grandchild, double-fork/setsid and TERM-ignoring processes;
  continuous forking cannot escape. Stop leaves no process or new local effect.
- Two agents in distinct boundaries: stopping one does not affect the other.
- Crash before/after spawn, boundary persistence, stop and DB commit; original
  readback never duplicates a runtime or releases unknown capacity.
- Wrong project/service/daemon/image/mount/config/generation, reused name/PID,
  foreign marker, stale proof and untracked runtime are denied before effects.
- Config activation/rollback and task handover wait for actual quiescence;
  model/secret readiness, workflow admission and external-job receipts remain
  independent prerequisites.
- Verify exact host/SDK/API/source/image heads, authorization/redaction and
  adversarial Linux acceptance in owned Compose. No fixture is promoted into a
  kernel or installed-runtime receipt.

Until implementation and this acceptance pass, safe descendants, loaded
configuration generation and SDLC runtime admission remain open in
[the gap register](../GAP_REGISTER.md).
