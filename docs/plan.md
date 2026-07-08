# Ongoing Work List

Living to-do file for thelivery. Update this when items are started, completed, or deprioritized.

---

## Active — ordered by priority

### 1. Multi-car mashup shakedown
Foundation built on Smokin (3-car test case: 599D, FD Corvette #777, Austin Healey Sprite).

**Shipped:**
- Tab strip UI in `RecipeSection.vue` — renders when `variants.length >= 2`
- Auto-propose banner in edit mode — detects distinct carIds across card images, offers one-click tab setup
- Auto-propose builds all N variant tabs directly (sorted by photo count), current recipe data on the anchor tab
- `+` button consolidation — single button, expands to Car/Tune choice when both are valid
- Gallery `activeCarId` prop + image filtering wired (`Gallery.vue`, `CardView.vue`)

**Still needs shakedown:**
- Verify gallery filters correctly when switching tabs (end-to-end on Smokin)
- Fill in tune/spec data per variant and confirm save/restore round-trip
- View-mode tab strip experience for visitors (not just edit mode)
- Edge cases: removing a variant, single-car cards unaffected, save/discard behavior

---

### 2. Mobile layout
Narrow-screen pass for the full catalog. Known gaps:
- Theme builder flyout — doesn't fit on small screens
- General card layout on narrow viewports
- Deferred until now; no blocking dependencies remaining

---

## Maintenance

### Pre-launch checklist
- **Lock CORS to production domain** — currently `CorsLayer::permissive()` in `backend/src/main.rs`. Change to `CorsLayer::new().allow_origin("https://thelivery.silverleaf.services")` before public launch.

### Backfill pass (another round coming)
- Card data was brought in line once. Expect another round after card accent, tuning gate, and other structural changes land.
- Done in-app via EditCardModal; revisit when the dust settles.

### Deferred
- **`car_colors`** — factory color options per car. Requires scraping Forza wikis. No ETA.

---

## Recently completed

- Settings/Admin UI reorganization: Account panel (change password + Add User gated behind toggles, Admin Panel → button, Sign Out); separate Admin panel with Tools tab (image tools, stats, orphans, trash, seed) + Export Card tab (YAML download/import, legacy repair at bottom); Tune Suggestions stays in Filters flyout only — 2026-07-07
- figurePath patching in migration flow: ImageMigrationModal snapshots old paths and patches TextSection.figurePath after migrate; Repair Figure Paths endpoint matches by sequence number before falling back to lead image — 2026-07-07
- Trash compactor: orphans scan moves to trash instead of hard-delete; user-deleted images move to trash on card save; GET/DELETE /api/admin/trash + POST /api/admin/trash/restore; trash_log table (migration 0012); reason badges, select/restore/delete — 2026-07-06
- Security & quality audit: fixed orphan scanner (queried card body instead of images table), delete_images legacy variant naming, orphan scan skips uploads/trash/, rate limit HashMap growth, suggestion adjustments 64KB cap, e:any catches — 2026-07-06
- Multi-car mashup foundation: tab strip, auto-propose banner, + button consolidation, gallery carId filtering — 2026-07-06
- Image migration pipeline: full re-file + rename, all 11 cards migrated, structured filenames, drum CarPicker, toast drawer, dotenvy, FH6 FD cars, Bronco R — 2026-07-06
- Car identity model: cars table, CarPicker, PhotoDetail, livery picker, AI color assess, Step 8 hardening — 2026-07-06
- Image identity refactor: images table as source of truth, integer PKs, inject_images on read, sync on write, ImageMigrationModal — 2026-07-05
- Suggest bar two-tier trigger, card accent override, per-section defaultOpen, RecipeSection one-way flow, code quality passes 1 + 2 — 2026-07-05
- Transmission/gearing system, TuningAdjustments Define Stock + undo, ThemeBuilder effects sliders — 2026-07-04
- Theme builder + ColorPicker, DrawerPanel, card history, YAML export/import, suggest bar two-surface redesign — earlier sessions
