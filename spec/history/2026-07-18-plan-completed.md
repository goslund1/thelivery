# 2026-07-18: finished items from the work list

Moved word for word from `docs/plan.md` on 2026-10-09, when it was split
into `spec/status.md` and `spec/backlog.md` (frozen copy:
`verbatim/2026-10-09-plan.md`). Each item carries this date. Paths
inside them are as they were then (`docs/` is now `spec/`).

## From "Live data gaps (from session-36 shakedown)"

- ~~Create Geoff's editor account on live~~ **DONE** — Jason created it via Add User; as of 2026-07-18 Geoff hasn't signed in yet (temp password stays valid until he completes the forced change).

## From "User management gaps (from session-38 audit follow-up)"

- ~~No way to list users~~ **DONE 2026-07-18** — admin-only `GET /api/users` + read-only Users section in Account Settings (username, role badge, "pending" tag for temp-password accounts that haven't signed in yet; refreshes after Add User).

- ~~`user.create` isn't audit-logged~~ **DONE 2026-07-18** — `create_user` now records `user.create` with the acting admin, role, and temp-password flag; shows in the Timeline tab.
