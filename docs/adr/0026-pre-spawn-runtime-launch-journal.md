# ADR 0026: Pre-Spawn Runtime Launch Journal

## Status

Implemented in the integration source; release and runtime-host integration
remain pending. See [the internal contract](../contracts/RUNTIME_LAUNCH_JOURNAL_V1.md).

## Context

A per-agent Tokio mutex protects only one Fleet process. Ready/starting/PID
metadata cannot distinguish a crash before spawn from a crash after spawn or
prevent another controller starting the same agent in the intervening window.
Configuration activation and HTTP readiness must not erase that uncertainty.

## Decision

Persist an immutable launch/config/command binding in PostgreSQL before spawn,
under agent/config-head locks and a unique outstanding-launch constraint.
Preserve original child custody across failed acknowledgements. Require the
original acknowledged binding before new dispatch. Record gateway observations
explicitly, without claiming namespace quiescence, loaded config or admission.

Pin this launch ID in each new private dispatch capability snapshot. Check it
when preparing/claiming the dispatch and again before HTTP POST under the
lifecycle lock; never relabel a prepared legacy or older-generation intent.
Commit gateway observations with runtime metadata atomically. Read back a lost
ACK only with the retained original child, and preserve already-waited custody
when a terminal metadata commit fails.

Activation and rollback use different source revisions. History cannot be
rewritten/deleted and a nonempty journal blocks downgrade. Unknown launches have
no expiry/reset/retry permit or numeric-PID adoption.

## Consequences

Cross-controller duplicate launches and metadata-reset recovery are prevented.
An infrastructure failure can hold an agent until reviewed reconciliation;
availability does not override single-execution safety. Legacy sessions remain
legacy and are not silently given a host-generation receipt. The kernel boundary,
immutable loaded config, boot identity and fenced workflow authority are separate
mandatory work, not inferred from this journal.

## Alternatives

- Local mutex alone: cannot serialize different Fleet processes or survive death.
- Ready/starting status CAS alone: loses immutable source identity and audit/history.
- Adopt a current PID or healthy listener: cannot identify the original generation.
- Expire a claim and spawn again: can duplicate an already accepted execution.
- Treat gateway exit as container-empty: cannot account for detached descendants.
