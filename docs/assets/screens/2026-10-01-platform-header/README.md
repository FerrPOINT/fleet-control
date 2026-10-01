# Fleet Header Acceptance

Production candidate, 2026-10-01, isolated QA, real Central Auth/Fleet APIs.
No API mocks, skipped tests or Playwright retries. Shared Base #119 is consumed
through the actual installed file-package snapshot.

- Five live tests passed in 5.7 minutes: Header, agent tabs, detail geometry,
  main routes and localization. 393 layout/theme/viewport combinations plus
  four localization viewports, three themes and 320-2560 px breakpoint edges.
- Header measures 60 px full width; buttons are 44 px mobile / 40 px desktop.
  Sidebar starts below it and measures 72/264 px. Navigation, keyboard/touch,
  menus, opaque runtime six-service catalogue, account identity, native inert
  cleanup, Escape/focus and central logout/re-entry are verified.
- No page overflow, unexpected scroller, console/request errors or serious/
  critical axe in measured layout gates. Header navigation performs no API writes.
- The four selected PNGs are full-page, opened and visually inspected.
  QA detail agents are never started and are archived via their own API.
  Session history remains only in the isolated QA project until cleanup.
- This is a candidate/component acceptance, not an OCI revision attestation,
  working-demo rollout or final platform release. The post-merge image repeats
  all five tests and the full twelve-test SSO file.

Artifacts: [metadata](metadata.json), [raw Header cases](results.json),
[mobile](agents-dark-375.png), [desktop](agents-dark-1920.png),
[wide desktop](agents-dark-2560.png), [service menu](services-menu-dark-375.png).
The older README gallery is explicitly historical.
