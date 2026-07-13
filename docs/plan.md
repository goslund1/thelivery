# Ongoing Work List

Living to-do file for thelivery. Update this when items are started, completed, or deprioritized.

---

## Active — ordered by priority

### 1. New card flow — guided import UX (in progress)

The new card modal currently shows everything at once like a static form. The goal is a guided wizard feel matching the migration tool's panel-style step-through. Two sub-items remain:

- **Import panel**: when photos are staged and the user hits "Import →", a focused panel should float over the modal showing thumbnail grid + car/+IMG selection + livery name + progress log. Mirrors ImageMigrationModal's guided-per-card experience. This is the natural container for the card-name-first → photos → car/refimg → import sequence.
- **Figure section pickers** (inspiration/notes): currently bare `<input type="file">`. Should show the card's image pool (gallery + refimg images already uploaded) via `ImagePicker`, with "upload new" triggering the RefImg pipeline. Currently figure images uploaded here bypass the structured pipeline entirely.

---

### 2. Finish CarTabs implementation (in progress)
See `docs/plan-cartabs-tunetabs.md` for the full action plan covering steps 1–4.
CarTabs wizard and tab strip UI are built. The following gaps remain before the mashup feature is shippable:

- ~~**Figure image near recipe**~~ — done. Lead image for the active carId appears before the tune name (48px tall, hover shows 200px preview, click opens lightbox). Tune name falls back to `'YY Model` when empty in multi-car mode. Share code is click-to-copy in view mode.
- **Discipline preset values** — the wizard walks through applying a preset per car, but the presets themselves have no actual base slider values yet. The names (Race / Rally / Drift / Street / etc.) exist; the values don't. These need to be filled in with Jason's established starting points before the wizard is useful.
- ~~**NewCardModal multi-car detection**~~ — done. After a successful photo import, modal pauses and shows "Done / + Add another car" instead of auto-closing. Each additional round creates a new livery on the same card. RecipeSection's auto-propose banner handles tab setup once 2+ carIds are present in the photos.
- ~~**Shakedown pass**~~ — done. Two bugs found and fixed: (1) spurious Springs dialog on tab switch (`_inPropUpdate` + `flush:'sync'` in TuningAdjustments); (2) tune-name invisible when empty in edit mode (placeholder prop on EditableText). Save/restore round-trip, discard, tab deletion, single-car cards unaffected — all confirmed. — 2026-07-12

---

### 2. Car → Tunes hierarchy (design revision — supersedes either/or mode)
The original "cars mode vs tunes mode" design (implicit from data shape, flat `variants[]` array) is replaced with a proper two-level hierarchy:

```
Card
  └─ Car (0 to many, identified by carId)
       └─ Tune (1 to many per car, named: Race / Rally / Drift / etc.)
            └─ Slider tab group (Alignment, Gearing, Aero, etc.)
```

**Why this is better:** Every car can have multiple tunes. A single-car card with one tune is just the degenerate case (1 car, 1 tune). A Smokin-style mashup is 3 cars × N tunes each. The either/or framing was an artificial constraint.

**Data model change required:**
- Current: flat `variants[]` array on the recipe, mode inferred from whether carIds are distinct
- New: `cars[]` where each car entry has its own `tunes[]` array. Each tune carries a name + the full recipe payload (upgrades, adjustments, specs, share code). The existing slider tab groups (Alignment, Gearing, etc.) sit inside each tune — unchanged structurally, just nested one level deeper.

**TuneTabs — new module, parallel to CarTabs:**
- CarTabs is the UI for navigating between cars on a card. TuneTabs is the UI for navigating between tunes within a single car.
- Both use the same folder-tab visual pattern. TuneTabs is a new but similar module — same tab strip component, scoped to a car context rather than the card.
- CarTabs wizard (already built) handles the car-level setup. A TuneTabs authoring flow handles adding/naming tune variants per car — triggered from within a car tab, not at the card level.
- A single-car card with one tune renders no tab strips at all (current default behavior, unchanged).

**Implementation order:**
1. Finish item 1 (current CarTabs shakedown) first — validate the car-level layer before adding tune nesting
2. Design the `cars[]` data shape and migration path from the current `variants[]` flat array
3. Build TuneTabs UI and authoring flow as a parallel module to CarTabs

**Backend migration note:**
The `variants[]` array lives in the card's JSON body, so no new SQL migration is needed. However, `normalize_bodies()` in `backend/src/main.rs` will need a new step to reshape existing `variants[]` flat arrays into the `cars[]` nested structure. Smokin is currently the only card with variants, so the migration surface is small — but it must go through `normalize_bodies()` (idempotent, runs on startup) rather than a one-time manual DB patch. See CLAUDE.md → "Seeding" for the normalize_bodies pattern.

**Deferred — community tune scrape + compare:**
- Park until app is live and primary content is established.

---

### 3. AI low-balance alert — discuss and plan
Before the catalog scales up and image imports become frequent, we need a proactive warning when Anthropic credit is running low — not just a toast when it's already gone.

**Questions to answer before building:**
- Does the Anthropic API expose a balance or usage endpoint queryable with an API key? If yes, the backend can poll it on a schedule and fire an alert at a threshold. If no, the only trigger is catching a 429/quota error in the assess endpoint.
- What's the notification target? Options: email (needs SMTP or a mail API), push (needs a webhook service like Pushover or ntfy), or both via a `NOTIFY_WEBHOOK` env var the backend POSTs to.
- What threshold triggers the alert — a fixed dollar amount, a percentage of last top-up, or just "first time we hit a 429"?

**Rough shape once answers are settled:**
- `NOTIFY_WEBHOOK` env var in the systemd unit
- Backend fires a POST on quota error AND (if balance API exists) on a scheduled low-balance check
- No frontend changes needed

See Maintenance → AI billing notification for context.

---

### 4. Mobile layout
Narrow-screen pass for the full catalog. Known gaps:
- Theme builder flyout — doesn't fit on small screens
- General card layout on narrow viewports
- Deferred until now; no blocking dependencies remaining

---

## Maintenance

### Pre-launch checklist
- **Lock CORS to production domain** — currently `CorsLayer::permissive()` in `backend/src/main.rs`. Change to `CorsLayer::new().allow_origin("https://thelivery.silverleaf.services")` before public launch.
- **Update README.md** — significantly out of date: still references `/api/liveries` (now `/api/cards`), `seed/liveries.json` (now `seed/cards.json`), "single-user, no auth" (JWT auth exists), and is missing most current endpoints. Rewrite the API table and data description to match reality before the repo goes public.

### Backfill pass (another round coming)
- Card data was brought in line once. The next structural change is the Cars→Tunes hierarchy refactor (active item 2) which will reshape card body data again. Hold off on a full backfill pass until that data model stabilizes — otherwise it's two passes.
- Done in-app via EditCardModal; revisit when the dust settles.

### AI billing notification (planned, lower priority)
- See active item 3 for the discussion and planning work needed before this gets built.
- When `assess-color` hits a 429 or quota error, the toast already surfaces it in-app. What's missing is a *proactive* low-balance alert before hitting zero.
- Implementation shape is roughly settled (NOTIFY_WEBHOOK env var, POST on quota error + optional scheduled balance check) but key questions remain — see active item 3.

### Deferred
- **`car_colors`** — factory *stock* color options per car model (e.g. "this Corvette comes in Arctic White, Rapid Blue..."). Requires scraping Forza wikis. Not to be confused with `primary_color`/`secondary_color` on the `liveries` table, which is the AI livery color assessment already wired into the import flow. No ETA on car_colors.

---

## Recently completed

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
