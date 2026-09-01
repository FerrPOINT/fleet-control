# Java Agent Adapter Contract

Java Agent is modeled in MVP and implemented in phase 2.

Expected launch inputs:

- `AGENT_SERVER_PORT`
- `SPRING_CONFIG_ADDITIONAL_LOCATION`
- workspace cwd

Expected endpoints:

- `GET /actuator/health`
- `POST /api/v1/agent/chat/stream`
- `GET/POST /api/v2/sessions`
- `GET /v1/capabilities`

Until implemented, provisioning and lifecycle operations must return a typed
phase 2 response instead of attempting a partial launch.
