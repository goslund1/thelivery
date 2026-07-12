---
name: frontend-patterns
description: Vue component patterns and subsystem behavior for the Livery Catalog frontend — floating panels, edit-mode persistence, slideshow, tooltips, card list rendering, drawer/theme system, TuningAdjustments, SuggestionViewer. Also covers build rules: data model extensibility, Pinia store split, per-card vs global decisions, CSS conventions, persistence patterns, and pre-build checklist. Load when working on these components or extending similar patterns.
---

# Frontend Patterns — Livery Catalog

## Float panel system

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

## Edit mode + per-card persistence

- **Per-card save:** each card shows a Save button in edit mode (`CardMeta`). `ui.saveCard(id)` PUTs that one card and clears its dirty flag. There is no global "save all" button; the exit prompt handles saving on the way out.
- **Dirty tracking is per card** (`ui.dirtyIds: Set<string>`). `CardView` `provide`s a `MarkDirtyKey` (`keys.ts`) bound to its id; descendant editors (notably `EditableText`) `inject` and call it. Components that already have the card id call `ui.markCardDirty(id)` directly. The global pickers (`ChipPicker`/`ImagePicker`) use the id from their context (`ui.chipPicker`/`ui.imagePicker`).
- **Snapshots are per-card baselines** (`cards.ts`): entering edit mode snapshots every card; saving a card refreshes *its* baseline; "Discard and Exit" reverts each card to its baseline — so a card you already saved is not rolled back.
- `EditableText` writes content to the DOM imperatively (not via `{{ }}`) so typing never resets the caret; it syncs external changes back in only when not focused.

## Slideshow (`composables/useSlideshow.ts` + `Gallery.vue`)

- **Autoplay only while the card is ≥50% visible** (IntersectionObserver): resumes on enter, suspends on exit. Manual pause is **sticky** (`userPaused`) — pausing or clicking a thumbnail/stage keeps it paused across scrolling until you press play.
- **Reveal/dissolve choreography:** on entering view the play button shows "Autoplaying" over a grounded progress bar, then slow-fades out (`BUTTON_REVEAL` → `BUTTON_FADE`) as autoplay starts; the `.stage.settled` CSS class drives button visibility.
- **GOTCHA — thumb rail:** keep the active thumbnail in view by setting `thumbs.scrollLeft` directly. **Do not use `scrollIntoView`** — `block:'nearest'` walks up the scroll chain and jumps the whole page (this caused a real "page won't stop jumping" bug across the multiple autoplaying galleries).

## Custom tooltips (`composables/tooltip.ts` + `CustomTip.vue`)

One shared tooltip element + a global `v-tip` directive. It drawer-slides open (width 0 → content width), snaps shut before reopening for a new target, and closes on scroll — all via imperative DOM + `requestAnimationFrame`, ported from the original. `v-tip` takes a string or a `() => string` (evaluated on each hover for live state like favorited/theme/expand).

## Card list rendering

All filtered cards are always mounted — a plain `v-for` over `visibleCards` with no lazy mounting or virtual scroll. `visibleCards` (the Pinia filter computed) already bounds what's shown; at current catalog scale the memory cost is negligible.

**Why not virtual scroll:** `vue-virtual-scroller` (used previously) pools and recycles component slots as if they're stateless DOM nodes. Vue components aren't. Two failure modes emerged: slot recycling resets `<script setup>` state, and the pool can assign the same card to two concurrent slots (triggered by card height changes), causing display state to snap back when the pool swaps which slot is active. Both required module-level singleton workarounds (`stackedState.ts`, `variantState.ts`) that added permanent complexity to `TuningAdjustments` and `RecipeSection`.

**`<script setup>` is per-instance.** Everything inside `<script setup>` runs inside `setup()` per component mount — `const x = ref(0)` is recreated fresh for each mount. With all cards always mounted, each card has exactly one instance and this is never a problem.

**`scrollToCardId()`** — provided from `App.vue` and injected wherever needed. Uses `getElementById` + `getBoundingClientRect()` in a `requestAnimationFrame` to scroll to the card's current position.

**If the catalog grows to hundreds of cards:** the right tool is CSS `content-visibility: auto` (browser-native — skips paint for off-screen elements, preserves layout, keeps component state intact) combined with `contain-intrinsic-size` for the initial height estimate. A `useCardVisibility.ts` composable (IntersectionObserver + KeepAlive) also exists in `composables/` as a foundation for lazy mounting if needed. Do not re-introduce `vue-virtual-scroller`.

