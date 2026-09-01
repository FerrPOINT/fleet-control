# Technical Specification

## Runtime Types

`hermes` is implemented in MVP. It is launched with per-agent `HERMES_HOME` and
workspace cwd.

`java_agent` is a first-class enum value and runtime template. Its launch
adapter is phase 2 and must target Spring Boot with:

- `AGENT_SERVER_PORT`
- `SPRING_CONFIG_ADDITIONAL_LOCATION`
- `/actuator/health`
- `/api/v1/agent/chat/stream`
- `/api/v2/sessions`
- `/v1/capabilities`

## Folder Allocation

The backend allocates the next ordinal in PostgreSQL and derives the agent name
from it. Directory creation is idempotent and uses a `.fleet-agent.json` marker.

## Security

All path operations must stay under `FLEET_CONTROL_FLEET__AGENTS_ROOT`. Secrets
must be redacted before logs and API env previews are persisted.
