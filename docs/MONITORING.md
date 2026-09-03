# Monitoring

Monitor these signals:

- API health endpoint.
- Database connectivity and migration status.
- Redis connectivity if SSE/cache features are enabled.
- Running runtime processes and desired-state reconciliation.
- Hermes health per agent.
- Provision/update job state and age.
- Failed runtime starts and repeated restarts.
- Audit-log volume and unusual role changes.
- Disk usage under `agents_root`.

Operator dashboard:

- running agents
- failed runtimes
- leaders/executors health
- active private and leader-scoped sessions
- recent events

Alerts should prefer actionable states: failed process, stale job, path guard
failure, migration drift and secret redaction failure.