## Theme builder / ColorPicker / DrawerPanel / two-surface drawer design principle

**Theme builder** (`ThemeBuilder.vue` + `ColorPicker.vue`) — launched from SideBug → Theme flyout → Customize. Three-panel layout: left picker wing (slides in, contains ColorPicker), center toggle tab, right list panel. Sections: Base ambiance (5 presets), Effects (glass opacity slider, picker opacity slider, card jump duration slider), Main palette (7 colors), Advanced (panelWell + steelLight), Tuning palette (9 colors). The Card Jump slider controls `ThemeEffects.scrollDur` (ms, default 250); the right-hand value field is the editable slider max (default 1000, session-only — resets on close, not persisted). Picker wing and tab share a lighter glass surface (`pickerBg` computed in ThemeBuilder script from `theme.current?.colors.panel` at 0.18 opacity). Right panel uses standard `var(--glass-bg)`. Theme store persists to backend; `applyAll()` sets CSS vars on `document.documentElement` at load and on every change. `effects.glassOpacity` drives `--glass-opacity`; `applyColors()` drives all `--*` color vars.

**ColorPicker palette** — unified FH built-ins + user swatches in a single `palette` ref (`cp-palette` localStorage key). Draggable via pointer events (trackpad-safe); live bump reorder with TransitionGroup FLIP + double-rAF cooldown to prevent flicker; dragged swatch shows gold glow ring. Add-swatch dialog with color info and name input; remove button on hover (user swatches only). Palette scroll area fills remaining wing height (`flex: 1`), `overscroll-behavior: contain` prevents page scroll bleed.

**ColorPicker title bar** — Oswald all-caps swatch name above the gradient. Clicking a swatch anchors the name; drifting sliders shows a gold `+` deviation marker; mini swatch (20×20) resets to anchor on click when deviated; `×` deselects. When no swatch selected, shows a live HSL-generated color name: 3-zone model (achromatic s<5%, tinted neutral s<50% with 7×5 hue×lightness lookup table, saturated) — names dark near-neutral colors with precision: Dark Warm Grey, Dark Slate, Dark Cool Grey, Dark Grey-Green, etc.

**DrawerPanel pattern** — reusable slide-out drawer for deep controls. Two layers: `composables/useDrawer.ts` (pure open/close state, no markup — portable to any project) and `components/DrawerPanel.vue` (Thelivery-styled preset: glass surface, backdrop blur, 0.22s width slide, tab strip with `‹` chevron that flips on open, scroll containment via `v-scroll-contain`). Props: `open` (v-model), `width` (default 272px), `tabWidth` (default 14px), `background` (optional glass tint override). Slots: `#header` (pinned strip above body), `#default` (scrollable body), `#tab` (custom tab label, defaults to `‹`). First consumer: `ThemeBuilder.vue` ColorPicker wing. To add another drawer anywhere in the app, drop in `<DrawerPanel v-model:open="...">` and slot in the controls.

**Two-surface drawer design principle** — core visual language for all collapsible panels in this app. A panel that can expand/collapse always uses two physically distinct surfaces with a clear visual parent/child hierarchy: (1) the **primary surface** is always visible and houses the persistent controls (toggle, action buttons); (2) the **secondary/child surface** is visually subordinate — less opaque — and houses the collapsible content (message text, detail controls). The secondary attaches flush to the primary's shared edge with no border between them (remove the border on whichever side they meet), and slides open/closed with a 0.22s ease transition. The toggle that controls the secondary lives on the secondary surface itself (as its tab/handle strip), not on the primary. Use theme CSS variables for all color/opacity values — never hardcode. **Orientation rules**: (a) **Horizontal drawer** (e.g. DrawerPanel, ColorPicker wing): the secondary can be slightly narrower in height because the tab is a vertical side strip — the depth reads naturally. Width transitions. (b) **Vertical drawer** (e.g. suggest bar): the secondary must be the **same width** as the primary — narrowing it creates misaligned edges and broken corners. Height transitions. Visual subordination comes from transparency alone, not narrowing. The tab handle must be `position:absolute; bottom:0` anchored — flex layout fails when the wing has padding that forces a minimum height. **Examples**: ThemeBuilder ColorPicker wing (horizontal, slides left, secondary slightly inset) + list panel (primary); suggest bar message drawer (vertical, slides up, same width, clear glass) + button strip (primary, smoked glass).

