# Environment

Prefix: `FLEET_CONTROL_`.

Required production values:

- `FLEET_CONTROL_DATABASE__URL`
- `FLEET_CONTROL_JWT_SECRET`
- `FLEET_CONTROL_AUTH__MODE=hmac`
- `FLEET_CONTROL_AUTH__JWT_ISSUER=fleet-control`
- `FLEET_CONTROL_AUTH__JWT_AUDIENCE=sdlc`
- `POSTGRES_PASSWORD` when using Docker Compose

Important runtime values:

- `FLEET_CONTROL_FLEET__AGENTS_ROOT`
- `FLEET_CONTROL_FLEET__HERMES_SOURCE`
- `FLEET_CONTROL_FLEET__HERMES_COMMAND`
- `FLEET_CONTROL_FLEET__JAVA_AGENT_SOURCE`
- `FLEET_CONTROL_FLEET__JAVA_AGENT_COMMAND`
- `FLEET_CONTROL_FLEET__AGENT_PORT_BASE`
- `FLEET_CONTROL_FLEET__AGENT_PORT_STRIDE`
| `FLEET_CONTROL_FLEET__RETENTION__STALE_ARCHIVED_DAYS` | 30 | Stale threshold (days) for archived agent folders |
| `FLEET_CONTROL_FLEET__RETENTION__REVIEW_INTERVAL_SECS` | 3600 | Scheduled stale-folder review period (seconds) |

Default ports:

- backend: `23801`
- frontend: `23802`

## OIDC mode (auth.mode=oidc)

- `FLEET_CONTROL_AUTH__MODE=oidc` — включает RS256/JWKS-валидацию access-токенов; локальный HMAC и local login отключены fail-closed.
- `FLEET_CONTROL_AUTH__OIDC_ISSUER_URL` — issuer провайдера (обязателен в oidc-режиме); `iss`-клейм проверяется строго.
- `FLEET_CONTROL_AUTH__OIDC_JWKS_URL` — переопределение JWKS (default `<issuer>/keys`).
- `FLEET_CONTROL_AUTH__OIDC_AUDIENCE` — ожидаемый `aud` (пусто = не проверять).
- `FLEET_CONTROL_AUTH__OIDC_ROLE_CLAIM` — клейм роли (default `role`; admin→Admin, operator/maintainer→Operator, прочее→User).
- `FLEET_CONTROL_AUTH__OIDC_JWKS_REFRESH_SECS` — интервал обновления кэша ключей (default 300; принудительный refresh при неизвестном `kid`).
