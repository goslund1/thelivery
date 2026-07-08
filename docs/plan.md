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

**Tab mode design (settled):**
- **Cars mode** — auto-triggered when slideshow images span multiple distinct carIds. Tab order: lead image's car first, then by photo count descending. Each tab = that car's tune. Smokin is the canonical example (599D → Corvette → Austin Healey).
- **Tunes mode** — single car, multiple named tune variants (Race, Rally, Drift, etc.). Triggered manually via the `+` → Tune choice.
- Both modes share the same tab strip UI. Mode is implicit in the data shape: distinct carIds = cars mode, same/no carId with multiple variants = tunes mode.
- Tab mode can also be manually forced via the `+` button regardless of auto-detection.

**Figure image near recipe (pending):**
- In cars mode: each tab shows a small figure image of that car (lead image for that carId) near the tune section — anchors the tune visually.
- In tunes mode: same car image across all tabs.
- Recipe sections in general could benefit from a figure image slot regardless of tabs.

**Authoring gap / discipline presets (pending):**
- Secondary cars in a mashup (e.g. Corvette, Austin Healey in Smokin) have no tune data yet and no authoring UI to add one from within the card.
- Solution: discipline preset dropdown (Race / Rally / Drift / Street / etc.) on an empty variant tab that seeds base slider values from Jason's established starting points per discipline. Same concept as the existing upgrade presets but for adjustments. Then nudge sliders to the specific car's quirks and done.
- "Add tune for this car" affordance in the empty tab opens the discipline dropdown first, then the full slider editor.
- The upgrade presets system (`localStorage`, save/apply/delete) is the reference pattern to follow.
- **The existing preset list needs actual base values populated** — currently the discipline names exist but default slider values are not filled in.

**New card creation UX flow for multi-car detection (pending):**
- When a second batch of images is submitted with a carId that doesn't match the lead car, a passive button appears in NewCardModal signalling unresolved multi-car content. Non-interrupting — user keeps adding cars until done.
- Resolution is triggered either by clicking the button voluntarily, or by Save redirecting there if unresolved variants exist.
- At resolution, two paths:
  - **Yes tabs** — discipline dropdown per variant (from established profiles) seeds base slider values → tabs get defaults → finish creation
  - **No tabs** — single tune for the lead car or no tune at all → proceed as normal
- This mirrors the auto-propose banner already in RecipeSection (edit mode) but lives in the new card creation flow (NewCardModal) and is passive rather than inline.

**Deferred — community tune scrape + compare:**
- Scraping Forza tune share codes to compare community tuning approaches vs. in-house builds would be interesting analytical tooling.
- Park until app is live and primary content is established. Not a data quality tool, more a research/learning one.

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
- Car Tabs wizard: floating modal walks through each non-anchor car, user picks a tuning preset per car, creates tabs with pendingPresetId, auto-applies on first tab open — 2026-07-08
- Tab strip UX polish: delete 2→1 leaves last tab visible, delete last collapses to clean state + re-activates Car tabs button, short model-name labels with full name on hover title — 2026-07-08
- Folder tab visual styling: gold shelf via container border-bottom, all tabs margin-bottom:-1px, active tab border-bottom-color:var(--panel) to erase shelf beneath it — 2026-07-08 (NOTE: took ~1hr and significant token burn due to Claude inventing z-index/pseudo-element schemes instead of researching the standard CSS pattern first. Jason's feedback: research known UI patterns before writing code, present the approach for sign-off before executing.)
- CollapsibleSection HMR sync fix: immediate:true on store watch so expand/collapse state survives hot reload — 2026-07-08
- Multi-car mashup foundation: tab strip, auto-propose banner, + button consolidation, gallery carId filtering — 2026-07-06
- Image migration pipeline: full re-file + rename, all 11 cards migrated, structured filenames, drum CarPicker, toast drawer, dotenvy, FH6 FD cars, Bronco R — 2026-07-06
- Car identity model: cars table, CarPicker, PhotoDetail, livery picker, AI color assess, Step 8 hardening — 2026-07-06
- Image identity refactor: images table as source of truth, integer PKs, inject_images on read, sync on write, ImageMigrationModal — 2026-07-05
- Suggest bar two-tier trigger, card accent override, per-section defaultOpen, RecipeSection one-way flow, code quality passes 1 + 2 — 2026-07-05
- Transmission/gearing system, TuningAdjustments Define Stock + undo, ThemeBuilder effects sliders — 2026-07-04
- Theme builder + ColorPicker, DrawerPanel, card history, YAML export/import, suggest bar two-surface redesign — earlier sessions
