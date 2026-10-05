# Runtime Launch Journal V1

Status: internal Fleet source contract. This is a native gateway launch journal,
not a runtime-host isolation, loaded-configuration or SDLC admission receipt.
It adds no public route, runtime credential or automatic task dispatch.

## Ownership And Identity

Fleet owns PostgreSQL `runtime_launches` (additive migration `000017`). Before
native Hermes or Java spawn, the supervisor commits one immutable binding:
launch UUID, concrete agent, fresh controller-instance UUID, runtime kind,
paths/port, configuration phase/revision/snapshot SHA256 and explicit-command SHA256.
The controller UUID is not an OS boot ID or a surviving runtime-host identity.
No raw argv, environment secret or configuration snapshot enters this table.
Java chat/control remains phase 2.

The transaction locks the agent, runtime metadata and configuration head. It verifies current
paths/kind/port, stopped ownership and the exact persisted snapshot, not caller assertions. One
partial unique index allows at most one `claimed`/`gateway_started` launch per
agent across Fleet replicas. Local lifecycle locks remain necessary for owned
child custody; they are not the cross-process authority.

Regular launches pin the effective active revision (nullable only for explicitly
unversioned legacy configuration). Activation launches pin the desired, claimed
activating revision while drained. Rollback pins the prior effective revision
while drained. New drafts do not relabel the already effective configuration.
This records intended source identity, not proof of bytes loaded by Hermes.

## Execution And Observation

1. Validate/read the source binding and commit `claimed` before `Command::spawn`.
2. A definitive spawn error records `spawn_failed` for that original binding.
3. Actual child creation records `gateway_started` with its positive PID, atomically
   with runtime metadata. On failed DB acknowledgement, retain the owned child and
   original launch. A committed ACK can be reconciled by original binding readback
   only while this controller retains that exact child handle/PID.
4. Fresh health and a new Hermes submission require the acknowledged original
   journal plus the same controller's retained child handle/PID. No numeric PID
   adoption or a healthy HTTP listener supplies missing acknowledgement.
5. An owned child wait/kill+wait may record `gateway_exited` with the original PID.
   Persist this observation and cleared runtime metadata atomically before
   discarding the child identity. DB failure preserves the original already-waited
   handle for retry. Repeating the same observation is read-only and cannot reset
   newer runtime metadata; another binding/PID/state conflicts.

Runtime metadata updates lock the same agent row and recheck the outstanding
launch. A late stopped/failed observation cannot clear a newly claimed/started
gateway; a health update cannot substitute an older PID. The journal check does
not grant ownership from a PID: actual dispatch still requires retained custody
and the immutable launch ID. Once a gateway is closed, descendant safety remains
the separate boundary gate below.

An unresolved launch blocks replacement, even if `agent_runtime` still says
Ready because the controller died before the status write. With no owned child,
stop cannot assert success or release the claim. Configuration activation checks
the journal before preparing backups or writing files. Infrastructure identity
edits cannot mutate an agent pinned by an outstanding launch.

Unknown acceptance does not expire into a retry permit. Reusing the original
launch key is not a second spawn permit; a new controller cannot adopt it.
History and binding are immutable (including truncate denial), including definite failures. Nonempty history
prevents migration downgrade; clean-schema down/reapply is allowed.

## Dispatch Generation

New free-chat dispatch intents pin `fleet_launch: {version: 1, launch_id}` in
the private, immutable capability snapshot. Fleet writes this field after
sanitizing upstream capabilities; Hermes cannot supply it. A null launch ID
means an explicitly unjournaled legacy runtime, not managed authority. Historical
intents without this field retain only that legacy meaning. The closed binding
rejects unknown fields, nil/noncanonical IDs and unknown versions.

Preparation and consumption of the one-shot submission permit verify this ID
against the outstanding started journal under the same agent database lock.
The actual HTTP submission rechecks the exact original generation and retained
child under the local lifecycle lock. A replaced or exited gateway, or a new
managed launch for a legacy intent, denies submission without translating the
old intent to a new generation. GET-only recovery of a previously submitted run
remains governed by its original native recovery-store identity.

This fences Fleet-native generations only; it does not attest the HTTP server's
loaded configuration, host boot, descendant tree or task assignment authority.
Database metadata failure cannot create a new submission permit.

## Deliberate Limits And Next Gate

`gateway_exited` means only that Fleet positively waited for its original child.
It does not prove descendant, container namespace, cgroup or remote-job cessation.
Legacy native restart semantics remain, not a certified safe-stop path. The
offline Base host registry in PR144 is not consumed by this native journal.

Before isolated production execution, bind this logical launch to the immutable
host resource/boot generation before start and commit verified boundary-empty
receipts before activation/replacement/capacity reuse. Immutable config mounts,
loaded-generation attestation, online Hermes transport, crash/operator recovery,
task leases/fencing/first-step and scoped machine credentials remain mandatory.
Unknown launches stay held until such reconciliation can prove their outcome;
no force-release, reset or PID-signal repair API is introduced.

## Verification

PostgreSQL/controller tests exercise concurrent replicas, crash-window holds,
stale path/port/revision/snapshot denial, correct activation/rollback binding,
missing DB ACK, immutable PID/owner/history and definitive spawn failure. Dispatch
tests cover original generation, replacement denial before permit consumption,
legacy-to-managed denial, closed binding validation and real-child replacement.
Linux owned-child tests are gateway lifecycle evidence only. The migration test
requires its own empty disposable database and checks legacy row preservation,
empty down/up, identity constraints and nonempty-history downgrade denial.
Exact command/log/source evidence belongs in
[the verification ledger](../CHAT_CLARIFICATION_VERIFICATION.md), not this protocol.
