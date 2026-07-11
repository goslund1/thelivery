# CLAUDE.md — Livery Catalog

Guidance for working in this repository. Read this before making changes.

## What this is

A catalog of Forza car liveries. It was **migrated from a single 17 MB HTML file**
(now in `archive/livery_catalog_edited.html`, kept locally / gitignored) into a
data-driven app:

- **`frontend/`** — Vue 3 + Vite + TypeScript + Pinia SPA.
- **`backend/`** — Rust (Axum) + SQLite (SQLx) API; in production it also serves the built SPA.
- **`tools/extract/`** — one-time Node script that parsed the original HTML into seed data + images.
- **`deploy/`** — systemd unit, deploy script, and Caddy/TLS setup for the DigitalOcean droplet.

Live at **https://thelivery.silverleaf.services** (Caddy terminates TLS, reverse-proxies to the backend).

## The data model is the center of everything

One `Card` object (see `frontend/src/types.ts`) drives the DB rows, the API, the
Pinia store, and the components. The entity is **"card"** — a generic catalog
entry; the codebase is a generic card-gallery, not Forza-specific (only the UI
display copy still says "Livery"). A card's content is an **ordered, type-
dispatched `sections` array**, so new section types can be added without changing
the schema:

```ts
interface Card {
  id: string                 // "1".."6", "legend"
  catalogNumber: number
  name: string
  subtitle: string
  isFavorite: boolean
  isLegend: boolean          // the "legend" template card
  collections: string[]      // e.g. FH5, FH6, Drift, Street, Photo Safari
  tags: string[]
  images: CardImage[]        // lead/feature image === order 0 (no isLead flag)
  sections: Section[]
}

interface CardImage {
  id: number          // images table PK — stable across file moves; negative temp while pre-upload
  path: string        // resolved server-side from images table on every read
  thumbPath?: string
  stagePath?: string
  alt?: string
  order: number
  carId?: string | null
  liveryId?: number | null
}

type Section =
  | { type: 'text'; key; label; body; figurePath? }          // Inspiration, Design Notes
  | { type: 'forza_recipe'; key; label; tuneName; shareCode;
      coreSpecs: Record<string,string>;
      upgrades: { category; parts[] }[];
      adjustments: { name; description }[] }
```

Sections render through a dispatcher in `CardView.vue` (`v-if` on `section.type`
→ `TextSection.vue` / `RecipeSection.vue`). The section `key` (`inspiration`,
`notes`, `recipe`) is a stable slug used for the section filter and the dom id.
To add a section type: extend the `Section` union, the extractor, and the
`CardView` dispatcher. The backend stores the whole card as JSON, so it needs no
change. When adding a plain field, thread it through `types.ts` → the component;
it persists automatically.

## Running locally

Two processes. From the repo root:

```bash
# Terminal 1 — backend on :8787 (creates/seeds data.db on first run)
cd backend && cargo run

# Terminal 2 — frontend dev server on :5173 (proxies /api and /uploads to :8787)
cd frontend && npm install && npm run dev
```

Open http://localhost:5173. Vite's proxy (`frontend/vite.config.ts`) forwards `/api`
and `/uploads` to the backend, so the app always uses same-origin relative URLs.

**Reset to clean seed data:** stop the backend, `rm backend/data.db`, restart.

## Build / typecheck / verify

- **Frontend:** `npm --prefix frontend run build` runs `vue-tsc -b && vite build`. This is the typecheck gate — run it after frontend changes.
- **Backend:** `cd backend && cargo build`. For the production target: `cargo build --release --target x86_64-unknown-linux-musl` (needs `musl-tools`; set `CC_x86_64_unknown_linux_musl=musl-gcc`).
- **No automated test suite exists.** Verify by building, by curling the API, and by the production-simulation pattern (run the release binary with `FRONTEND_DIR`/`UPLOADS_DIR`/`SEED_PATH`/`DATABASE_PATH` set and curl `/`, `/api/cards`, `/uploads/...`).
- `frontend/shot.mjs` is a Playwright screenshot helper, but headless Chromium needs system libs (`libnspr4`, etc.) that require root to install — it won't run here without that.

## Backend (`backend/`)

