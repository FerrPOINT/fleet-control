# Resilience

Required resilience behavior:

- Provisioning is idempotent and marker-guarded.
- Session and message creation are idempotent.
- Runtime desired state is stored and reconciled after Fleet restarts.
- Process death updates runtime status and preserves logs.
- Archive stops agents but does not delete folders.
- Java Agent not-yet-runnable operations fail with typed, visible errors.
- UI exposes loading, empty, error, access denied and disabled states.

Failure handling:

- Failed runtime send should not duplicate the prompt on browser retry with the
  same idempotency key.
- Stuck deployment jobs can be inspected and cancelled.
- Audit entries must survive frontend failures.
