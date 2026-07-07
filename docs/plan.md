# Ongoing Work List

Living to-do file for thelivery. Update this when items are started, completed, or deprioritized.
Completed items move to `docs/completed/`.

---

## Active — ordered by priority

### 0. Image identity refactor + migration tool  ← SHIPPED (2026-07-05, commit aee2dfd)
**Safe return point:** commit `45be373` (feat: batch import flow) — everything before this work is stable.

**What we're doing and why:**
Card body JSON currently stores image paths as strings — paths are the de-facto identifier.
The `images` table has integer PKs but is a secondary/parallel structure; the card body is the source of truth.
This is fragile: path changes break references, sharing the same image across contexts requires file duplication.

**Target architecture:**
- `images` table is the single source of truth for all image data (path, thumbPath, stagePath, livery_id, car_id, alt, sort_order)
- Card body stores only `id` (images PK) + `alt` + `order` + `carId` per image — no paths
- Backend resolves full image data from the table on every card fetch (JOIN or second query)
- Backend syncs the table on every card save
- Migration tool handles existing cards (see below)

**Files that will change:**
- `backend/src/main.rs` — card read (inject images from DB), card write (sync to DB), normalize_bodies()
- `backend/migrations/0012_*.sql` — ensure images table has needed columns (may be a no-op)
- `frontend/src/types.ts` — CardImage: `path?` becomes optional (resolved server-side)
- `frontend/src/stores/cards.ts` — addImageToPool, setImageMeta touch points
- `frontend/src/components/` — Gallery, ImagePicker, PhotoDetail (all use image.path; stays same since backend still provides it)
- New: `frontend/src/components/ImageMigrationModal.vue` — admin tool

**Migration tool plan:**
Admin-only modal, walks cards one-by-one:
1. Shows card name + all current images (resolved from body paths — files exist on disk)
2. Ensures each image has an `images` table row (inserts if missing, skips if already present)
3. Car picker + livery name input for the batch
4. On confirm: creates livery → sets livery_id on all images → triggers assess → advances to next card
5. Skip button for cards with no photos or no livery needed

---

### 1. Image migration pipeline — full re-file + rename  ← SHIPPED (2026-07-06)

All 11 production cards migrated. Images live under structured `{slug}_{id}/` folders with structured filenames. Old files in `uploads/trash/`. All liveries have AI color assess data.

**What shipped:**
- Backend re-file pipeline, new folder/filename scheme, `POST /api/admin/images/migrate`
- `ImageMigrationModal` — car required gate, drum CarPicker, toast drawer, error persistence
- `dotenvy` — `.env` loaded at startup; JWT_SECRET and ANTHROPIC_API_KEY now stable across restarts
- Drum/reel CarPicker replacing the dropdown (no viewport overflow, `v-scroll-contain`)
- Error toasts stay visible until manually dismissed
- Smart catch block surfaces which step failed in the toast
- Assess skips cleanly when 0 images were migrated; backend tries all images before giving up
- All 13 FH6 Formula Drift cars added to DB + seed
- Ford Bronco R #2069 (2020) added for Photo Safari Japan card

---

### 2. Car identity model  ← SHIPPED (2026-07-06)
All 12 build steps shipped. Step 8 hardening done (livery.id no longer optional-chained in NewCardModal). Remaining deferred items:
- **Step 2 (deferred)**: `car_colors` scrape — factory color options per car. Requires a Forza wiki source.
- **Audit + crash test pass** — planned for later; harden further if gaps found.

---

### 4. Trash + orphan management  ← SHIPPED (2026-07-06, commit 5bee55d)

Ongoing maintenance workflow for unreferenced and user-deleted images.

**Why:** Two sources of dead files accumulate over time — (1) the orphan scanner finds files with no `images` table reference, (2) users explicitly remove images from cards via the × button. Both currently land in `uploads/trash/` but there's no UI to review or manage them. The trash dir also contains the migration-era originals that need clearing.

**Full workflow:**