- **Axum + SQLx (SQLite).** Single file: `backend/src/main.rs`.
- **Storage model:** one row per card in the `cards` table — `id`, `catalog_number`, and `body` (the full `Card` JSON). The `images` table is the **single source of truth** for image data; card body stores only `{ id, alt, order, carId }` per image — **no paths**. On every card read, `inject_images()` replaces body["images"] with the full rows from the DB (path, thumbPath, stagePath, livery_id, etc.). On every card write, `sync_card_images()` upserts the images table from the body and strips paths before saving. `normalize_bodies()` step 3 migrates legacy cards at startup (idempotent).
- **Endpoints:** `GET/PUT/POST/DELETE /api/cards[/:id]`, `POST /api/images` (multipart upload, accepts `livery_id` field → returns `{ id, path, ... }`), `GET /api/cars` (search), `GET/POST /api/liveries`, `POST /api/admin/liveries/:id/assess-color` (auth-gated, calls Claude with thumbnail, stores primary/secondary color), `GET /api/health`, static `/uploads/*`, and (production) the SPA at everything else.
- **Serving the SPA:** `ServeDir::new(FRONTEND_DIR).not_found_service(ServeFile::new(index.html))`. Real files (index, hashed assets) serve at 200; unknown paths return index.html with a 404 status — acceptable because the app has **no client-side router** (only `/` is a real entry point). Don't "fix" this with `ServeDir::fallback` — that broke static serving entirely in this tower-http version.
- **Config via env** (set by the systemd unit in prod):
  - `BIND_ADDR` (default `0.0.0.0`; prod `127.0.0.1`), `PORT` (default `8787`)
  - `DATABASE_PATH`, `UPLOADS_DIR`, `SEED_PATH`, `FRONTEND_DIR`
- **Seeding:** on first run, if `cards` is empty, it imports `backend/seed/cards.json`. A startup `normalize_bodies()` migrates any old-shape rows (the `liveries`→`cards` rename + inspiration/designNotes/recipe → `sections[]`, dropping `isLead`) — idempotent.
- **Migrations:** SQLx migrations in `backend/migrations/`. **Never edit an applied migration — always add a new one** (`sqlx migrate add <desc>`). This matches the global rule about migrations.

## Frontend (`frontend/src/`)

### Stack & layout
- Vue 3 `<script setup>` SFCs, Pinia stores, TypeScript. Entry: `main.ts` (registers Pinia + the global `v-tip` directive).
- **State lives in Pinia, never in the DOM** (the original was the opposite). Two stores:
  - `stores/cards.ts` (`useCardsStore`) — the `Card[]` data + mutations + API calls (`load`, `save`, per-card snapshots).
  - `stores/ui.ts` — theme, text size, edit mode, expand/collapse, filters, which modal is open, and the **per-card dirty set**.

### CSS — important
- **All styling is one global stylesheet, `src/styles/catalog.css`, copied verbatim from the original HTML.** Components reuse those exact class names. **Do not rename classes or convert to scoped styles** — visual parity depends on the global rules. Scoped `<style>` is only for genuinely new bits (e.g. the per-card save button, unsaved-count).
- **Themes:** `data-theme` on `<html>` swaps ~35 CSS variables; 5 themes (dark/light/rainbow/clouds/stormy). Two live knobs: `--text-delta` (text scaling) and `--dissolve` (crossfade).

### Float panel system
Every floating surface (modal, drawer) uses a two-class naming convention — a structural class alongside the legacy visual class:
- `float_[instance]_backdrop` — the fixed dim/blur overlay (e.g. `float_admin_backdrop`)
- `float_[instance]_panel` — the interactive dialog surface (e.g. `float_admin_panel`)
- `drawer_[instance]_panel` — slide-out drawer surface, no backdrop (e.g. `drawer_theme_panel`)

The shared structural CSS in `catalog.css` gives all `float_*_panel` and `drawer_*_panel` elements `overflow-y: auto` and `overscroll-behavior: contain`. The global scroll guard (`composables/useScrollGuard.ts`, wired in `App.vue`) intercepts wheel events at the document level. The guard's logic:
1. Walk up from the event target looking for a `float_*_panel` or `drawer_*_panel` class — if none found, do nothing (body scrolls freely).
2. Walk up from the target to the panel boundary looking for a scrollable element (`scrollHeight > clientHeight`), including the boundary element itself.
3. If a scrollable element is found: **do not call `preventDefault`** — let native scroll handle it (preserves trackpad momentum/inertia). Only call `preventDefault` at scroll edges to prevent overscroll chaining to the body.
4. If no scrollable element is found: call `preventDefault` to block the scroll from reaching the body.

This means: scroll always follows the pointer. Native browser scroll handles the panel surface (full momentum); the guard only steps in to block leakage.

**GOTCHA — `float_*_panel` on the right element:** the class must go on the element that is (or contains) the actual scroll container. If the backdrop IS the scroll container (unusual — only SuggestionViewer does this), put `float_suggestions_panel` on `sv-body` (the inner scroll area), not on the backdrop. The backdrop gets `@wheel.self.prevent` instead to block body scroll when the pointer is on the dark overlay area.

**When adding a new floating surface:** add `float_[instance]_backdrop` to the overlay div and `float_[instance]_panel` to the inner dialog div. No other wiring needed — the scroll guard picks it up automatically.