## Tune Suggestion Viewer (`SuggestionViewer.vue`)

Admin-only panel (launched from Filters flyout badge count). Fetches all suggestions via `GET /api/admin/suggestions`. Two tabs: Pending / Liked. Dropdown selector cycles between suggestions on the active tab; auto-advances to the next entry after Dismiss or Like (which moves the card to the other tab). Actions: Like (toggleable; moves pending → liked), Promote (calls `cardsStore.promoteCard()` to fork the card with the suggested adjustments), Dismiss (removes from list).

Layout is four fixed zones stacked in the modal flex column — header (card name + car), controls (tabs + dropdown), infobar (tune title, credit, date, action buttons — never scrolls), scrollable body (TuningAdjustments read-only widget). This keeps the identity and action buttons always visible regardless of how long the tuning widget is. The float panel class (`float_suggestions_panel`) sits on `sv-body`, not the backdrop, so the scroll guard targets only the scrollable zone.

**Bg scroll-to-card:** Switching suggestions scrolls the background page so the associated card is visible behind the glass. `CardView.vue` renders `<div class="card" :id="\`card-${card.id}\`">` — `suggestion.cardId` maps directly to this anchor. A `watch(current, ...)` in SuggestionViewer runs `scrollToCard(cardId)`: a custom rAF ease-in-out cubic animation (duration read from `--scroll-dur` CSS var, default 250ms) scrolling `window` to `el.getBoundingClientRect().top + window.scrollY`. Do **not** use `scrollIntoView` — its duration is not controllable, and `block:'nearest'` walks the scroll chain and causes the thumb-rail page-jump bug. The `--scroll-dur` CSS var is set by `applyEffects()` in `stores/theme.ts` from `ThemeEffects.scrollDur` and is user-controllable via the Card Jump slider in ThemeBuilder → Effects.

## TuningAdjustments — transmission system (`TuningAdjustments.vue`)

Full per-tab slider UI. **Transmission/gearing system:** `frontend/src/data/fh_transmissions.json` lists 11 transmissions with `name`, `group`, `gears`, and `tier` (`none`/`sport`/`race`/`drift`). `viewTransmissionId` ref (initialized from upgrades or defaults to "Stock 5-Speed") drives `viewTransmissionTier` computed, which controls ALL lock states in both view and edit mode — `buildGearRows()` always uses `viewTransmissionTier.value` with no mode branch. Locked sliders are interactive (opacity 0.28, no `pointer-events:none`); dragging or nudging a locked gear slider opens a glass picker modal (Teleport to body, `.ta-trans-modal-backdrop`/`.ta-trans-modal`). Final Drive dialog lists all transmissions, defaults to Sport Transmission; gear-count slider dialog lists Stock + Drift + Race options, defaults to Race 6-Speed. Confirming a non-stock transmission emits `implied-upgrades` to auto-add it; returning all values to stock auto-removes it (`autoAddedPart` ref + `checkGearingStock()`). `checkImplied()` and `checkGearingStock()` run in both view and edit mode; `flush()` (save to store) is gated to edit mode in RecipeSection. `LEGACY_TRANS_NAMES` map in `defaultViewTransmission()` normalizes old stored names (e.g. "Race Transmission" → "Race 6-Speed Transmission"). Suggest bar capped to one instance via module-level singleton (`suggestState.ts`); dismiss `×` on suggest overlay. Suggest bar uses the two-surface vertical drawer pattern: secondary (message + tab, `ta-suggest-drawer`) is a clear glass pane (35% `glass-bg`) sitting 4px inset each side above the primary smoked glass bar (`ta-suggest-strip`); tab is `position:absolute; bottom:0` so it never shifts during height transition; no divider line when expanded.

## RecipeSection — one-way flow (`RecipeSection.vue`)

Fully refactored to emit `update:recipe` instead of mutating props directly. Local reactive copy + `flush()` pattern; loop-prevention flags (`skipNextPropsSync`, `inPropsSync`) prevent watch cycles. All four callers (CardView, EditCardModal, NewCardModal, and the component itself) handle the emit correctly.

---

## Build rules (from DESIGN_SYSTEM.md)

### The data model is generic — don't make it Forza-specific

The whole point of the `Card` + typed `sections[]` model is that it's a generic card gallery, not a Forza tool that happens to render in a browser. Concretely:

