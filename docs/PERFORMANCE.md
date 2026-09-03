# Performance

MVP targets:

- Session list should remain responsive with filters by user, agent, leader and
  state.
- Message transcript queries use `(session_id, created_at)` ordering.
- Process logs and audit logs are bounded by limit and indexed by time.
- Dashboard avoids full transcript loads.
- Screenshot and Playwright fixtures mock API calls for deterministic UI checks.

Backend considerations:

- Add pagination before large production fleets.
- Keep runtime health checks bounded by timeout.
- Avoid serial health polling across many agents when a background reconciler can
  cache status.
- Use database indexes listed in `DATABASE_INDEXES.md`.

Frontend considerations:

- Query by page/view.
- Disable buttons while mutations are running.
- Prefer derived summaries over full JSON dumps outside debug panes.
