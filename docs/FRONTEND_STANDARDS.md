# Frontend Standards

Standards:

- React pages use the shared app shell and permission-aware navigation.
- Admin/operator routes use `PermissionGate`.
- User routes rely on backend filtering and show access denied when forbidden.
- Every page has loading, empty and error behavior.
- Mutating controls are disabled while saving.
- User filters show avatar/icon, display name and removable chips for
  admin/operator.
- The session create agent select must have a stable default after agents load.
- Executor technical tabs may alias agent detail tabs, but aliases must appear in
  route docs and screenshots.
- Use icon buttons where icons are clear and text buttons for explicit
  destructive or domain commands.

Evidence:

- Playwright covers core flows.
- `pnpm screenshots:local` regenerates manifest and screenshots.
- `pnpm screenshots:verify` checks manifest rows, required routes and PNG files.
