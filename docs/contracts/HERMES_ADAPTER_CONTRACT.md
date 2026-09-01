# Hermes Adapter Contract

Hermes is the first implemented runtime.

Environment:

- `HERMES_HOME=agentN/config`

Working directory:

- `agentN/workspace`

Managed files:

- `config/config.yaml`
- `config/SOUL.md`
- `config/.env`
- `config/skills`
- `runtime/source.json`

Lifecycle:

- start: configured Hermes command with dashboard host and port
- stop: terminate tracked process
- restart: stop then start
- health: reconcile tracked process state
