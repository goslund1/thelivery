# 2026-10-09: finished items from the work list

Moved word for word from `docs/plan.md` on 2026-10-09, when it was split
into `spec/status.md` and `spec/backlog.md` (frozen copy:
`verbatim/2026-10-09-plan.md`). Each item carries this date. Paths
inside them are as they were then (`docs/` is now `spec/`).

## From "Active"

### 2. Mobile layout — DONE 2026-10-09
Built from the Mobile & Touch work order, Phases 0–6 (commits 24282cd → 633b0d9, all live). The phone is a full tuning-entry and slideshow-building surface; desktop at 1440×900 is pixel-identical to the pre-work baseline. What's left is Jason's device shakedown: `docs/shakedown.md` §17 (two iPhone runs + device-only checks). Per-phase audits are tracked in `docs/mobile-pass/<phase>/audit.txt`; the after-screenshots are in the same folders locally (gitignored). Re-run with `node frontend/shot.mjs --set <label>` and `--compare <baseline> <label>`. The `before` set predates the license page (its 1440 shots no longer match HEAD); `partA-base` is the baseline from 2026-10-09.

**Bottom bar, star and Playing pill** (work order in `Drop Offs/`, 2026-10-09): Part A committed (`1f638b8`), not yet live: on phones the side bug is a bottom bar (the collapsed tab is gone), flyouts open upward, and the bar hides while a text field has focus. Part B built, not yet committed or live: the star toggles off on touch and follows the hover table with a mouse; stars, theme pick and text size are kept in the visitor's browser (`composables/visitorMemory.ts`). Part C built, not yet committed or live: the Playing pill sleeps after every start (a resume no longer leaves it up), a resting mouse puts it back to sleep after 3 s, and on touch a tap or a scroll wakes it for 1.5 s; paused, it stays.

## From "Maintenance"

### ~~Phantom unsaved card on page load~~ FIXED 2026-10-09
A fresh load (signed out, no edits) marked card 1 dirty, arming the `beforeunload` "Leave site?" guard for every visitor. Cause: `RecipeSection.vue`'s `onMounted` applied the saved car variant without `suppressFlush`, so swapping in its upgrades fired the sync `local.upgrades` watcher → `flush()` (copying the tuning panel's not-yet-updated rows over the variant) + `markDirty()`. Fixed with the same guard the variant switcher and the reset use.

### ~~Touch step could press "Reset Stock"~~ FIXED 2026-10-09
Found while testing the fix above. A tuning −/+ step (acts on `pointerdown`, mobile pass Phase 3) can open the Springs & Dampers or Transmission prompt under the finger; in Chromium the same tap's click then landed on the prompt's button — "Reset Stock" zeroed the tune's alignment, springs and damping. Both prompts now ignore presses for 400ms after opening (`promptReady()` in `TuningAdjustments.vue`). This was live from Phase 3 until this fix: any tune edited with the step buttons on a touch device in that window is worth a look in its card History.

## From "Found during the bottom bar work order, 2026-10-09"

- **Stars already on live cards** go grey for everyone once Part B is live: the stored `isFavorite` is ignored (D6). Locally no card has one set.

## From "Pre-launch checklist"

- ~~**License / terms page**~~ **DONE 2026-10-09** — `/license` (LicensePage.vue, served with a 200, in the sitemap) states CC BY-NC-ND 4.0 by JasonHunt3r @ The Livery; site-wide footer line links to it; `GET /api/cards` attribution `url` now points at it. Built from Jason's `docs/LICENSE_PAGE_DRAFT.md` with the ND wording (no "remix").