1. **Scan** (manual trigger in Admin panel) — walks `uploads/`, identifies files with no matching row in the `images` table, moves them to `trash/`. Returns count + size of what was herded. Distinct from "delete" — scan only moves, never permanently removes.

2. **Trash viewer** — list of files currently in `uploads/trash/`, showing filename, size, origin card (if known), and reason:
   - `orphan` — no images table reference found at scan time
   - `user_delete` — explicitly removed from a card via the × button

3. **Delete permanently** — select all or individual files, hard-delete from disk. Irreversible; requires explicit confirm.

4. **Restore** — move a file back from trash, re-create its `images` table row, then open PhotoDetail-style assignment (pick card, set car/livery). `user_delete` entries are most likely to need this; the UI should surface them first or flag them differently.

**Data model — trash log:**
When any file is moved to trash (by scan or by card save), write a row to a new `trash_log` table:
- `id`, `filename` (basename in trash), `original_path`, `original_thumb_path`, `original_stage_path`, `card_id` (nullable), `images_row_id` (the PK that was deleted, for reference), `reason` (`orphan` | `user_delete`), `trashed_at`

This log is what makes restore a one-click undo rather than a manual re-discovery. Without it, restore has to treat every trashed file as unknown.

**Build order:**
1. Migration: add `trash_log` table (`backend/migrations/`)
2. Backend: `GET /api/admin/trash` — list trash with log data; `DELETE /api/admin/trash` — permanent delete (accepts `{ ids: [...] }` or `{ all: true }`); `POST /api/admin/trash/restore` — move back + re-insert images row
3. Update `DELETE /api/admin/orphans` — move to trash + write log instead of hard-delete
4. Update card save (`put_card`) — when `sync_card_images` drops an image row, move the physical files to trash + write log with `reason: 'user_delete'`
5. Frontend: Admin tab — relabel existing "Delete" button to "Move to Trash"; add Trash section with file list, reason badges, select/delete/restore controls

---

