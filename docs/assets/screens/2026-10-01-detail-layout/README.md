# Fleet Detail Geometry Evidence

Captured on 2026-10-01 (Europe/Moscow) from live Central Auth/Fleet APIs and a
production nginx image in isolated QA Compose. No intercepted APIs or mock
responses. QA agents were never started; their records were archived via API.
The QA session is retained because the product has no session delete endpoint.

- [Workspace, dark, 1920 px](workspace-dark-1920.png)
- [Leader sessions/team rail, light, 1024 px](leader-light-1024.png)
- [Session transcript/controls, gray, 375 px](session-gray-375.png)

The browser measured the actual 320 px rail, column gap, horizontal placement
and primary-before-rail stacking, not just DOM markers. All four detail routes
passed 7 viewports x 3 themes (84 combinations), including 1023/1024 and
1279/1280 boundaries. The same run repeated the 90 agent-tab and 120 main-route
combinations: 3 tests passed without retries. Full-page captures were opened
and inspected; image height can exceed viewport height.

[results.json](results.json) records image/config identity and source-content
fingerprints of the reviewed patch before commit. This is not an OCI source
revision attestation or evidence that the working user deployment was updated.
The platform-wide detail audit remains open for other consumers.
