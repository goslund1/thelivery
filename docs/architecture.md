# Architecture Reference

This document deconstructs the Livery app as a framework — not just a fan site. The Forza-specific content (card data, livery images, tune recipes) is a skin over a generic card-gallery engine with editorial tooling, a theming system, and an admin OS. This reference documents the repeatable patterns so new features stay on-pattern, and so this scaffolding can be adapted to other catalogs without reinventing the wheel.

---

## 1. The Framework Layer

### What the app actually is

The Livery app has two layers:

**Domain layer (Forza-specific):**
- Card content: livery photos, tune recipes, car identity
- Section types: `text` and `forza_recipe`
- Vocabulary: "livery", "tune", "share code", "car"

**Framework layer (generic card-gallery engine):**
- Card data model and type-dispatched sections array
- Edit mode with per-card dirty tracking and snapshot/discard safety
- Image upload, storage, and hydration pipeline
- Float panel system (modals, drawers, scroll guard)
- Two-surface drawer design principle
- CSS variable theming system (5 themes, 35+ variables)
- Admin OS: orphan cleanup, trash, seed export/import, history viewer
- JWT auth with single-user-per-tenant model
- AI integration point (assess-color endpoint)

### What to change to adapt this to a new catalog

1. **Content**: Replace `ForzaRecipeSection` with domain-specific section types. `TextSection` is already domain-agnostic.
2. **Vocabulary**: Update display copy (component labels, UI strings). The codebase entity is `card` — only display copy says "Livery".
3. **Seed data**: Replace `backend/seed/cards.json` and `backend/seed/cars.json`.
4. **Domain tables**: The `cars`, `liveries`, `tunes`, `tune_types` tables are Forza-specific. New domains add their own entity tables via migrations.
5. **Unchanged**: Everything in the framework layer above — stores, CSS system, edit mode, modal system, image pipeline, admin tools, auth.

---

## 2. The Card Data Model

The `Card` interface (`frontend/src/types.ts`) drives the DB rows, the API, the Pinia store, and the components. Everything else is built around it.

```ts
interface Card {
  id: string                  // stable slug: "1".."6", "legend"
  catalogNumber: number       // sort key
  name: string
  subtitle: string
  isFavorite: boolean
  isLegend: boolean           // the "legend" template card
  collections: string[]       // e.g. FH5, FH6, Drift, Street
  tags: string[]
  images: CardImage[]         // lead image = order 0; no isLead flag
  sections: Section[]         // ordered, type-dispatched
  accentOverride?: string     // per-card CSS color override
  carId?: string              // FK to cars table
}
```

### sections[] — the primary extensibility point

`sections` is an ordered array of typed objects. The backend stores it as opaque JSON — no schema validation, no migration needed for new types. The frontend dispatches on `section.type` in `CardView.vue`.

Current types:
```ts
type Section = TextSection | ForzaRecipeSection

interface TextSection {
  type: 'text'
  key: string          // stable slug: 'inspiration', 'notes'
  label: string
  body: string
  figurePath?: string
  defaultOpen?: boolean
}

interface ForzaRecipeSection {
  type: 'forza_recipe'
  key: string          // 'recipe'
  label: string
  tuneName: string
  shareCode: string
  coreSpecs: Record<string, string>
  upgrades: UpgradeCategory[]
  adjustments: AdjustmentRow[]
  variants?: CardVariant[]   // multi-car/multi-tune tabs
}
```