### 3. Multi-car mashup card  ← IN PROGRESS (2026-07-06)
Foundation working on Smokin (3-car test case: 599D, FD Corvette #777, Austin Healey Sprite).

**Shipped:**
- Tab strip UI in `RecipeSection.vue` — renders when `variants.length >= 2`
- Auto-propose banner in edit mode — detects distinct carIds across card images, offers one-click tab setup
- Auto-propose builds all N variant tabs directly (sorted by photo count), current recipe data on the anchor tab
- `+` button consolidation — single button, expands to Car/Tune choice when both are valid
- Gallery `activeCarId` prop + image filtering already wired (`Gallery.vue`, `CardView.vue`)

**Still needs shakedown:**
- Verify gallery filters correctly when switching tabs (end-to-end on Smokin)
- Fill in tune/spec data per variant and confirm save/restore round-trip
- View-mode tab strip experience for visitors (not just edit mode)
- Edge cases: removing a variant, single-car cards unaffected, save/discard behavior

---

## Parked

### Mobile layout
- Theme builder flyout + general catalog narrow-screen pass.
- Deferred until identity/livery model lands — that affects card layout.

---

## Maintenance

### Pre-launch checklist
- **Lock CORS to production domain** — currently `CorsLayer::permissive()` in `backend/src/main.rs:368`. Change to `CorsLayer::new().allow_origin("https://thelivery.silverleaf.services")` before public launch. Low risk while the backend binds to 127.0.0.1, but wrong in principle.

### Backfill pass (another round coming)
- Card data was brought in line once. Expect another round after card accent, tuning gate, and other structural changes land.
- Done in-app via EditCardModal; revisit when the dust settles.

---

## Recently completed

- Trash compactor: orphans scan now moves to trash instead of hard-delete; user-deleted images move to trash on card save; GET/DELETE /api/admin/trash + POST /api/admin/trash/restore; Admin tab trash section with reason badges, select/restore/delete; trash_log table (migration 0012) — 2026-07-06
- Security & quality audit pass: fixed orphan scanner (queried card body instead of images table — would have wiped all uploads), delete_images legacy variant naming (thumbs/stages never cleaned up on discard), orphan scan/delete now skips uploads/trash/, rate limit HashMap unbounded growth, suggestion adjustments 64KB cap, e:any error catches in UserSettingsModal + cardYaml.ts — 2026-07-06

- Multi-car mashup: tab strip, auto-propose banner, + button consolidation, gallery carId filtering wired — 2026-07-06

- Car identity model Step 8 hardening: livery.id required (no optional chaining), assess failure retry list + badge, assess failures non-blocking (queued for retry) — 2026-07-06
- Image migration pipeline — full re-file + rename: all 11 cards migrated, structured filenames, drum CarPicker, toast drawer, dotenvy, error persistence, assess skip, FH6 FD cars, Bronco R — 2026-07-06
- Suggest bar two-tier trigger: inline Share tweaks button activates on any slider change; floating push bar fires on 2+ tab categories; ASK ME LATER uses sessionStorage reload reminder; NOT FOR ME = session dismiss; S&D tier dialog gated to edit mode only — 2026-07-05
- Shakedown pass (desktop): 2 bugs found + fixed (CardHistoryModal z-index, EditCardModal Escape handler); sections 1-3, 5-7, 9 suggest bar, 11-13 verified; sections 4, 8, 14-15, 17 (mobile) still need manual/device run — 2026-07-05
- Suggestion Viewer + Promote: SuggestionViewer.vue shadowbox, Pending/Liked tabs, read-only TuningAdjustments with diff highlighting, Like/Dismiss/Promote actions, SideBug badge + Filters row entry; Promote clones source card with suggestion adjustments (share code cleared, "(Updated)" name), opens in EditCardModal immediately — 2026-07-05
- Per-section default collapsed/expanded: `defaultOpen` on section types, set at modal save time, seeds CardView openState — 2026-07-05
- Card accent override: `accentOverride` field on Card, three preset color dots in CardMeta edit mode — 2026-07-05
- RecipeSection one-way data flow refactor: replaced loop-prevention flags with resetToken prop — 2026-07-05
- Recipe section gate: hide Tune / Build Parts bar in view mode when tuneName, shareCode, upgrades, and adjustments are all empty — 2026-07-05
- Code quality pass 2: formatShareCode dedup in EditCardModal, querySelector<HTMLElement> typing, mutateDirty Set helper, closeTopModal() priority-ordered Escape handling (fixed 3 missing modals), installedParts computed cache in RecipeSection — 2026-07-05
- Code quality pass 1: formatShareCode util, collectOrphans helper, watcher consolidation, String() coercion removal — 2026-07-05
- TuningAdjustments — gearing-only lock/unlock, navigation scope fix, tire pressure unlock — 2026-07-05
- ThemeBuilder effects sliders: picker opacity + color swatches for both glass surfaces — 2026-07-04
- TuningAdjustments — Define Stock confirm dialog + in-session Cmd+Z undo snapshot — 2026-07-04
- All 5 ta-prompt-strip dialogs converted to centered glass modal (ta-trans-modal-backdrop) — 2026-07-04
- CLAUDE.md + docs/plan.md housekeeping, session AARs — 2026-07-04
- Transmission/gearing system (locked sliders, glass picker modal, auto-add/remove upgrades, view+edit parity, legacy name migration) — 2026-07-04
- Car identity: `cars` table, CarPicker, PhotoDetail, alt text — 2026-07-04
- Submit Tune feature: suggest bar, submit modal, backend submissions table + admin queue — shipped earlier, plan entry was stale
- Suggest bar two-surface redesign — 2026-07-04
- Theme builder + ColorPicker (palette drag-reorder, title bar, HSL namer, DrawerPanel) — ~2026-06-30
- Card history UI (CardHistoryModal) — earlier session
- Card migration tool (YAML export/import) — earlier session
- CSS variable rename: `--gold` → `--accent` — earlier session
- Add-photos SVG icon (thumbs rail add button) — resolved
- Initial backfill of card data fields — done, another round expected