Legacy class names (e.g. `image-picker`, `ch-backdrop`) are kept alongside the new ones and marked with `<!-- TODO: remove legacy class ... -->` comments. Remove them once the float_ system is fully adopted and visually verified.
- **Edit-only affordances** (chip add/remove, lead-star, change-image, contenteditable styling) are `display:none` until `body.editing-mode` — so render them in markup always; the `ui.isEditing` watcher toggles the body class.

### Component tree
`App.vue` → `SideBug` (+ `Filters` slot), `EditBar`, and a `v-for` of `CardShell`/`CardView` (all filtered cards always mounted — see Card list rendering below), plus global modals (`Lightbox`, `ChipPicker`, `ImagePicker`, `ExitConfirmModal`, `CustomTip`).
`CardView` → `CardMeta`, `Gallery`, `TagCloud`, then a `CollapsibleSection` per `card.sections` entry, dispatched by type to `TextSection` / `RecipeSection`. Reusable: `EditableText`.

### Edit mode + per-card persistence
- **Per-card save:** each card shows a Save button in edit mode (`CardMeta`). `ui.saveCard(id)` PUTs that one card and clears its dirty flag. There is no global "save all" button; the exit prompt handles saving on the way out.
- **Dirty tracking is per card** (`ui.dirtyIds: Set<string>`). `CardView` `provide`s a `MarkDirtyKey` (`keys.ts`) bound to its id; descendant editors (notably `EditableText`) `inject` and call it. Components that already have the card id call `ui.markCardDirty(id)` directly. The global pickers (`ChipPicker`/`ImagePicker`) use the id from their context (`ui.chipPicker`/`ui.imagePicker`).
- **Snapshots are per-card baselines** (`cards.ts`): entering edit mode snapshots every card; saving a card refreshes *its* baseline; "Discard and Exit" reverts each card to its baseline — so a card you already saved is not rolled back.
- `EditableText` writes content to the DOM imperatively (not via `{{ }}`) so typing never resets the caret; it syncs external changes back in only when not focused.

### Slideshow (`composables/useSlideshow.ts` + `Gallery.vue`)
- **Autoplay only while the card is ≥50% visible** (IntersectionObserver): resumes on enter, suspends on exit. Manual pause is **sticky** (`userPaused`) — pausing or clicking a thumbnail/stage keeps it paused across scrolling until you press play.
- **Reveal/dissolve choreography:** on entering view the play button shows "Autoplaying" over a grounded progress bar, then slow-fades out (`BUTTON_REVEAL` → `BUTTON_FADE`) as autoplay starts; the `.stage.settled` CSS class drives button visibility.
- **GOTCHA — thumb rail:** keep the active thumbnail in view by setting `thumbs.scrollLeft` directly. **Do not use `scrollIntoView`** — `block:'nearest'` walks up the scroll chain and jumps the whole page (this caused a real "page won't stop jumping" bug across the multiple autoplaying galleries).

### Custom tooltips (`composables/tooltip.ts` + `CustomTip.vue`)
- One shared tooltip element + a global `v-tip` directive. It drawer-slides open (width 0 → content width), snaps shut before reopening for a new target, and closes on scroll — all via imperative DOM + `requestAnimationFrame`, ported from the original. `v-tip` takes a string or a `() => string` (evaluated on each hover for live state like favorited/theme/expand).

### Card list rendering

All filtered cards are always mounted — a plain `v-for` over `visibleCards` with no lazy mounting or virtual scroll. `visibleCards` (the Pinia filter computed) already bounds what's shown; at current catalog scale the memory cost is negligible.

**Why not virtual scroll:** `vue-virtual-scroller` (used previously) pools and recycles component slots as if they're stateless DOM nodes. Vue components aren't. Two failure modes emerged: slot recycling resets `<script setup>` state, and the pool can assign the same card to two concurrent slots (triggered by card height changes), causing display state to snap back when the pool swaps which slot is active. Both required module-level singleton workarounds (`stackedState.ts`, `variantState.ts`) that added permanent complexity to `TuningAdjustments` and `RecipeSection`.

**`<script setup>` is per-instance.** Everything inside `<script setup>` runs inside `setup()` per component mount — `const x = ref(0)` is recreated fresh for each mount. With all cards always mounted, each card has exactly one instance and this is never a problem.

**`scrollToCardId()`** — provided from `App.vue` and injected wherever needed. Uses `getElementById` + `getBoundingClientRect()` in a `requestAnimationFrame` to scroll to the card's current position.

**If the catalog grows to hundreds of cards:** the right tool is CSS `content-visibility: auto` (browser-native — skips paint for off-screen elements, preserves layout, keeps component state intact) combined with `contain-intrinsic-size` for the initial height estimate. A `useCardVisibility.ts` composable (IntersectionObserver + KeepAlive) also exists in `composables/` as a foundation for lazy mounting if needed. Do not re-introduce `vue-virtual-scroller`.

