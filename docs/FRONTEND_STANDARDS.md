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

## 7. UI Shell Contract

Fleet Control follows the Base [UI Shell Standard](https://github.com/FerrPOINT/services-base/blob/main/docs/platform/UI_SHELL_STANDARD.md).
The shared app shell owns one left navigation, one global header and a fluid
right work area; operational routes must not replace them with route-local shell
variants.

- `/leaders`, `/executors`, `/agents`, `/workflows`, `/logs` and audit/catalog
  surfaces use the available work width; filters and pagination stay in a local
  page row and data overflow stays inside the data container.
- Agent, executor and leader details use a fluid primary column plus a bounded
  technical rail that moves below content at narrow widths.
- Settings and destructive/edit forms use a readable constrained column and do
  not impose their maximum width on operational catalog routes.
- Expanded desktop sidebar, compact tablet rail and mobile drawer preserve the
  same order, active state and permission-aware navigation. The mobile drawer
  traps focus, closes with Escape and returns focus to its trigger.
- Global service/profile/theme controls stay in the one-row header; page title,
  breadcrumbs, filter bars and mutation actions stay below it in page content.
- Shell changes prove 375, 1440 and 2560 px behavior, direct-route active nav,
  header alignment, no document overflow and a keyboard drawer path.

## 8. References

- [UI/UX](UI_UX.md) — route behavior and operational states.
- [Frontend Architecture](FRONTEND_ARCHITECTURE.md) — implementation boundaries.
- [Base UI Shell Standard](../../../../UI_SHELL_STANDARD.md) — common shell contract.
