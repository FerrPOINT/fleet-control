# Risk Register

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Hermes CLI command differs by environment | start fails | command is configurable through `FLEET_CONTROL_FLEET__HERMES_COMMAND` |
| Backend restart loses child process handles | stale status | health action reconciles untracked Hermes as stopped |
| Java Agent model drifts before implementation | rework | keep adapter contract documented and visible in UI |
| Agent folders are manually edited | inconsistent state | `.fleet-agent.json` marker and idempotent provisioning |
| Secrets leak in logs | credential exposure | redact before persistence and API response |
