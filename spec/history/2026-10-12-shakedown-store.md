# 2026-10-12: Shakedown's store, deployed

Shakedown (the hands-on checker, `~/Projects/Shakedown`) is hosted at
`/shakedown/`; its order asked for a store here, admin only, built on a
local branch and deployed on Jason's say.

- **Caddy:** Geoff set `/shakedown/` up by hand on the droplet; it is his.
  A Caddy change of Claude's was taken out of the branch unrun. B-17:
  `deploy/setup-caddy.sh` would overwrite his setup if rerun.
- **Built** on `shakedown-store`: migration `0022_shakedown.sql` (new
  tables only) and `backend/src/shakedown.rs`, every route under
  `/api/shakedown/`, admin only. A 90-day Shakedown sign-in; The Livery's
  routes refuse it and Shakedown's refuse The Livery's 7-day token, which
  is unchanged. `auth::login`'s check moved into `auth::check_credentials`,
  shared, same behaviour. `cargo test` 42 → 51.
- **Before deploying:** the deploy keeps no copy of `data.db`, and
  `rollback.sh` only repoints the binary (B-18). Geoff took a backup
  (`sqlite3 .backup`, the service running) first.
- **Deployed** 2026-10-12 (`main` fast-forwarded to `4730809`, pushed):
  the deploy succeeded; before and after, `/`, `/license`, `/api/health`,
  `/api/cards` 200, the same bundle (`index-oY5TVQqk.js`), the same cards;
  `/api/shakedown/me` 404 → 401. Shakedown's own page went live after it,
  and Jason's live test passed (Shakedown's `history/2026-10-12-hosting-live.md`).
