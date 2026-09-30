# Agent Detail Evidence

Captured on 2026-10-01 (Europe/Moscow) from an isolated production Compose
deployment with live Central Auth and Fleet APIs. No API interception or mock
responses were used. The QA executor was never started and was archived by the
test through the normal API afterwards.

- [Runtime, light, 375 px](runtime-light-375.png)
- [Config, gray, 1280 px](config-gray-1280.png)
- [Workspace, dark, 1920 px](workspace-dark-1920.png)

All six tabs passed the 90-combination matrix (5 viewports x 3 themes). Each
capture is full-page, so image height can exceed the viewport height. Selected
images were opened and inspected. Source-content and image/config identity are
recorded in [results.json](results.json); capture preceded the task commit, so
the file fingerprints identify the tested patch rather than asserting an OCI
revision label.

This closes agent-detail localization/loading/mutation findings, not the
separate detail-column geometry, shared header or whole-platform release gates.
