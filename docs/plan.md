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

### 1. Image migration pipeline — full re-file + rename

The `ImageMigrationModal` assign flow needs to physically re-file existing images, not just update metadata.

**New folder structure (all uploads, new and migrated):**
```
/uploads/{card-title-slug}_{card-id}/
         └── Lowres_Assets/
```

**New filename scheme:**
```
{FH5|FH6}_{Make}_{Model}_{Year}_{LiveryName}_{NNN}_{YYYYMMDD}_{uuid6}.jpg
```
All components derived from DB lookups at upload/migrate time.

**Pipeline per batch in ImageMigrationModal (assign flow):**
1. Car required gate — if no car selected, blocking popover forces car pick before proceeding
2. For each selected image: read existing file from disk, run through upload pipeline with new naming/folder, regenerate thumbs, insert new `images` row, delete old row, move old file + thumbs to `/uploads/bin/`
3. Create livery record linked to car
4. Card re-saved pointing to new image rows
5. AI color assess fires on livery; result stored on livery row

**Parts:**
- [ ] **Backend A**: Refactor upload handler — new folder scheme (`{slug}_{id}`), new filename stem (car/livery DB lookups)
- [ ] **Backend B**: `POST /api/admin/images/migrate` — re-files selected image IDs with car_id/livery_id, moves old files to bin
- [ ] **Frontend A**: Car required gate in ImageMigrationModal (blocking popover, can't assign without car)
- [ ] **Frontend B**: `assignSelected` calls migrate endpoint instead of just setting metadata; progress feedback

**New uploads** (NewCardModal batch import) also get the new folder + filename scheme via Backend A.

---

### 2. Car identity model — shakedown + backfill
All 12 build steps shipped (2026-07-05). Remaining items before this is fully live:
- **Step 2 (deferred)**: `car_colors` scrape — factory color options per car. Requires finding a source and scraping Forza wikis.
- **Backfill**: Existing cards have no `livery_id` / `tune_id` set (lazy migration). Needs manual tagging pass via PhotoDetail livery picker.
- **Step 8 hardening**: `CardVariant.liveryId` + `tuneId` are optional; tighten to required once liveries are linked to cards and RecipeSection resolves from stores.
- **AI color assess UI**: `POST /api/admin/liveries/:id/assess-color` is built but no admin UI button yet. Needs a trigger in the livery management UI.

---

## Parked

### Mobile layout
- Theme builder flyout + general catalog narrow-screen pass.
- Deferred until identity/livery model lands — that affects card layout.

---

## Maintenance


### Backfill pass (another round coming)
- Card data was brought in line once. Expect another round after card accent, tuning gate, and other structural changes land.
- Done in-app via EditCardModal; revisit when the dust settles.

---

## Recently completed

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
