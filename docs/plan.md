# Ongoing Work List

Living to-do file for thelivery. Update this when items are started, completed, or deprioritized.
Completed items move to `docs/completed/` (create a dated file when a batch lands).

---

## Active / next up

### Add-photos SVG icon
- File: `frontend/src/components/Gallery.vue` ~line 295
- Current state: 4 landscape 4:3 rects filling quadrants (11.5×8.625 in a 24-unit viewBox), rendered at 22px. `+` arm stroke-width 3.6, drawn on top.
- Status: Jason was not happy with the last iteration; benched. Resume carefully — read `docs/aar-session2` for the full failure postmortem before touching anything.
- If still too small: options are (a) shorten arm stroke-width, (b) shorten arm length, (c) increase render size with Jason's OK.

### Backfill car IDs
- Existing cards have no `carId` set. Need to go through each in EditCardModal and assign via CarPicker.
- No code work needed — this is a content task done in-app.

---

## Parked (needs more thought / not started)

### Submit Tune feature
- Full plan at `docs/plan-submit-tune.md`.
- Trigger redesign needed: current logic fires on first non-stock slider move, but cards already ship with non-stock values — needs a different trip condition.
- UI: contact/credit form (Gamertag, PSN, Discord, Reddit, etc.), "Ask Me Later" deferred state.
- Backend: submissions table + API, admin review queue ("the pile").

### Mobile layout
- Theme builder flyout on narrow screens (the DrawerPanel + ColorPicker wing).
- General narrow-screen pass for the main catalog.
- Deferred until browser tuning work settles.

### E2E gearing dialog flow verification
- Confirm the full round-trip: upgrade added via transmission picker → gear rows update → UpgradesPicker reflects the part → stock-restore removes it.
- Not a code task — needs manual walkthrough on a fresh card.

---

## Recently completed (last 2 sessions)

- Transmission/gearing system (locked sliders, glass picker modal, auto-add/remove upgrades, view+edit mode parity, legacy name migration) — 2026-07-04
- Car identity: `cars` table, CarPicker, PhotoDetail, alt text — 2026-07-04
- Suggest bar two-surface redesign — 2026-07-04
- Theme builder + ColorPicker (palette drag-reorder, title bar, HSL namer, DrawerPanel) — ~2026-06-30
- Card history UI (CardHistoryModal) — earlier session
- Card migration tool (YAML export/import) — earlier session
