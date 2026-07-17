# Ongoing Work List

Living to-do file for thelivery. Update this when items are started, completed, or deprioritized.

---

## Active — ordered by priority

### 1. Chrome shakedown — live site
End-to-end pass on https://thelivery.silverleaf.services in a real browser. Check view mode, edit mode, sharing, OG Maker, photo manager, CarTabs/TuneTabs, and theme switching. Log anything broken or visually off.

---

### 2. Mobile layout
Narrow-screen pass for the full catalog. Known gaps:
- Theme builder flyout — doesn't fit on small screens
- General card layout on narrow viewports
- No blocking dependencies remaining

---

## Maintenance

### Pre-launch checklist
- **Lock CORS to production domain** — currently `CorsLayer::permissive()` in `backend/src/main.rs`. Change to `CorsLayer::new().allow_origin("https://thelivery.silverleaf.services")` before public launch.
- **Update README.md** — significantly out of date: still references `/api/liveries` (now `/api/cards`), `seed/liveries.json` (now `seed/cards.json`), "single-user, no auth" (JWT auth exists), and is missing most current endpoints. Rewrite the API table and data description to match reality before the repo goes public.

### Backfill pass (another round coming)
- Cars→Tunes hierarchy refactor is complete (migration 0017, normalize_bodies step). Data model is now stable.
- Ready for a backfill pass via EditCardModal when content work resumes.

### AI billing notification (deferred — wait until credits actually run out)
- When `assess-color` hits a 429 or quota error, the toast already surfaces it in-app. That's enough for now.
- If a proactive alert becomes needed: `NOTIFY_WEBHOOK` env var in the systemd unit; backend POSTs on quota error; check if Anthropic API exposes a balance endpoint to poll. No frontend changes needed.

### Deferred
- **`car_colors`** — factory *stock* color options per car model (e.g. "this Corvette comes in Arctic White, Rapid Blue..."). Requires scraping Forza wikis. Not to be confused with `primary_color`/`secondary_color` on the `liveries` table, which is the AI livery color assessment already wired into the import flow. No ETA on car_colors.

---

## Recently completed

- **Code audit pass 4** — six fixes across compositor, OG Maker, ShareModal, auth: `@input` on content field for live preview; scrim overlap check; `as any` removed from ShareModal; `list_og_presets` auth-gated; `render_glyphs` + `blit_with_transform` extracted (−75 lines); `TextStyle` enum + `OgTextStyle` union replace magic strings. Deferred: circular store import warning (`ui.ts` ↔ `modal.ts`). — 2026-07-16 (commit b3214b2, AAR: `docs/aar-2026-07-16b.md`)

- **OG overlay access fixed** — ShareModal OG design section now gated by `auth.isAuthenticated` instead of `ui.isEditing`. Admins see the overlay composer in normal user view (click card title → Share modal) — the correct flow since sharing is a post-edit action, and the title is an editable input in edit mode. Also added `/share` to the Vite dev proxy so the compositor preview and share routes work in local dev. — 2026-07-16

- **OG Overlay Studio** — server-generated 1200×630 `og:image` for social share unfurls (Discord, iMessage, Reddit). Pure-Rust compositor (`fontdue` + `image` crate): `POST /share/preview` for live OG Maker preview, `GET /share/:id/card.png` for public card image, `og_presets` table + CRUD. Three text styles: POSTCARD (Bebas Neue, floating), SIGNAL (Oswald VF + dark chyron backdrop), GHOST (Oswald VF, semi-transparent). Polish: bottom scrim gradient, per-style drop shadows, logo slot ("THE LIVERY" placeholder). OG Maker modal: freely-positioned text boxes with corner-resize, rotation handle, shear slider; 200ms debounced live preview; preset save + "Save to Card." ShareModal integration with preset picker, preview thumbnails, Adjust/Reset flow. — 2026-07-16

- **CarTabs/TuneTabs shakedown + polish**: discipline baseline presets (Race/Rally/Drift/Street) seeded in DB; `+ ADD CAR` / `+ ADD TUNE` buttons styled as dashed tabs (grey→gold/pink on hover); TuneTabs shelf color = `--highlight`; spacing and gap fixes; `v-tip-up` directive for right-flying tooltip from text end; card title opens ShareModal. — 2026-07-13
- **Social sharing foundation + Reddit pre-fill**: OG share page endpoint (`GET /share/:id/:slug`) added to Rust backend — server-rendered HTML with full OG + Twitter card tags; real browsers get `<meta refresh>` to `/`. Card title opens `ShareModal` (view mode only, `v-tip-up` tooltip). Modal: Copy Link button (clipboard API, 2s feedback) + **Post to Reddit** (pre-fill approach — opens `reddit.com/submit?url=…&title=…` in new tab with editable title pre-populated from card + car name + share code; Reddit's native form handles subreddit selection and posting) + Discord stub. Note: direct Reddit API posting blocked — Reddit ended self-service OAuth access Nov 2025 for personal projects. See `docs/plan-reddit-share.md`. — 2026-07-13

- **CarTabs shakedown + bug fixes**: spurious Springs and Dampers dialog on tab switch fixed (`_inPropUpdate` flag + `flush:'sync'` on gearCount watcher in TuningAdjustments — prevents prop-driven gearCount change from calling checkImplied); tune-name placeholder added to EditableText (CSS `::before` + `data-placeholder` attr) so empty tuneName in edit mode shows the car model name as hint text. — 2026-07-12
- **Gear preset fix + preset kind** (migration 0016): `applyPresetValues`, `executeApplyPreset`, and `saveAsPreset` all now include locked gear rows — previously skipped them, so presets saved/applied without Race Transmission had no gear values. `getAdjustments()` now includes locked rows so gear values survive the flush→rebuild cycle. `applyImpliedTransmission()` added: infers required transmission from highest `gearN` key in preset, auto-adds it via `implied-upgrades` emit (mirrors `onTransChoice` pattern). Rally preset (id=6) was saved while locked and has no gear keys — needs a re-save after adding Race Transmission. Migration 0016 adds `kind TEXT NOT NULL DEFAULT 'build'` to `tuning_presets`; Baseline checkbox in save dialog; `◆` prefix on baseline entries in dropdown. Baseline/build conflict logic (skip baseline-locked rows when applying a build preset) deferred until first real baseline preset exists. — 2026-07-11
- **RefImg pipeline** (migration 0015): `image_role` + `included` columns on images table; `included` now persists across page reloads (was ephemeral). RefImg images get structured filenames `{card_slug}_RefImg{NN}_{date}_{uuid6}`, default to excluded from slideshow, skip livery creation + color assess. CarPicker gets `+IMG` button (`showImageBtn` prop, dashed style) that emits `select-image` — no car search. NewCardModal tracks `imageRole`, shows RefImg chip, passes role to uploads. `sync_card_images` now syncs `included` back to DB on card save. — 2026-07-11
- **NewCardModal car/recipe parity**: `RecipeSection` gets `forceEdit` prop so all edit-mode fields (CarPicker, spec table, upgrades, preset bar) appear in NewCardModal without requiring global edit mode. — 2026-07-11
- Replaced `vue-virtual-scroller` with plain `v-for` (all filtered cards always mounted). Virtual scroll's slot-pool model is incompatible with stateful Vue components — two failure modes (slot recycling + concurrent duplicate slots) required module-level singleton workarounds that added permanent complexity. At catalog scale the memory cost of mounting all cards is negligible. `useCardVisibility.ts` (IntersectionObserver + KeepAlive) left in place for when the catalog is large enough to need lazy mounting. See `CLAUDE.md` → "Card list rendering" — 2026-07-10
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
