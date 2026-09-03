# Ops Runbook

Common operations:

1. Check `/api/v1/health`.
2. Open `/dashboard` for fleet health.
3. Use `/deployments?tab=jobs` for stuck provision/update jobs.
4. Use `/logs?tab=process` for runtime stdout/stderr.
5. Use `/logs?tab=events` for control-plane state transitions.
6. Use `/logs?tab=audit` for who changed what.
7. Verify `agents_root` disk space before provisioning more agents.
8. Stop/archive an unhealthy agent before manual filesystem inspection.

Agent recovery:

- run health check
- inspect logs
- restart agent
- if restart fails, stop and reprovision only after checking marker ownership
- do not purge folders unless the explicit admin purge flow exists

Session recovery:

- inspect session runs
- retry message only with a new idempotency key when the previous send failed
- use handoff when ownership should move to another primary agent
