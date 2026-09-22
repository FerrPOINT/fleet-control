# Agents directory UI review

## Scope

- Localized the complete `/agents` experience in Russian and English.
- Replaced tall agent cards with a compact, divided directory.
- Simplified the storage review section and reduced mobile page height.
- Added explicit loading, empty, error, retry, and pending mutation states.
- Preserved create-form input after a failed request and disabled all controls while the request is pending.

## Live validation

The authenticated Fleet UI was rebuilt in the shared Docker workspace and checked at 375, 768, 1280, and 1920 px in dark, gray, and light themes (12 states total).

- No horizontal overflow or nested document scrollers.
- No serious or critical accessibility findings from axe.
- No unexpected browser console or network errors.
- Mobile navigation drawer and page actions remain usable.
- Russian dates and all page copy render in the selected locale.

## Automated checks

- Frontend unit tests: 88 passed.
- ESLint: passed.
- TypeScript and production build: passed.
- OpenAPI generated contract check: passed.
- Prettier check for all changed source and evidence files: passed.

The repository-wide Prettier check still reports existing Windows line-ending differences in four unchanged files: `index.html`, `src/main.tsx`, `src/pages/login/index.tsx`, and `src/pages/sso-callback/index.tsx`.

## Screenshots

- `sso-fleet-dark-375.png` - mobile, dark theme.
- `sso-fleet-light-375.png` - mobile, light theme.
- `sso-fleet-dark-1920.png` - desktop, dark theme.
- `sso-fleet-light-1920.png` - desktop, light theme.
