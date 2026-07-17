# Handoff: Separate Users Database — CLOSED

**Status: Not needed. Resolved via a different approach.**

The credential loss problem was solved by adding a user seed file (`backend/seed/users.json`) that auto-loads when the users table is empty. A `rm data.db` + backend restart now restores both cards and user credentials automatically.

Production was never affected — it has its own separate database that is never wiped.

No action required from Geoff.

See `aar-2026-06-28.md` for full details.
