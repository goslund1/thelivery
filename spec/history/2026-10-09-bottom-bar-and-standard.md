# 2026-10-09: bottom bar, star and Playing pill; thelivery to the Projects standard

Two of App Claude's work orders, carried out by Claude Code the same
evening in Jason's order. Both are frozen in `verbatim/`
(`2026-10-09-work-order-bottom-bar-star-pill.md`,
`2026-10-09-work-order-projects-standard.md`).

## Bottom bar, star and Playing pill

Built in three commits, none pushed (a push deploys; Jason says when).
Checked in the Playwright harness, not on a device. After each part
the 1440×900 shots are pixel-identical to `partA-base`, a new baseline
taken before Part A: the old `before` set predates the license page
and no longer matches.

**Part A, `1f638b8`.** On phones the side bug's edge tab became a
floating bar at the bottom: viewing set (Account, Edit, Filters, Theme,
Text size, Expand), editing set (Edit, Save All with its count, Next
edit, New Card, Filters, Expand). Flyouts open upward from the bar;
the bar hides while a text field has focus and never moves. The
harness gained bar audits for signed out, signed in and editing,
flyouts against the bar, overlays covering it, the focused-field hide
and the unsaved count. Two harness slips were caught by measuring: an
extra wait shifted the 1440 edit shot's slideshow timing (made
phone-only), and the `.ta-header` check first counted headers
scrolling past in flow (narrowed to the stuck one).

**Part B, `ae3bf3e`.** The star turns off on touch, and with a mouse
follows Jason's table (hover invites, a starred star brightens on
hover, a click shows its plain result until the cursor leaves). Stars,
the theme pick and text size live in a visitor memory
(`composables/visitorMemory.ts`); a star no longer writes the card or
marks it unsaved. Found while there, and filed: the app is blank with
storage blocked (it already was: `auth.ts` and `api.ts` read storage
unguarded); two admin controls paint their "on" look on hover; the
star's `.pop` was never wired.

**Part C, `a3fe277`.** Reproduced first at 1440: pausing and resuming
from the pill left "Playing" up for good. Now every start ends with the
pill asleep, a resting mouse puts it back to sleep after 3 s, and on
touch a tap or a scroll wakes it for 1.5 s; paused, it stays.

## thelivery to the Projects standard

Docs only. One commit per step: frozen copies of `plan.md` and
`shakedown.md`; `docs/` renamed `spec/` and `completed/` renamed
`history/`; dated files moved into history with the date in front (the
date written in the file, else its last content change, since many
last commits were the 07-17 move into `completed/`); the history
README; `plan.md` split into `backlog.md` (B-1 to B-16) and one history
file per date for its 44 finished items, checked by script against the
frozen copy; `status.md`; every path pointed at `spec/`; the checker.

Decisions on the way:
- `touchups.md`, a bug log last touched in June, went to history. The
  other root docs stayed as live specs, each named in `CLAUDE.md`.
- The shakedown already fits the standard's shape (147 rows, none over
  450 characters, 13 KB), so it was left as it is.
- Jason's license draft (`LICENSE_PAGE_DRAFT.md`) served as the license
  page's work order (built as `14db2d2`): frozen as
  `verbatim/2026-10-09-license-page-draft.md`.
- The ignition patch in `Drop Offs/` had been committed as `1aaf3dc`:
  frozen as `verbatim/2026-10-09-fix-ignition-blank-download.patch`.
