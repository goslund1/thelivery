# Action Plan: CarTabs Shakedown → TuneTabs Implementation

This document is the handoff for the next session. Read it top to bottom before touching any code.

---

## Current state (as of session 18, 2026-07-10)

### What's built and working
- `ForzaRecipeSection.variants[]` — flat array of `CardVariant`, each holding `carId` + full inline tune data
- Tab strip UI in `RecipeSection.vue` — renders when `variants.length >= 2` in view mode, always in edit mode
- Auto-propose banner — detects distinct carIds across card images, offers one-click tab setup
- Car Tabs wizard — floating modal, walks through each non-anchor car, user picks a discipline preset per car, creates tabs with `pendingPresetId`, preset auto-applies on first tab open
- `+` button expands to Car / Tune choice
- Remove variant flow (3→2, 2→1, 1→collapse)
- Gallery `activeCarId` filtering wired

### What's NOT built yet
- ~~**Figure image near recipe**~~ — done (session 19). Lead image for active carId appears before the tune name at 48px, hover preview, click opens lightbox. Tune name falls back to `'YY Model` when empty.
- **TuneTabs** — multiple tune variants per car. Not yet built. See full section below.
- **NewCardModal multi-car detection** — lower priority, not blocking this work.

### Known data about Smokin card (the test case)
- 3 cars: 599D (anchor), FD Corvette #777, Austin Healey Sprite
- Only the 599D has real tune data; the other two have empty variants from the wizard
- This is the card to use for all shakedown testing

### Key files
- `frontend/src/types.ts` — `CardVariant`, `ForzaRecipeSection`
- `frontend/src/components/RecipeSection.vue` — all tab logic, wizard, auto-propose (~1400 lines)
- `frontend/src/components/TuningAdjustments.vue` — slider UI (~2500 lines)
- `backend/src/main.rs` — `normalize_bodies()` for data migrations

---

## Step 1: CarTabs shakedown

Run these manually on the Smokin card (dev server: `cd backend && cargo run` + `cd frontend && npm run dev`).

### Test checklist — completed 2026-07-12 (dummy data)
- [x] View mode: tab strip visible with 3 tabs (599D, Corvette, Austin Healey)
- [x] View mode: switching tabs updates the recipe section content
- [x] Edit mode: all tabs editable, dirty tracking works per-card (not per-tab)
- [x] Edit mode: save → reload → tab data round-trips correctly for all 3 variants
- [x] Edit mode: remove Corvette tab → confirm dialog → tab gone, 599D becomes active
- [x] Edit mode: remove until 1 tab → tab strip collapses, card looks like single-car card
- [x] Single-car card (any other card): no tab strip in view or edit mode
- [x] Discard: edits to variant data revert correctly on exit without save
- [x] Wizard: auto-propose banner / wizard covered in prior sessions (not re-run this pass)
- [x] Race condition guard: demonstrated with 3s fetch delay — anchor sliders unaffected, pendingPresetId deferred (see audit-2026-07-12.md)
- [ ] **Discipline preset values** — STILL EMPTY. Presets exist (Race/Rally/Drift/Street/etc.) but have no base slider values. Jason needs to supply starting-point values per discipline before the wizard is useful in practice.

**Bugs found and fixed during shakedown:**
1. Spurious Springs dialog on tab switch → `_inPropUpdate` + `flush:'sync'` in TuningAdjustments (commit 5eaae18)
2. Tune name invisible in edit mode when empty → placeholder prop on EditableText (commit 5eaae18)

---

## Step 2: Figure image near recipe

**Design decision (settled):** Each car tab shows a small figure image of that car's lead photo near the tune section header — to anchor the tune visually to the car. This is NOT gallery filtering (see `feedback_mashup_no_gallery_filter.md`). In tunes mode (single car, multiple tunes) the same car image shows across all tabs.

**Implementation:**
- In `RecipeSection.vue`, resolve the lead image for `activeVariantCarId` from `props.images` (already passed down — filter by `carId`, take `order === 0` or lowest order)
- Render a small `<img>` tag above or inline with the recipe section header when a variant is active and a matching image exists
- Should be visible in both view and edit mode
- No new props needed — `images` is already available

---

## Step 3: Data model change — `variants[]` → `cars[].tunes[]`

**Why:** The current flat `variants[]` array uses implicit mode detection (distinct carIds = cars mode, same carId = tunes mode). The new hierarchy is explicit:

```
Card
  └─ Car (0 to many)
       └─ Tune (1 to many per car, named)
            └─ Slider tab group (existing TuningAdjustments, unchanged)
```

A single-car, single-tune card has no `cars[]` entry — it uses the existing top-level recipe fields as today. The array only appears when there are 2+ cars or 2+ tunes on a car.

