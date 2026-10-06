# ADR 0031: Private Bounded Container Log Readback

## Status

Implemented in the integration candidate; broad Linux/PostgreSQL checks pass.
Actual Docker evidence and remaining ingestion requirements are recorded in
[verification](../CHAT_CLARIFICATION_VERIFICATION.md). Not installed or a claim
of complete SDLC readiness.

## Context

Native child pipe capture does not collect output from a separately managed
Docker namespace. Accepting a container name supplied by an agent or exposing
raw Docker output through the public logs API would bypass original runtime
custody and may disclose credentials. A read must also bound memory and time,
including when stderr fills while stdout is being read.

## Decision

Use the existing controller-private Base utility, not a new HTTP service.
The additive `logs` operation requires the exact original registration,
Compose policy, private journal and, where applicable, immutable mount mapping.
Only acknowledged running or exited originals qualify. Base verifies the
original receipt before and after reading; drift discards both streams.

The Linux Docker CLI reader uses a fixed full container ID and tail of 1-200
lines, drains stdout/stderr concurrently and limits their combined output to
32 KiB and five seconds. Timeout, overflow or nonzero exit is a read failure,
not partial success. Its reader process is killed/reaped on failure. The private
response encodes each binary stream separately; no UTF-8 assumption is needed.

Fleet's typed client checks captured source hashes, mapped registration,
closed response fields, original receipt, base64 and combined byte limit.
Raw output deliberately implements neither `Debug` nor a public serialized DTO.
No public API, database schema, deployment pin or business scheduler changes.

## Consequences

This is a diagnostic transport foundation. It is not connected to production
health polling or `agent_logs` persistence. Such integration must first prove
current launch custody and redact the exact resolved per-launch credentials
before storage, API, audit or SSE. Durable generation-bound cursor/deduplication,
rotation/overflow handling and restart recovery require additional implementation
and actual acceptance. A bounded tail cannot certify complete log history.

The private utility and Rust client need compatible source versions; an older
Base rejecting `logs` must fail closed, without native or arbitrary-ID fallback.
Windows pipe readback is deliberately unsupported; the trusted controller runs
on Linux. Agents never receive the Docker socket or raw controller response.

## Alternatives

- Inherit native child capture: cannot read separate Docker namespace output.
- Give agents Docker access or expose arbitrary container IDs: rejects custody
  and isolation requirements.
- Unbounded `docker logs --follow` and sequential pipes: cannot bound memory,
  lifetime or pipe deadlock.
- Persist repeated tails as a complete history: duplicates entries and silently
  loses gaps; durable ingestion needs its own provenance and replay contract.
