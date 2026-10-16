# 2026-10-16 — Shakedown's store leaves The Livery

From App Claude's order "Housework before the week" (part 1; Shakedown's
B-31 step 6, its B-32 dropped). Jason, 2026-10-16: "make the call to say
fuck it and clean up the droplet as well as local. thelivery.silverleaf
can go back to just being for the car show for now."

## Why

SharkAttack at `https://shakedown.qdeco.com` has carried every run since
the cutover on 2026-10-14 (Shakedown's `spec/sharkattack-silo.md`). The
store here had nothing left worth keeping that wasn't already copied.

## What was done (on `main`, not pushed)

- `main` brought level with `origin/main` (fast-forward to `b1f0e88`, the
  backup route), then `backend/src/shakedown.rs` frozen word for word in
  `verbatim/2026-10-16-shakedown.rs` and removed: its module, its routes
  (`/api/shakedown/…`), the live channel in `AppState`, and the four
  crates only it used (`futures-util`, `sha2`; dev: `tower` with `util`,
  `http-body-util`).
- The token `kind` check in `auth.rs` stays: the old 90-day sign-ins are
  still signed with the shared secret, and must never edit The Livery.
- **No migration.** The `shakedown_*` tables stay in the database, and
  `0022_shakedown.sql` and `0023_shakedown_counts.sql` stay in
  `migrations/` (sqlx refuses a database that has run a migration the
  binary doesn't know). A no-op migration was left out: it would only
  make a binary rollback refuse the database (B-18). Dropping the tables
  waits for Jason (the order's D-HW5).
- `CLAUDE.md` lost the exception that let Shakedown's code be pushed
  without asking; the Git rule is plain again: push only when asked.
- `spec/shakedown.md` is **kept**: it is Jason's hands-on checklist for
  The Livery itself, not Shakedown's (the order took it for Shakedown's).

## The record of the old runs

The backups in `~/Projects/SharkAttack-Backups/`, the last
`shakedown-2026-10-15T115011Z.db` (2026-10-15 11:50, 1.0 MB). What came
after on the old page is let go (Jason: "nothing of too much value").

## The branches, and what each carried

Each tip is named so it can be found again (`git branch <name> <sha>`)
while git still keeps it.

| Branch | Tip | What it carried | In `main`? |
|---|---|---|---|
| `shakedown-store` | `4730809` | The store's first build (2026-10-12) | Yes |
| `shakedown-test-keys` | `e0e1e56` | Test-build keys and counts (migration 0023) | Yes |
| `shakedown-parts` | `73f61dc` | Part ids, the box, carry over | Yes |
| `shakedown-run-texts` | `5a93dfd` | A run keeps its checklist's text; never pushed | Yes |
| `sharkattack/backup-route` | `b1f0e88` | `GET /api/shakedown/backup` for the move (B-31 step 1) | Yes |
| `shakedown-run-history` | `5e27608` | One run, one history: migration 0024, undo and redo (B-26). Never merged or deployed here; the same work is live in SharkAttack | **No** |

Deleted on this Mac. Deleting `shakedown-run-history` and
`sharkattack/backup-route` on GitHub waits for the deploy, with the push.

## Left for Jason's say (the deploy)

Push `main` (deploys the live site), remove `/shakedown/` from Caddy
(Jason with Geoff), delete the two branches on GitHub, then
`curl -I …/shakedown/` should be 404 and the car show's home page 200.

## Deployed (same morning)

Jason pushed `main` at about 06:25 (the Deploy run 38907706744 passed,
3 min 18 s) and deleted `shakedown-run-history` and
`sharkattack/backup-route` on GitHub; `git branch -a` shows `main` only.
The old `/shakedown/` page is now served by nothing but Geoff's Caddy
block, which stays until Jason has talked to him. Not checked from here:
`curl` of the live site was refused by this session's permissions.
