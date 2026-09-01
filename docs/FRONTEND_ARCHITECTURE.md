# Frontend Architecture

The frontend is a React/Vite application with:

- React Router route tree in `src/app/router.tsx`
- shared shell in `src/widgets/app-shell.tsx`
- typed API helpers in `src/api`
- page modules under `src/pages`
- shared UI primitives under `src/shared/ui`

Server state uses TanStack Query. Auth state uses Zustand with persisted
non-secret user metadata only.