**New type shape (replace `CardVariant` + `ForzaRecipeSection.variants`):**

```ts
export interface CardTune {
  tuneName: string
  tuneType?: string
  shareCode: string
  coreSpecs: Record<string, string>
  upgrades: UpgradeCategory[]
  adjustments: AdjustmentRow[]
  isSuggested?: boolean
  pendingPresetId?: number   // same wizard pattern as now
}

export interface CardCar {
  carId: string
  carName?: string
  liveryId?: number
  liveryName?: string
  tunes: CardTune[]          // always at least 1 entry per car
}

// In ForzaRecipeSection:
cars?: CardCar[]             // replaces variants[]
```

**Migration — `normalize_bodies()` in `backend/src/main.rs`:**
- Add a new step that detects cards with `variants[]` in their recipe section body
- For each variant: group by `carId` → each unique `carId` becomes a `CardCar`, its variants become `tunes[]` within it
- Remove `variants[]` after reshaping
- This is idempotent (runs on every startup, skips cards already in the new shape)
- Smokin is currently the only card with variants — low migration risk

**Frontend changes:**
- `types.ts`: add `CardTune`, `CardCar`; update `ForzaRecipeSection` to use `cars?`; keep `CardVariant` as a deprecated alias until migration is confirmed
- `RecipeSection.vue`: the active selection becomes two levels — `activeCar` index + `activeTune` index within that car. The car-level tab strip (CarTabs) selects the car; the tune-level tab strip (TuneTabs, built in Step 4) selects the tune within a car.
- All existing variant logic (`activeVariantIndex`, `hasVariants`, `isMultiCar`, `isMultiTune`, auto-propose, wizard) needs to be rewritten against the new shape. This is the largest change in the file.

---

## Step 4: TuneTabs

**Design (settled):** TuneTabs is a new module parallel to CarTabs. CarTabs selects which car you're viewing; TuneTabs selects which tune for that car. TuneTabs renders only when the active car has 2+ tunes.

**Where the authoring UI lives:**
- NOT in the import flow — tune variants are added post-import
- The `+ Add tune` button lives **under the tune adjustments section header bar** (the bar that labels the TuningAdjustments section within a recipe tab)
- In edit mode only, this button adds a new `CardTune` entry to the active car's `tunes[]`

**TuneTabs vs CarTabs — similarities and differences:**

| | CarTabs | TuneTabs |
|---|---|---|
| Level | Card | Per-car |
| Tab label | Car short name | Tune name (e.g. Race, Rally) |
| Tab strip location | Top of recipe section | Below car tab, above sliders |
| Add trigger | Auto-propose or `+` Car button | `+ Add tune` button under adjustments header |
| Wizard | Yes (preset picker per car) | No — user names the tune and adjusts from discipline preset |
| Remove | Confirm dialog | Confirm dialog |
| View mode | Shows at 2+ variants | Shows at 2+ tunes on active car |

**Implementation approach:**
- Reuse the same `rs-variant-tabs` CSS as CarTabs (same folder-tab visual pattern)
- Add a `activeTuneIndex` ref alongside `activeCarIndex` in RecipeSection
- When switching car tabs, `activeTuneIndex` resets to 0
- The `+ Add tune` button in edit mode appends a new empty `CardTune` to `activeCar.tunes[]` and sets `activeTuneIndex` to the new entry
- User names the tune inline (editable tab label, same pattern as `EditableText`)
- Discipline preset: show the same preset dropdown from the wizard inline, on the new empty tune tab, to seed base slider values

**Single-car multi-tune cards:**
- No car tab strip renders (only 1 car)
- TuneTabs strip renders below the recipe header
- Functionally equivalent to the old "tunes mode" but now explicit in the data shape

---

## What NOT to do in this session

- Do not wire TuneTabs into NewCardModal or the import flow — that's a separate item
- Do not start Step 3 (data model) until Step 1 + 2 (figure image + shakedown) are complete and confirmed working
- Do not start Step 4 (TuneTabs) until Step 3 migration is validated on Smokin

---

## Reference

- `docs/plan.md` — active item 1 (CarTabs shakedown) and item 2 (Cars→Tunes hierarchy)
- `CLAUDE.md` → "Card list rendering" section — all cards are always mounted (plain v-for, no virtual scroll), so per-card display state lives normally in `<script setup>` as plain `ref()`. No module singletons needed.
- TuneTabs `activeTuneIndex` is a plain `ref(0)` in RecipeSection's `<script setup>` — same as `activeVariantIndex` after the session 19 refactor.
- `feedback_mashup_no_gallery_filter.md` in memory — figure image, not gallery filter
- `feedback_research_before_coding.md` in memory — for any new UI patterns, look it up first