### CSS overflow-x: auto implies overflow-y: auto

Setting `overflow-x: auto` on an element implicitly promotes `overflow-y` from `visible` to `auto` (CSS spec: the two overflow axes cannot have one `auto` and the other `visible`). This makes the element a vertical scroll container unexpectedly, causing bounce/swipe behavior on trackpads. Always pair with an explicit `overflow-y: hidden` when horizontal-only scroll is intended.

### vue-tsc gotcha
String template refs (`ref="x"`) aren't counted as "used" by `vue-tsc`'s unused-locals check. When a composable needs an element ref, create it in the component and **pass it into the composable** (so it's read in script) — see `Gallery.vue` passing `stageRef`/`barRef`/`toggleRef` into `useSlideshow`.

### Imports must be at the top of `<script setup>`
An import placed after `defineProps`/`defineEmits` silently breaks Vite HMR for that file — code changes on disk have no visible effect even after a hard refresh, because the server is handing out a stale transform. The fix: move the import to the top, then restart the dev server (`npm run dev`).

### `e.preventDefault()` on `mousedown` blocks focus
Calling `e.preventDefault()` on a `mousedown` event blocks the element from receiving keyboard focus — not just the default interaction you intended to suppress. Whenever you use this pattern (suppressing a range input's jump-to-position, blocking a drag-start, etc.), manually call `.focus({ preventScroll: true })` on the element that should own keyboard events next. See `onSliderMouseDown` in `TuningAdjustments.vue` for the reference implementation.

### `focusedKey` ≠ DOM focus (TuningAdjustments)
In `TuningAdjustments.vue`, `focusedKey` is a reactive ref that drives the visual highlight ring. It has nothing to do with `document.activeElement`. Setting `focusedKey` without also ensuring a DOM element inside that row has focus produces a highlighted row that ignores all keyboard events — arrow keys fall through to the browser's scroll behavior.

### Props are a separate reactive graph from the Pinia store
When a parent passes a deep-cloned reactive object as props (e.g. `RecipeSection` receiving a `local` recipe copy), **never bypass those props to read the same data from a store**. The clone and the store diverge immediately on first edit — the store still holds the pre-edit value. Read from props; flush to store explicitly. This is why `TuningAdjustments` reads `upgrades` and `coreSpecs` from props, not from `cards.byId()`.

### Multi-column layout decision framework
- **CSS grid** — when items in different columns share a common baseline or you want aligned rows across columns.
- **CSS `columns`** — when each column should be independently tall. Use `break-inside: avoid` to keep blocks intact; `break-before: column` to force a new column at a specific block. Works only when all blocks are similar in height — `column-fill: balance` will still break a block that exceeds the target column height.
- **Explicit column divs + JS height balancing** — when blocks vary wildly in height (one may be 3× taller than another) or you need a guaranteed no-break. Assign blocks to the shortest column with a greedy algorithm; each column is an independent `flex-direction: column` container. See `tweakColumns` computed in `TuningAdjustments.vue` and `.up-picker` in `UpgradesPicker.vue` for both patterns.

## Images

- Files live under `backend/uploads/`, served at `/uploads/*`.
- **`images` table is the single source of truth** — `id INTEGER PRIMARY KEY AUTOINCREMENT`, `card_id`, `path`, `thumb_path`, `stage_path`, `car_id`, `alt_text`, `sort_order`, `livery_id`. Card body JSON stores only `{ id, alt, order, carId }` — no paths. Paths are resolved server-side by `inject_images()` on every card read.
- `POST /api/images` (multipart) accepts `file`, `card_id`, and optionally `livery_id`; resizes to thumb + stage variants; inserts an `images` row immediately; returns `{ id, path, thumbPath, stagePath }`. `id` is the integer PK — always use it as the stable image identifier, never the path.
- `CardImage.id` on the frontend is a `number` (DB PK). The only valid negative value is a temp id (`--_imageIdCounter`) used in `addImageToPool` for the brief window before the upload response arrives. All component logic (`Gallery.vue`, `ImagePicker.vue`, `PhotoDetail.vue`, `cards.ts`) treats image id as `number`.
- The seed images were decoded from the original HTML's base64. **The data URIs claimed `image/png` but the bytes are JPEG** — `tools/extract/extract.mjs` sniffs magic bytes for the real extension. Re-run extraction only if needed (`cd tools/extract && npm run extract`); it reads `archive/livery_catalog_edited.html` (kept locally, not in the repo).
- `backend/uploads/` is **tracked in git** (the seed set), so a fresh clone runs as-is. Production deploys **never overwrite** `data.db` or uploaded images (seed images are copied in only if missing).

## Deployment

- **Push to `main`** triggers `.github/workflows/deploy.yml`: builds the static musl backend + the frontend, bundles them with seed data + the systemd unit, rsyncs to the droplet, and runs `deploy/remote-deploy.sh` (idempotent installer).
- In production the **single binary** serves API + `/uploads` + SPA on `127.0.0.1:8787`; **Caddy** (installed once via `deploy/setup-caddy.sh <domain>`) terminates TLS on 443 and reverse-proxies to it, auto-renewing the Let's Encrypt cert.
- Required GitHub secrets: `DEPLOY_SSH_KEY`, `DEPLOY_HOST`, `DEPLOY_USER` (passwordless sudo). The droplet runs the service as a dedicated `thelivery` user under `/opt/thelivery`.
- After a push, you can confirm the live build updated by checking the asset hash in `frontend/dist/index.html` against `https://thelivery.silverleaf.services/`.

## Feature status

### Shipped and working
- **Card gallery** — full-page scrolling catalog, 16:9 slideshow with autoplay (IntersectionObserver), thumbnail rail, lightbox
- **Edit mode** — inline `EditableText` for name/subtitle/sections, per-card Save button, dirty tracking, snapshot/discard on exit, `ExitConfirmModal`
- **New card modal** (`NewCardModal.vue`) — photo upload (drag/drop + browse), staged thumbnail strip, feature-image selection, tag/collection pickers, full RecipeSection (tune + specs + upgrades + adjustments). **Batch import flow:** when photos are staged, a photo setup row appears (CarPicker + livery name input — must be changed from default to unlock Import). Clicking Import: creates card, creates livery, launches all uploads in parallel via XHR (`uploadImageWithProgress` with `liveryId` field), shows a per-file progress log with a CSS linear-gradient bg bar (`--prog` custom property per row), appends a "Color assess" row that fires `assessLiveryColor()` after first upload resolves, then fades the whole log and closes after assess settles.
- **Edit card modal** (`EditCardModal.vue`) — same section parity as card edit view: `CollapsibleSection` headers, textareas for Inspiration/Design Notes, full `RecipeSection` for recipe with Cancel-safe snapshot/restore
- **Recipe section** (`RecipeSection.vue`) — tune name, share code (auto-formatted), 5-column spec table with dropdowns, `UpgradesPicker` (add/remove parts by category), Show Stock toggle, upgrade cost tally, preset system (save/apply/delete via localStorage), adjustments list (view/inline edit)
- **Orphan image cleanup** — auto-wired into `save()` (deleted images on card = orphan delete on save); also available on-demand via Admin panel
- **Account panel** (`UserSettingsModal.vue`) — collapsible Change Password form, collapsible Add User form (admin only), "Admin Panel →" button (admin only), Sign Out. Intentionally minimal; all admin tooling lives in the separate Admin panel.
- **Admin panel** (`AdminPanel.vue`) — separate modal launched from Account panel. Two tabs: **Tools** (Image Migration launch, Repair Figure Paths, System Stats, Orphan scan + sweep to trash, Trash viewer with restore/permanent-delete, Seed export + reload); **Export Card** (YAML download per card, YAML import as new card, Legacy Repair section at bottom for one-time category/adjustment-row fixes). Tune Suggestions entry point stays in the Filters flyout only.
- **User management** — login (JWT), logout (clears token + exits edit mode), change password, create users (admin only), sign-out button (redlight style)
- **Theme system** — 5 themes (dark/light/rainbow/clouds/stormy) via `data-theme` on `<html>`; text-size knob; both persist to localStorage
- **Filters** — by collection, tag, search text, livery color (15 taxonomy values from `COLOR_TAXONOMY`), and tune type (SideBug flyouts). `isCardVisible(card)` is the single gate; color axis looks up liveries linked to the card's images; tune type axis checks recipe sections.
- **Favorites** — per-card star toggle, persisted to DB
- **Upgrade presets** — save/apply/delete named upgrade configs via localStorage (per-browser, not per-card)
- **DB sync workflow** — Admin → Export Seed → git push → production Admin → Reload from Seed (no SSH/Geoff required for content pushes)
- **Card history** — per-card version list, structured diff (sliders, upgrades, specs, text), one-click restore; accessed via History button in EditCardModal top-right (`CardHistoryModal.vue`)
- **Tuning adjustments** (`TuningAdjustments.vue`) — full per-tab slider UI. **Transmission/gearing system:** `frontend/src/data/fh_transmissions.json` lists 11 transmissions with `name`, `group`, `gears`, and `tier` (`none`/`sport`/`race`/`drift`). `viewTransmissionId` ref (initialized from upgrades or defaults to "Stock 5-Speed") drives `viewTransmissionTier` computed, which controls ALL lock states in both view and edit mode — `buildGearRows()` always uses `viewTransmissionTier.value` with no mode branch. Locked sliders are interactive (opacity 0.28, no `pointer-events:none`); dragging or nudging a locked gear slider opens a glass picker modal (Teleport to body, `.ta-trans-modal-backdrop`/`.ta-trans-modal`). Final Drive dialog lists all transmissions, defaults to Sport Transmission; gear-count slider dialog lists Stock + Drift + Race options, defaults to Race 6-Speed. Confirming a non-stock transmission emits `implied-upgrades` to auto-add it; returning all values to stock auto-removes it (`autoAddedPart` ref + `checkGearingStock()`). `checkImplied()` and `checkGearingStock()` run in both view and edit mode; `flush()` (save to store) is gated to edit mode in RecipeSection. `LEGACY_TRANS_NAMES` map in `defaultViewTransmission()` normalizes old stored names (e.g. "Race Transmission" → "Race 6-Speed Transmission"). Suggest bar capped to one instance via module-level singleton (`suggestState.ts`); dismiss `×` on suggest overlay. Suggest bar uses the two-surface vertical drawer pattern: secondary (message + tab, `ta-suggest-drawer`) is a clear glass pane (35% `glass-bg`) sitting 4px inset each side above the primary smoked glass bar (`ta-suggest-strip`); tab is `position:absolute; bottom:0` so it never shifts during height transition; no divider line when expanded.
- **SideBug** — car key button inverts colors (gold bg, panel icon) when edit mode is active
- **Theme builder** (`ThemeBuilder.vue` + `ColorPicker.vue`) — launched from SideBug → Theme flyout → Customize. Three-panel layout: left picker wing (slides in, contains ColorPicker), center toggle tab, right list panel. Sections: Base ambiance (5 presets), Effects (glass opacity slider, picker opacity slider, card jump duration slider), Main palette (7 colors), Advanced (panelWell + steelLight), Tuning palette (9 colors). The Card Jump slider controls `ThemeEffects.scrollDur` (ms, default 250); the right-hand value field is the editable slider max (default 1000, session-only — resets on close, not persisted). Picker wing and tab share a lighter glass surface (`pickerBg` computed in ThemeBuilder script from `theme.current?.colors.panel` at 0.18 opacity). Right panel uses standard `var(--glass-bg)`. Theme store persists to backend; `applyAll()` sets CSS vars on `document.documentElement` at load and on every change. `effects.glassOpacity` drives `--glass-opacity`; `applyColors()` drives all `--*` color vars.
- **ColorPicker palette** — unified FH built-ins + user swatches in a single `palette` ref (`cp-palette` localStorage key). Draggable via pointer events (trackpad-safe); live bump reorder with TransitionGroup FLIP + double-rAF cooldown to prevent flicker; dragged swatch shows gold glow ring. Add-swatch dialog with color info and name input; remove button on hover (user swatches only). Palette scroll area fills remaining wing height (`flex: 1`), `overscroll-behavior: contain` prevents page scroll bleed.
- **ColorPicker title bar** — Oswald all-caps swatch name above the gradient. Clicking a swatch anchors the name; drifting sliders shows a gold `+` deviation marker; mini swatch (20×20) resets to anchor on click when deviated; `×` deselects. When no swatch selected, shows a live HSL-generated color name: 3-zone model (achromatic s<5%, tinted neutral s<50% with 7×5 hue×lightness lookup table, saturated) — names dark near-neutral colors with precision: Dark Warm Grey, Dark Slate, Dark Cool Grey, Dark Grey-Green, etc.
- **DrawerPanel pattern** — reusable slide-out drawer for deep controls. Two layers: `composables/useDrawer.ts` (pure open/close state, no markup — portable to any project) and `components/DrawerPanel.vue` (Thelivery-styled preset: glass surface, backdrop blur, 0.22s width slide, tab strip with `‹` chevron that flips on open, scroll containment via `v-scroll-contain`). Props: `open` (v-model), `width` (default 272px), `tabWidth` (default 14px), `background` (optional glass tint override). Slots: `#header` (pinned strip above body), `#default` (scrollable body), `#tab` (custom tab label, defaults to `‹`). First consumer: `ThemeBuilder.vue` ColorPicker wing. To add another drawer anywhere in the app, drop in `<DrawerPanel v-model:open="...">` and slot in the controls.

- **Two-surface drawer design principle** — core visual language for all collapsible panels in this app. A panel that can expand/collapse always uses two physically distinct surfaces with a clear visual parent/child hierarchy: (1) the **primary surface** is always visible and houses the persistent controls (toggle, action buttons); (2) the **secondary/child surface** is visually subordinate — less opaque — and houses the collapsible content (message text, detail controls). The secondary attaches flush to the primary's shared edge with no border between them (remove the border on whichever side they meet), and slides open/closed with a 0.22s ease transition. The toggle that controls the secondary lives on the secondary surface itself (as its tab/handle strip), not on the primary. Use theme CSS variables for all color/opacity values — never hardcode. **Orientation rules**: (a) **Horizontal drawer** (e.g. DrawerPanel, ColorPicker wing): the secondary can be slightly narrower in height because the tab is a vertical side strip — the depth reads naturally. Width transitions. (b) **Vertical drawer** (e.g. suggest bar): the secondary must be the **same width** as the primary — narrowing it creates misaligned edges and broken corners. Height transitions. Visual subordination comes from transparency alone, not narrowing. The tab handle must be `position:absolute; bottom:0` anchored — flex layout fails when the wing has padding that forces a minimum height. **Examples**: ThemeBuilder ColorPicker wing (horizontal, slides left, secondary slightly inset) + list panel (primary); suggest bar message drawer (vertical, slides up, same width, clear glass) + button strip (primary, smoked glass).

- **Car identity** — `cars` table (FH5 + FH6 models, seeded from `backend/seed/cars.json`), backend migration `0008_cars.sql`, `/api/cars` endpoint (search by game+query, up to 50 results), `stores/cars.ts` singleton. `CarPicker.vue`: [+ FH5]/[+ FH6] game-gated buttons → search input → results dropdown → chip display; emits `update:carId`. Wired into `RecipeSection` (view badge + edit picker), `CardView` (threads carId, handles update via `cardsStore.setCarId()`), `EditCardModal` (snapshot/restore on Cancel), `NewCardModal`. `PhotoDetail.vue`: full-size photo shadowbox (Teleport to body), prev/next nav, per-photo CarPicker + LiveryPicker + alt text input; launched via ⤢ button on `ImagePicker` thumbs. Tagging a livery in PhotoDetail auto-triggers `assessLiveryColor()` — inline assess log shows livery name → "assessing…" → "Gold / Black", fades after 2s. This is the per-photo edit/fix path; bulk tagging uses the import flow or `ImageMigrationModal`.
- **Livery identity** — `liveries` table (`id`, `car_id`, `name`, `primary_color`, `secondary_color`), `/api/liveries` endpoint, `stores/liveries.ts`. `LiveryPicker.vue`: filtered by carId, shows livery names as a dropdown chip. `livery_id` on `images` table rows links a photo to its livery.
- **AI color assessment** — `POST /api/admin/liveries/:id/assess-color` (auth-gated). Loads `thumb_path` (falls back to `path`) from the livery's linked images, sends to Claude claude-haiku-4-5-20251001 with a prompt constraining the answer to `COLOR_TAXONOMY` values. Updates `primary_color` / `secondary_color` on the livery row. Returns `{ primary, secondary }`. Frontend: `api.assessLiveryColor(id)` in `api.ts`.
- **Image migration tool** (`ImageMigrationModal.vue`) — admin-only (SideBug → Filters → "Image Migration"). Walks every non-legend card that has images one at a time. Per card: thumbnail grid (click to select), CarPicker (required before assign), livery name input. Assign: creates livery → calls `POST /api/admin/images/migrate` (physically re-files images with structured naming) → updates DB paths → AI color assess. Assigned images dim to 0.2 opacity (derived from `liveryId` presence — persistent across modal reopen). "Images Migrated" overlay appears on grid when all assigned; Enter key advances to next card; Prev/Next nav buttons in lower-left. **Toast drawer** (Migration Log): frosted glass side panel on the right edge of the modal. Starts collapsed. Slides open automatically when Assign fires (content appears with the motion). Auto-closes after all toast items have faded. Uses `--glass-bg/blur/border` CSS vars to match DrawerPanel. AI quota/429 errors surface as "AI quota exceeded / retry later" rather than silent skip.
- **Structured image filename scheme** — `{GAME}_{make}_{model}_{year}_{livery}_{NNN}_{YYYYMMDD}_{uuid6}_{WxH}.jpg`. Card folder: `{card-name-slug}_{card-id}/` (e.g. `smokin_1/`). Old files move to `uploads/trash/`. Live filename preview in modal header shows real date, `XXX` for series number.
- **Formula Drift cars** — all 16 FD cars have `make="Formula Drift"` and unique `code` fields in both DB and `backend/seed/cars.json`. Model format: `CarName #NNN` (team names in parentheses if needed: `Nissan Z #64 (Forsberg Racing)`). `next_livery_serial()` fallback is djb2 hash (prevents collisions when `code` is missing). `CarPicker` search: typing `fd...` expands prefix to `formula drift` so `fd 117` finds the 599D. Clicking the car chip label re-enters search mode pre-filled with the make.

- **Tune Suggestion Viewer** (`SuggestionViewer.vue`) — admin-only panel (launched from Filters flyout badge count). Fetches all suggestions via `GET /api/admin/suggestions`. Two tabs: Pending / Liked. Dropdown selector cycles between suggestions on the active tab; auto-advances to the next entry after Dismiss or Like (which moves the card to the other tab). Actions: Like (toggleable; moves pending → liked), Promote (calls `cardsStore.promoteCard()` to fork the card with the suggested adjustments), Dismiss (removes from list).

  Layout is four fixed zones stacked in the modal flex column — header (card name + car), controls (tabs + dropdown), infobar (tune title, credit, date, action buttons — never scrolls), scrollable body (TuningAdjustments read-only widget). This keeps the identity and action buttons always visible regardless of how long the tuning widget is. The float panel class (`float_suggestions_panel`) sits on `sv-body`, not the backdrop, so the scroll guard targets only the scrollable zone.

  **Bg scroll-to-card:** Switching suggestions scrolls the background page so the associated card is visible behind the glass. `CardView.vue` renders `<div class="card" :id="\`card-${card.id}\`">` — `suggestion.cardId` maps directly to this anchor. A `watch(current, ...)` in SuggestionViewer runs `scrollToCard(cardId)`: a custom rAF ease-in-out cubic animation (duration read from `--scroll-dur` CSS var, default 250ms) scrolling `window` to `el.getBoundingClientRect().top + window.scrollY`. Do **not** use `scrollIntoView` — its duration is not controllable, and `block:'nearest'` walks the scroll chain and causes the thumb-rail page-jump bug. The `--scroll-dur` CSS var is set by `applyEffects()` in `stores/theme.ts` from `ThemeEffects.scrollDur` and is user-controllable via the Card Jump slider in ThemeBuilder → Effects.

- **RecipeSection** (`RecipeSection.vue`) — fully refactored to emit `update:recipe` instead of mutating props directly. Local reactive copy + `flush()` pattern; loop-prevention flags (`skipNextPropsSync`, `inPropsSync`) prevent watch cycles. All four callers (CardView, EditCardModal, NewCardModal, and the component itself) handle the emit correctly.
- **Card migration tool** (Migrate tab in UserSettingsModal) — upgrade category normalization (auto), free-text adjustment row migration (manual per-card form with tab defaults), YAML export/import. Export downloads a human-readable `.yaml` file; import parses, previews, and POSTs as a new card via `crypto.randomUUID()` + `max(catalogNumber) + 1`. Images excluded from YAML; header comment notes original count. Uses `js-yaml` (v5).
- **Upgrades ↔ Tuning Link** — `impliedUpgrades()` and `applyImpliedUpgrades()` wired into RecipeSection; auto-populate indicator in UpgradesPicker (`impliedPartNames` computed). Springs and Dampers dialog fires once per session when alignment/springs/damping slider moves off-stock with no S&D entry. `SLIDER_UPGRADE_MAP` defined in `src/constants/tuning.ts`.

### Pending / in progress

See `docs/plan.md` for the current work list. High-level categories:
- **Livery backfill** — use `ImageMigrationModal` (admin, SideBug → Filters → Image Migration) to walk through cards and tag them. Smokin card done; remaining cards still need the migration pass.
- **AI quota notification** — when `assess-color` returns 429/quota error the toast shows a message, but there's no proactive alert to Jason. Pending: backend should send a real notification (email or push) when quota is hit or balance is critically low. Add `NOTIFY_WEBHOOK` env var; fire a POST when error is caught in the assess endpoint.
- **migrated_at marker** — flag images after successful re-file so they don't re-appear in the migration queue. Deferred until first full backfill pass is complete.
- **AI assess admin UI** — `POST /api/admin/liveries/:id/assess-color` is built but no trigger button in the livery management UI yet.
- **Step 2 (car_colors)** — factory color options per car; requires scraping Forza wikis.
- **Step 8 hardening** — `CardVariant.liveryId` + `tuneId` currently optional; tighten to required once backfill is complete.
- **Mobile layout** — theme builder flyout + general narrow-screen pass; deferred.
- **Multi-car mashup card** — plan doc at `docs/plan-multi-car-mashup.md`. Foundation already live (`images` table + per-photo carId); next: `variants` array on `ForzaRecipeSection` + tab strip UI in RecipeSection + gallery carId filtering.

## Conventions & rules

- **Git:** never auto-commit; commit only when asked (this repo's branch is `main`). End commit messages with the `Co-Authored-By` trailer.
- **Staging before committing:** always run `git status` before any commit to see the full picture of modified and untracked files. The codebase often has inter-dependent files in flight (types, api, components, backend) — committing only the files you touched and leaving the rest behind will break CI. Stage everything that's part of the same feature together.
- **Migrations:** new migration files only; never edit applied ones.
- **Don't break visual parity:** keep `catalog.css` and its class names intact; the original single-file app in `archive/` is the reference to diff against.
- After frontend changes, run `npm --prefix frontend run build` (typecheck) before considering it done; after backend changes, `cargo build`.
- Keep edits same-origin and relative (`/api`, `/uploads`) — never hardcode hosts/ports in the frontend.
