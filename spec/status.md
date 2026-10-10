# thelivery — status

**Read this first each session.** The state of play, in the present
tense. Rules are in `CLAUDE.md`, what is left in `spec/backlog.md`,
what happened in `spec/history/`. This file is rewritten, not added
to, and stays at 150 lines or fewer.

Repo: `~/Projects/thelivery`, pushed to GitHub (`main`). **A push to
`main` deploys the live site** (https://thelivery.silverleaf.services),
so commits are pushed only when Jason says.

## Where it stands

**Live** (`de2c793`): the card catalog with edit mode, roles and the
audit timeline, the OG Overlay Studio and share pages, the Mobile &
Touch work order (phases 0–6, `spec/history/2026-10-09-aar.md`) and the
`/license` page (`spec/history/2026-10-09-aar-b.md`).

**Committed, not live** (pushed when Jason says), the bottom bar, star
and Playing pill work order:
- Part A `1f638b8`: on phones the side bug is a bottom bar; flyouts
  open upward; it hides while a text field has focus.
- Part B `ae3bf3e`: the star toggles off on touch and follows the
  hover table with a mouse; stars, theme pick and text size are kept
  in the visitor's browser (`frontend/src/composables/visitorMemory.ts`).
  Once live, stars stored on cards are ignored (D6); locally none are set.
- Part C `a3fe277`: the Playing pill sleeps after every start, a
  resting mouse puts it back to sleep, and a tap or a scroll wakes it
  on touch; paused, it stays.
- All three checked in the harness (`spec/mobile-pass/partA…partC`):
  1440×900 identical to `partA-base`. Nothing checked on a device.

**Docs** follow the Projects standard since 2026-10-09: `docs/` is
`spec/`, finished work is in `spec/history/`, open work in
`spec/backlog.md`.

**Jason's hands-on checks**: `spec/shakedown.md`, §17 (two iPhone
runs and the device-only checks, including the bottom bar and the
Playing pill).

**Pre-launch**: the license page is live; CORS lock (B-7), the draft
tag (B-8) and the README (B-9) are open.

## Next

1. Push Parts A–C, when Jason says; then his device shakedown (§17).
2. B-1: Jason's first OG Maker road test.
3. The pre-launch items B-7 to B-9.

## Working with Jason

- A push deploys: commit each step, push only when he says.
- Desktop at 1440×900 stays pixel-identical: `node frontend/shot.mjs
  --set <label>` then `--compare partA-base <label>`.