- New **structural** content types → extend the `Section` union, the extractor, and the `CardView` dispatcher.
- New **plain fields** → thread through `types.ts` → the component. The backend stores the whole card as JSON, so no migration is needed for this.
- Resist adding Forza-specific or brand-specific fields (e.g. a dedicated `abideBadge: boolean`) when the existing generic arrays (`tags[]`, `collections[]`) already cover the need. Ask "could this be a tag?" before adding a new typed field.

### State lives in Pinia, never in the DOM

Two stores, two jobs:

- `stores/cards.ts` — the data: `Card[]`, mutations, API calls, per-card snapshots (baselines for discard-on-exit).
- `stores/ui.ts` — everything else: theme, text size, edit mode, expand/collapse, filters, which modal is open, the per-card dirty set.

New global, app-wide state → `ui.ts`. New persisted data → `cards.ts` / the `Card` shape. Don't introduce a third store unless a feature genuinely doesn't fit either bucket.

### Per-card vs global is a real architectural fork, not a style choice

| Question | If global | If per-card |
|---|---|---|
| Where does the control live? | A flyout/panel off `SideBug` | An edit-only affordance inside `CardMeta` / the edit modals |
| How does it persist? | `localStorage` (matches theme, text-size, upgrade presets) unless multi-device sync is explicitly needed | A new optional key in the `Card` JSON body, via the existing `PUT /api/cards/:id` |
| Default value | One of the existing presets | `null` / `inherit` — most cards should carry zero extra data |

Before building any new control, answer "is this one setting for the whole app, or one setting per card?" first — it decides almost everything else about where it lives and how it's saved.

### CSS — new feature rules

(See also `CLAUDE.md → ## Frontend → ### CSS — important` for the foundational rules on the global stylesheet.)

- Scoped `<style>` is only for genuinely new UI with no prior art (e.g. the per-card save button).
- Theming is `data-theme` on `<html>` swapping CSS variables — never inline style overrides for theme-able properties. If a new control needs to be theme-aware, it needs a CSS variable, not a hardcoded color.
- **Edit-only affordances are always rendered, hidden via `body.editing-mode`** (chip add/remove, lead-star, contenteditable styling all work this way). New edit controls should follow this pattern — render in markup always, gate visibility with the existing class — rather than `v-if`-mounting them only in edit mode.

### Persistence precedent — check before adding a new mechanism

There are already three different persistence patterns in use. Pick the one that matches, don't invent a fourth without reason:

1. **Per-card, server-side** — part of the `Card` JSON body, `PUT /api/cards/:id`. Use for anything that belongs to a specific card and should survive across browsers/devices.
2. **Global, client-side** — `localStorage`. Used for theme, text-size, upgrade presets. Use for app-wide preferences that don't need to follow a user across devices.
3. **Global, server-side** — doesn't exist yet for settings (only cards and users are server-side). Only reach for this if a global setting genuinely needs to sync across devices/users — it's new infrastructure, not a free choice.

### Branding marks (Abide / Victory / Smokin') are content rules, not schema

These are rules about what gets painted into the livery and where, not fields the app needs to track structurally:

- **Abide** — universal artist signature, adapted per build, usually rear.
- **Victory** — crossed-flags tuning-quality mark, front fenders, only on cars carrying Jason's custom tune.
- **Smokin'** — drift-series identity (cursive, clouds, magenta), reserved for drift-spec builds.

If the catalog needs to *display* whether a card carries these, prefer representing them as tag values in the existing `tags[]` array rather than adding dedicated fields, unless there's a clear reason to special-case them.

### Before building anything new — checklist

- [ ] Is this global or per-card? (see Per-card vs global above) — decides where it lives and how it's saved.
- [ ] Does it need a new `Section` type, or is it a plain field on the existing `Card` shape?
- [ ] Does it belong in `ui.ts` or `cards.ts`?
- [ ] Could it reuse an existing generic mechanism (`tags[]`, `collections[]`, `localStorage` presets) instead of a new bespoke one?
- [ ] If it's theme-able, is it wired through a CSS variable, not a hardcoded value?
- [ ] If it's an edit-only control, is it rendered always and hidden via `body.editing-mode`, matching every other edit affordance?
- [ ] Does it touch any of the known gotchas in `.claude/skills/frontend-gotchas/`?
- [ ] Does the persistence choice match one of the three existing patterns, or is a new one genuinely justified?
