# Screenshot Manifest

Generate screenshots with:

```bash
cd frontend
pnpm build
pnpm preview -- --host 127.0.0.1 --port 4173
pnpm screenshots
```

Captured viewports:

- `375x812`
- `1920x1080`
- `2560x1440`

Core screens captured for every viewport:

- `01-login.png`
- `02-register.png`
- `03-dashboard.png`
- `04-agents.png`
- `05-agent-create.png`
- `06-agent-overview.png`
- `07-agent-runtime.png`
- `08-agent-skills.png`
- `09-agent-config.png`
- `10-agent-workspace.png`
- `11-agent-sessions.png`
- `12-sessions.png`
- `13-session-detail.png`
- `14-workflows.png`
- `15-deployments.png`
- `16-logs.png`
- `17-settings.png`

Additional mobile captures:

- `375x812/18-mobile-dashboard.png`
- `375x812/19-mobile-agent-detail.png`

Current set: 53 PNG files under `docs/assets/screens/<viewport>/`.
