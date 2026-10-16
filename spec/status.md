# thelivery — status

**Read this first each session.** The state of play, in the present
tense. Rules are in `CLAUDE.md`, what is left in `spec/backlog.md`,
what happened in `spec/history/`. This file is rewritten, not added
to, and stays at 150 lines or fewer.

Repo: `~/Projects/thelivery`, pushed to GitHub (`main`). **A push to
`main` deploys the live site** (https://thelivery.silverleaf.services),
so commits are pushed only when Jason says.

## Where it stands

**Live** (deployed 2026-10-09, bundle `index-oY5TVQqk.js`, checked
against a local build of HEAD): the card catalog with edit mode, roles
and the audit timeline, the OG Overlay Studio and share pages, the
Mobile & Touch work order (`spec/history/2026-10-09-aar.md`), the
`/license` page (`spec/history/2026-10-09-aar-b.md`), and the bottom
bar, star and Playing pill work order
(`spec/history/2026-10-09-bottom-bar-and-standard.md`):
- Part A: on phones the side bug is a bottom bar; flyouts open upward;
  it hides while a text field has focus.
- Part B: the star toggles off on touch and follows the hover table
  with a mouse; stars, theme pick and text size are kept in the
  visitor's browser (`frontend/src/composables/visitorMemory.ts`). No
  live card had a stored star, so none went grey.
- Part C: the Playing pill sleeps after every start, a resting mouse
  puts it back to sleep, and a tap or a scroll wakes it on touch;
  paused, it stays.
- Checked in the harness (`spec/mobile-pass/partA…partC`), 1440×900
  identical to `partA-base`. Not yet checked on a device.

**Shakedown's store is gone from `main`** (2026-10-16,
`spec/history/2026-10-16-shakedown-leaves.md`): the code, its routes and
its branches are out; its tables stay in the database (no migration).
**Deployed** 2026-10-16 06:25 (Jason's push, run 38907706744, passed);
its two branches deleted on GitHub by Jason. Left: the old `/shakedown/`
page is served by nothing but Geoff's Caddy block, which stays until Jason
has talked to him. The deploy keeps no copy of
`data.db` (B-18): ask Geoff for a backup before any deploy that brings a
migration.

**Docs** follow the Projects standard since 2026-10-09: `docs/` is
`spec/`, finished work is in `spec/history/`, open work in
`spec/backlog.md`.

**Jason's hands-on checks**: `spec/shakedown.md`, §17 (two iPhone
runs and the device-only checks, including the bottom bar and the
Playing pill).

**Pre-launch**: the license page is live; CORS lock (B-7), the draft
tag (B-8) and the README (B-9) are open.

## Next

1. Jason's device shakedown (§17), now that Parts A–C are live.
2. B-1: Jason's first OG Maker road test.
3. The pre-launch items B-7 to B-9.

## Working with Jason

- A push deploys: commit each step, push only when he says.
- Desktop at 1440×900 stays pixel-identical: `node frontend/shot.mjs
  --set <label>` then `--compare partA-base <label>`.
