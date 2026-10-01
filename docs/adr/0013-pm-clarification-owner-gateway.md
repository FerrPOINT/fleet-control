# Owner Gateway For PM Clarification

## Status

Accepted for the chat/PM slice on 2026-10-01. Gateway and chat controllers implemented;
runtime continuation integration is not yet accepted.

## Context

Fleet and Tracker have distinct local user UUIDs and different runtime/business authority.
Parsing assistant prose or copying requirements into Fleet would create competing state.
Operator access to troubleshoot a chat must not become permission to consent for its owner.

## Decision

Bind a chat explicitly to immutable Tracker instance/task/project/root identity, concrete
agent and verified central owner subject. Tracker owns structured questions, answers,
immutable requirements and exact-revision confirmation. Fleet forwards owner commands to
one operator-configured Tracker origin with the original verified bearer; Tracker checks
human session, membership and owner independently. Local fallback cannot authorize consent.
Fleet audit/event and binding commit once transactionally. Do not infer identity from keys.

## Consequences

Tracker failures disable dependent actions while Fleet transcript remains accessible.
Answer persistence is separate from PM delivery. Pending/unknown commands retain their
idempotency key; no blind new runtime dispatch. Task-bound generic prompts/steer stay closed
until assignment/checkpoint/rebind and actual runtime readback are integrated.
Separate contracts and live cross-service acceptance are required before rollout.

## Alternatives

- Share local UUID/email: rejected because they do not prove cross-service identity.
- Mirror authoritative requirements into Fleet: rejected because it permits divergent gates.
- Let operator confirm for owner: rejected because infrastructure access is not business consent.
- Parse questions from Markdown: rejected because versions/options/checkpoints need structured IDs.