The `key` is a stable slug used for section filtering and DOM IDs. Adding a section type requires four steps and no backend changes — see [§9 Extension Recipes](#9-extension-recipes).

### images[] — stored thin, hydrated on read

Card body JSON stores only `{ id, alt, order, carId }` per image — no paths. The `images` table is the single source of truth for all file locations. On every card read, `inject_images()` (backend) replaces the body's images array with full rows from the DB, adding `path`, `thumbPath`, `stagePath`, and `liveryId`.

This means:
- Files can be renamed or moved without touching card JSON
- The same image can be re-associated to a different card without duplication
- `CardImage.id` is always the integer DB primary key — never the path

---

## 3. Frontend Architecture

### Component conventions

All 41 components use `<script setup lang="ts">`. No Options API. No mixins. No render functions.

```ts
<script setup lang="ts">
import { ref, computed, provide, inject } from 'vue'
import type { Card } from '../types'
import { useCardsStore } from '../stores/cards'
import { useUiStore } from '../stores/ui'

const props = defineProps<{ card: Card }>()
const emit = defineEmits<{ 'update:card': [Card] }>()
const store = useCardsStore()
const ui = useUiStore()
```

Scoped `<style>` only for genuinely new CSS not in `catalog.css`. Never rename a global class or move it to scoped — visual parity depends on the global rules.

### Pinia store architecture

**Two primary stores** — everything flows through these:

`stores/cards.ts` (`useCardsStore`) — the catalog data:
- `cards: Card[]` — in-memory catalog; the single source of truth for all card state
- `snapshots: Record<string, string>` — per-card JSON baselines for discard safety
- Mutations: `save(id)`, `deleteCard`, `createNewCard`, `promoteCard`, image management (`setLeadImage`, `reorderImages`, `removeImage`, `addImageToPool`), field setters (`toggleFavorite`, `setCarId`, `setAccentOverride`), section editing (`setFigure`)
- History: `restoreCardVersion(id, version)`

`stores/ui.ts` (`useUiStore`) — global UI state only:
- `isEditing: boolean` — edit mode flag; all edit affordances depend on this
- `dirtyIds: Set<string>` — which cards have unsaved changes
- Modal open flags: `exitConfirmOpen`, `legendConfirmOpen`, etc.
- Edit list: `editCount`, `currentEditIndex` — focus management across contenteditable fields
- Theme and text scaling: `theme`, `textDelta`

**Eight secondary stores**: `auth`, `filters`, `modal`, `theme`, `cars`, `liveries`, `tunes`, `tune-types`, `toasts` — all follow the same pattern: reactive `ref()` fields + simple mutation functions.

**Rule: state lives in Pinia, never in the DOM.** Components read from store, call mutations, emit events. No component-local state for data that other components need.

### Provide/inject for per-card context

`CardView.vue` provides two tokens (defined in `src/keys.ts`) to all its descendants:

```ts
// keys.ts
export const MarkDirtyKey: InjectionKey<() => void> = Symbol('markCardDirty')
export const CardIdKey: InjectionKey<string> = Symbol('cardId')

// CardView.vue
provide(MarkDirtyKey, () => ui.markCardDirty(props.card.id))
provide(CardIdKey, props.card.id)
```

All editor components — `EditableText`, `TuningAdjustments`, `UpgradesPicker`, `RecipeSection`, `SubtitleEditor` — inject `MarkDirtyKey` and call it on change. This avoids prop-drilling dirty callbacks through potentially deep component trees.

### Composables

**`useScrollGuard.ts`** — installed once in `App.vue` onMounted. Intercepts wheel events at the document level. Walks up from the event target looking for a `float_*_panel` or `drawer_*_panel` class. If found: only calls `preventDefault` at scroll edges (to prevent overscroll chaining to body); otherwise lets native scroll handle it. If no panel found: body scrolls freely. No per-component wiring needed — adding the right class to a panel element is sufficient.

**`useSlideshow.ts`** — per-gallery instance. Manages autoplay: resumes when card is ≥50% visible (IntersectionObserver), suspends on exit. Manual pause is sticky (`userPaused`) — survives scroll until the user presses play. Reveal choreography: "Autoplaying" label shows and fades as autoplay starts. Thumb rail kept in view via direct `scrollLeft` — never `scrollIntoView` (which walks the scroll chain and causes page-jump bugs).

**`useDrawer.ts`** — pure state machine: `{ open, toggle, openDrawer, closeDrawer }`. No markup, no styling. Callers provide the visual layer (either via `DrawerPanel.vue` or inline).

**`onClickOutside.ts`** — pointerdown listener on mount; cleanup on unmount. Used by `Filters.vue` and `SideBug.vue` to dismiss flyouts.

**`tooltip.ts`** — singleton tooltip element registered by `CustomTip.vue` at mount. The `v-tip` directive (registered globally in `main.ts`) calls `showTip/hideTip`. Accepts string or `() => string` (evaluated on hover for live state). Width-animates open (0 → natural width); snaps shut before reopening.

---

## 4. The CSS System

### Single global stylesheet

`frontend/src/styles/catalog.css` (2100+ lines) is the only place global styles live. Component files import it via `main.ts`. No scoped styles for any class that traces back to the original app.

Adding a new component:
- Reuse existing global class names where they fit
- Add new classes to `catalog.css` if they need to be shared
- Add scoped `<style>` only for component-specific overrides that will never apply elsewhere

### CSS variable system

All colors and effects flow through CSS custom properties. Hard-coded color values in component files are a bug.

Theme variables (set on `html[data-theme="..."]`):
```css
/* Base surfaces */
--base, --panel, --panel-edge, --panel-well, --steel-light

/* Text */
--fg, --muted, --muted-light, --ink, --chip-ink

/* Accent */
--accent, --accent-bright, --accent-chip
--accent-tint-04, --accent-tint-06, --accent-tint-14  /* color-mix blends */

/* Highlight (secondary accent) */
--highlight, --highlight-tint-06, --highlight-tint-40

/* State */
--danger, --danger-bright, --success, --success-unlit

/* Glass surfaces */
--glass-opacity, --glass-bg, --glass-blur, --glass-border

/* Tuning tab colors (snapped to FH in-game palette) */
--tabc-tires, --tabc-gearing, --tabc-alignment, --tabc-antiroll,
--tabc-springs, --tabc-damping, --tabc-aero, --tabc-brakes
```

Live knobs (set programmatically, not per-theme):
- `--text-delta` (px) — text size offset; set by ui.ts watcher
- `--dissolve` (s) — crossfade transition duration
- `--scroll-dur` (ms) — card jump animation duration; set by `stores/theme.ts` `applyEffects()`

Five themes are defined as `html[data-theme="dark|light|rainbow|clouds|stormy"]` selectors, each overriding the same ~35 variables.

---

## 5. The Float Panel System

Every floating surface — modal overlay, slide-out drawer — uses a two-class naming convention that the scroll guard and shared CSS structural rules depend on.

| Class pattern | Applied to | Effect |
|---------------|------------|--------|
| `float_[name]_backdrop` | Fixed dim/blur overlay div | Structural backdrop positioning |
| `float_[name]_panel` | Interactive dialog surface | `overflow-y: auto; overscroll-behavior: contain`; scroll guard target |
| `drawer_[name]_panel` | Slide-out drawer surface | Same as panel; no backdrop |

The CSS structural rule (catalog.css line ~11):
```css
[class*="float_"][class*="_panel"],
[class*="drawer_"][class*="_panel"] {
  overflow-y: auto;
  overscroll-behavior: contain;
}
```

The scroll guard (`useScrollGuard.ts`) matches these class patterns automatically. **No per-modal wiring needed** — use the right class and scroll containment works.

### How to add a new modal

```vue
<div class="float_[name]_backdrop" v-if="isOpen" @click.self="close">
  <div class="float_[name]_panel">
    <!-- content -->
  </div>
</div>
```

That's it. Scroll guard picks it up automatically.

### Legacy class migration

Older components (`EditCardModal`, `ImagePicker`, `NewCardModal`) still carry borrowed class names like `image-picker` and `ch-backdrop` alongside new `float_*` names. These are marked with `<!-- TODO: remove legacy class [name] -->`. The scroll guard handles both patterns, so there's no functional issue. Clean up when convenient.

---

## 6. The Two-Surface Drawer Design Principle

All collapsible panels use two physically distinct surfaces. This is the visual language for anything that can expand or collapse in this app.

**Primary surface** — always visible; opaque glass; houses the persistent controls (toggle, action buttons, status).

**Secondary surface** — expandable; visually subordinate (35% glass opacity); houses the collapsible content (message text, detail controls, extended UI).

Rules:
- Secondary attaches flush to primary's shared edge. Remove the border on whichever side they meet.
- The toggle that controls the secondary lives on the secondary surface itself (as its tab/handle), not on the primary.
- Both surfaces share theme CSS variables for color/opacity — never hardcode.

**Orientation matters:**

Horizontal drawers (DrawerPanel, ColorPicker wing in ThemeBuilder): secondary can be slightly narrower in height because the tab is a vertical side strip — the depth reads naturally. Width transitions.

Vertical drawers (TuningAdjustments suggest bar): secondary must be the **same width** as the primary. Narrowing creates misaligned edges and broken corners. Height transitions. Visual subordination comes from transparency alone, not narrowing. Tab handle must be `position:absolute; bottom:0` — flex layout forces minimum height and breaks the tab anchor.

**Canonical examples:**
- `DrawerPanel.vue` + `ThemeBuilder.vue` ColorPicker wing — horizontal
- Suggest bar in `TuningAdjustments.vue` (`ta-suggest-drawer` above `ta-suggest-strip`) — vertical (predates DrawerPanel; intentional custom implementation)

---

## 7. The Edit Mode System

Edit mode is a global toggle managed by `ui.ts`. Everything that changes in edit mode flows from one watcher:

```ts
watch(isEditing, (on) => document.body.classList.toggle('editing-mode', on))
```

### Edit affordances (CSS gate)

Edit-only UI is hidden via CSS, not conditional rendering:
```css
.chip-add, .lead-star, .change-image, .edit-action-row {
  display: none;
}
body.editing-mode .chip-add,
body.editing-mode .lead-star { display: inline-flex; }
```

**Why CSS instead of `v-show`:** Render the edit affordances in markup always; the CSS gate shows/hides them. This avoids churning the DOM on mode toggle and makes the pattern declarative.

### Per-card dirty tracking

```ts
// Mark a card dirty
ui.markCardDirty(card.id)

// Check
ui.isCardDirty(card.id)  // returns boolean

// Clear on save
ui.clearCardDirty(card.id)
```

All editor components get `MarkDirtyKey` from `CardView` via provide/inject and call it on any change. The dirty set drives the save button visibility and the exit confirmation prompt.

### Snapshot/discard safety

On `enterEdit()`: `cards.takeSnapshot()` — serializes every card to `snapshots[id]`.

On `save(id)`: `cards.save(id)` → PUT to API → `snapshots[id] = JSON.stringify(updatedCard)`. The baseline refreshes to the saved state. Subsequent discard won't roll back an already-saved card.

On `confirmDiscardAndExit()`: `cards.restoreSnapshot()` — reverts all cards to their baselines; any newly-uploaded images that were never saved are deleted via orphan cleanup.

### Edit mode lifecycle

```
enterEdit()
  └─ takeSnapshot() for all cards
  └─ clearAllDirty()
  └─ isEditing = true → body.editing-mode

[editing...]

requestExit() [with unsaved changes]
  └─ exitConfirmOpen = true → modal

  ├─ saveAllDirty()
  │   └─ save(id) for each dirty card in parallel
  │   └─ isEditing = false
  │
  └─ confirmDiscardAndExit()
      └─ restoreSnapshot() [reverts + orphan cleanup]
      └─ isEditing = false
```

---

## 8. The Image Pipeline

Images in this app have a strict lifecycle. Understanding it prevents inconsistencies.

### Storage hierarchy

```
backend/uploads/
  {card-name-slug}_{card-id}/
    FH6_{make}_{model}_{year}_{livery}_{NNN}_{YYYYMMDD}_{uuid6}_{WxH}.jpg  ← original
    Lowres_Assets/
      [same stem]_600x400.jpg    ← thumb (200w max)
      [same stem]_1000x667.jpg   ← stage (1000w max)
  trash/
    {uuid8}_{original_basename}  ← orphaned or user-deleted files
```

### The three key backend functions

`inject_images(card_id, body)` — called on every card **read**. Queries the `images` table by `card_id`, ordered by `sort_order`. Replaces `body["images"]` with full rows including `path`, `thumbPath`, `stagePath`, `liveryId`. This is why card body JSON never needs to store paths.

`sync_card_images(card_id, body)` — called on every card **write**. Upserts `images` table rows from the body's images array, then strips all paths from body before saving to `cards.body`. Images with a numeric `id` → UPDATE metadata. Images with a path but no `id` → INSERT or UPDATE by path lookup.

`normalize_bodies()` — idempotent startup migration. Converts old-shape cards (top-level fields) to `sections[]`. Ensures the three standard sections always exist. Syncs legacy image data to the `images` table.

### Upload flow

```
[user selects file]
  → api.uploadImage(file, cardId, carId, liveryId, fileIndex)
  → POST /api/images (multipart)
      → build_image_stem() [structured filename]
      → resize: thumb (200w), stage (1000w)
      → save 3 variants to disk
      → INSERT images row (if cardId provided)
      → return { id, path, thumbPath, stagePath }
  → store.addImageToPool(cardId, path, thumbPath, stagePath, true, dbId)
      → appends to card.images with order = maxOrder + 1
  → ui.markCardDirty(cardId)
  → [on save] sync_card_images() updates DB
```

### Orphan cleanup

When a card is saved, `collectOrphans(snapshot, current)` diffs the two image arrays and finds any images in the snapshot but not in the current card. These are queued for `DELETE /api/images`, which moves them to `uploads/trash/` and logs to `trash_log`. Admin can restore from trash via the Admin panel.

When a card is discarded, the same logic runs in reverse — finding images added since the baseline (newly uploaded files that were never saved) and deleting them.

### `CardImage.id` is always the DB primary key

The `id` field on `CardImage` is the integer PK from the `images` table. The only valid negative value is a temp ID (`--_imageIdCounter`) used briefly between `addImageToPool()` call and the upload response arriving. All code treats image `id` as `number`.

---

## 9. Extension Recipes

### Add a new section type

1. **types.ts** — Add an interface and extend the union:
   ```ts
   export interface TimelineSection {
     type: 'timeline'
     key: string
     label: string
     events: Array<{ date: string; description: string }>
     defaultOpen?: boolean
   }

   export type Section = TextSection | ForzaRecipeSection | TimelineSection
   ```

2. **Create `TimelineSection.vue`**:
   ```vue
   <script setup lang="ts">
   import type { TimelineSection } from '../types'
   defineProps<{ section: TimelineSection }>()
   </script>
   <template><!-- render section.events --></template>
   ```

3. **CardView.vue** — Add to the section dispatcher:
   ```vue
   <template v-else-if="section.type === 'timeline'">
     <TimelineSection :section="(section as TimelineSection)" />
   </template>
   ```

4. **Import** in CardView:
   ```ts
   import TimelineSection from './TimelineSection.vue'
   ```

Backend: no changes. The API stores sections as opaque JSON.

### Add a new floating modal

```vue
<!-- In your component template -->
<Teleport to="body">
  <div class="float_[name]_backdrop" v-if="isOpen" @click.self="isOpen = false">
    <div class="float_[name]_panel">
      <!-- content -->
    </div>
  </div>
</Teleport>
```

Scroll guard picks it up automatically. Add open/close state to `stores/modal.ts` if other components need to trigger it; keep it local if only one component opens it.

### Add a slide-out drawer

Use `DrawerPanel.vue`:
```vue
<DrawerPanel v-model:open="showPanel" :width="280">
  <template #header>Panel Title</template>
  <template #default>
    <!-- scrollable body content -->
  </template>
  <template #tab>‹</template>
</DrawerPanel>
```

Or use `useDrawer()` composable for custom markup:
```ts
const { open, toggle } = useDrawer(false)
```

### Add a new API endpoint

1. **backend/src/main.rs** — Add handler function:
   ```rust
   async fn my_handler(
     State(st): State<AppState>,
     _auth: AuthUser,              // ← omit for public endpoints
   ) -> Result<Json<Value>, ApiError> {
     // ...
     Ok(Json(json!({ "result": value })))
   }
   ```

2. Register the route in the `Router::new()` block at the bottom of `main.rs`:
   ```rust
   .route("/api/my-endpoint", post(my_handler))
   ```

3. **frontend/src/api.ts** — Add client method:
   ```ts
   async myEndpoint(data: MyRequestType): Promise<MyResponseType> {
     return json('/api/my-endpoint', {
       method: 'POST',
       headers: authHeaders(),
       body: JSON.stringify(data),
     })
   }
   ```

### Add a new Pinia store

```ts
// stores/my-store.ts
import { ref, computed } from 'vue'
import { defineStore } from 'pinia'

export const useMyStore = defineStore('my-store', () => {
  const items = ref<Item[]>([])
  const loading = ref(false)

  async function load() {
    loading.value = true
    items.value = await api.listItems()
    loading.value = false
  }

  function mutate(id: string, value: string) {
    const item = items.value.find(i => i.id === id)
    if (item) item.field = value
  }

  return { items, loading, load, mutate }
})
```

### Add a database migration

Never edit an existing migration file. Generate a new one:
```bash
cd backend && sqlx migrate add my_description
```

Edit the new file in `backend/migrations/`. Use `CREATE TABLE IF NOT EXISTS` for new tables. Use `ALTER TABLE ... ADD COLUMN` for additive changes. Never destructive in a migration.

---

## 10. Backend Reference

### API endpoint inventory (abbreviated)

**Public (no auth):**
- `GET /api/health` — heartbeat
- `POST /api/login` — issues JWT `{ token, username }`
- `GET /api/cards` — list all non-deleted cards (images hydrated)
- `GET /api/cards/:id` — single card
- `GET /api/cards/:id/history` — version list
- `GET /api/cards/:id/history/:version` — version snapshot
- `GET /api/cars` — list cars (`?game=FH5|FH6`)
- `GET /api/liveries` — list liveries (`?carId=...`)
- `GET /api/tunes`, `GET /api/tune-types` — tune data
- `GET /api/theme` — current theme JSON
- `GET /api/tuning-presets` — saved presets
- `POST /api/suggestions` — submit tune suggestion (rate-limited: 3/hour/IP)

**Authenticated:**
- `POST/PUT/DELETE /api/cards[/:id]` — card CRUD
- `POST /api/images` — multipart upload; returns `{ id, path, thumbPath, stagePath }`
- `DELETE /api/images` — moves to trash; body: `{ paths: string[] }`
- `POST/PUT/DELETE /api/cars[/:id]`, `/api/liveries[/:id]`, `/api/tunes[/:id]`
- `PUT /api/theme` — update theme
- `POST/DELETE /api/tuning-presets[/:id]`
- `PUT /api/me/password`, `POST /api/users`

**Admin (authenticated, `/api/admin/...` prefix):**
- `GET /api/admin/stats` — counts and disk usage
- `GET/DELETE /api/admin/orphans` — scan/sweep orphaned files to trash
- `GET/DELETE /api/admin/trash` — list/permanently-delete trash
- `POST /api/admin/trash/restore` — restore from trash to original path
- `POST /api/admin/export-seed` — write cards to seed file
- `POST /api/admin/reload-seed` — reload from seed file
- `POST /api/admin/images/migrate` — re-file images under structured naming
- `POST /api/admin/repair-figure-paths` — patch TextSection figurePaths
- `GET/POST/DELETE /api/admin/deleted-cards[/:id]` — soft-deleted card management
- `GET/DELETE/PATCH /api/admin/suggestions[/:id]` — suggestion review
- `POST /api/admin/liveries/:id/assess-color` — AI color assessment

### Auth pattern

All mutating endpoints take `_auth: AuthUser` as the last parameter. The `AuthUser` extractor (Axum `FromRequestParts`) validates the `Authorization: Bearer <token>` header and returns 401 on failure. JWT TTL is 7 days. In production, set `JWT_SECRET` env var; without it, an ephemeral secret is generated per restart.

### Error pattern

```rust
type ApiError = (StatusCode, String);
fn err(code: StatusCode, msg: impl ToString) -> ApiError { (code, msg.to_string()) }

// All handlers return:
Result<Json<Value>, ApiError>
```

Standard codes: 400 bad request, 401 unauthorized, 404 not found, 409 conflict, 422 unprocessable, 429 rate limited, 500 internal, 502/503 external API errors.

---

## 11. Cohesion Audit

### In the global spec — consistent everywhere

| Pattern | Coverage |
|---------|----------|
| `<script setup lang="ts">` | 41/41 components |
| Pinia as single source of truth | All data; no DOM state |
| `api.ts` as single API client | No ad-hoc `fetch()` calls |
| `float_*_panel` / `drawer_*_panel` naming | New components; legacy migrating |
| Provide/inject for dirty-marking | 5/5 editor components |
| CSS variable system for theming | No hardcoded colors in components |
| Append-only SQLx migrations | 13/13 migrations |
| `inject_images()` / `sync_card_images()` on every read/write | All card endpoints |

### Intentional outliers — by design, not drift

| Location | Pattern | Why |
|----------|---------|-----|
| `EditableText.vue` | Imperative DOM writes | Contenteditable requires it; `v-model` resets caret |
| TuningAdjustments suggest bar | Custom two-surface drawer (pre-DrawerPanel) | Predates DrawerPanel component; fully intentional |
| ThemeBuilder color palette | `localStorage` instead of Pinia | Per-browser user customization; not server state |
| Direct API calls in modal/admin components | No store intermediate | Appropriate for transient uploads, admin tools, read-only views |

### In-progress migration — not bugs

| Item | Status |
|------|--------|
| Legacy modal class names (`image-picker`, `ch-backdrop`) | Being replaced with `float_*` naming; `<!-- TODO -->` comments mark each |
| Flyout pattern (SideBug, Filters) outside modal.ts | Works but not centralized; low priority to unify |

### Genuine gaps — worth normalizing eventually

| Gap | Impact |
|-----|--------|
| No error boundaries around `CardView` | A single malformed card body can break the whole gallery |
| `Filters.vue` fetches suggestions independently from `SuggestionViewer` | Duplicated API call; could share a suggestions store |
| `POST /api/cars` and `POST /api/liveries` not explicitly admin-gated | Implicit assumption (all authed users are admins); fine for single-tenant |

### Backend-specific outliers

| Item | Recommendation |
|------|----------------|
| `THEME_DEFAULT` — 1400 lines of inline JSON in `main.rs` | Move to `backend/seed/theme.json` |
| `COLOR_TAXONOMY` duplicated in prompt and validation | Extract to a shared constant |
| Suggestion IP addresses stored unmasked | Hash or truncate before public launch |
| CORS: `CorsLayer::permissive()` | Lock to production domain before public launch |

---

## 12. File Map

```
frontend/src/
  types.ts              ← Card interface, Section union, CardImage
  keys.ts               ← provide/inject tokens (MarkDirtyKey, CardIdKey)
  api.ts                ← all API calls; centralized fetch wrapper
  main.ts               ← registers Pinia, v-tip directive, mounts App
  styles/catalog.css    ← global stylesheet; single source of truth

  stores/
    cards.ts            ← catalog data, CRUD, snapshots
    ui.ts               ← edit mode, dirty set, modal flags, theme
    auth.ts, filters.ts, modal.ts, theme.ts, cars.ts,
    liveries.ts, tunes.ts, tune-types.ts, toasts.ts

  composables/
    useScrollGuard.ts   ← document-level scroll containment for all panels
    useSlideshow.ts     ← per-gallery autoplay with IntersectionObserver
    useDrawer.ts        ← pure open/close state machine
    onClickOutside.ts   ← lifecycle composable for flyout dismiss
    tooltip.ts          ← singleton tooltip + v-tip directive

  components/
    App.vue             ← root; installs scroll guard; renders cards + global modals
    CardView.vue        ← main card dispatcher; provides MarkDirtyKey + CardIdKey
    Gallery.vue         ← slideshow; DO NOT use scrollIntoView for thumb rail
    EditableText.vue    ← contenteditable wrapper; imperative DOM updates
    RecipeSection.vue   ← forza_recipe section; local reactive copy + flush()
    TuningAdjustments.vue ← slider UI; transmission/gearing system; suggest bar
    DrawerPanel.vue     ← reusable slide-out drawer; two-surface pattern
    [See components/ for full list]

backend/
  src/main.rs           ← entire backend; ~2900 lines
  migrations/           ← 0001–0013; append-only
  seed/
    cards.json          ← seed data loaded on first run
    cars.json           ← FH5 + FH6 car models
  uploads/              ← tracked in git (seed images); production adds to it
```
