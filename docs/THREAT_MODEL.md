# Threat Model

Primary risks:

- accidental sharing of one `HERMES_HOME` between agents
- path traversal from workspace/config inputs
- leaking model/API tokens through env previews or logs
- stale process status after backend restart
- starting Java Agent before its isolation policy is complete

Controls:

- sequential DB identity with unique paths
- guarded path helpers and folder markers
- redaction at write time
- status reconciliation through health checks
- Java Agent adapter disabled for launch in MVP
