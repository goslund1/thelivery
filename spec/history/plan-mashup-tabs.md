# Plan: Variant Tune Authoring — Multi-Car Detection + Discipline Presets

## What this is

When creating a new card with photos of multiple different cars, the app should
detect the situation passively and offer a guided resolution before the card is
finalised. Resolution either sets up per-car tune tabs with discipline-based
defaults, or skips tabs and stays single-tune.

---

## Context — read before touching anything

### Data model
`ForzaRecipeSection` (see `frontend/src/types.ts`) has a `variants: CardVariant[]`
field. When `variants.length >= 2` the tab strip renders. Each `CardVariant` has:
- `carId: string` — links to the `cars` table
- `tuneName`, `shareCode`, `coreSpecs`, `upgrades`, `adjustments` — per-variant tune

Two implicit modes (no stored flag — derived from data):
- **Cars mode**: variants have distinct `carId` values → each tab = a different car
- **Tunes mode**: variants share the same `carId` → each tab = a different setup

### What's already built
- **`RecipeSection.vue`** — `autoProposeCarIds` computed detects 2+ distinct carIds
  across card images; `showAutoPropose` renders a passive banner in edit mode;
  `acceptAutoPropose()` builds `local.variants` sorted by photo count with the lead
  car first. This is the edit-mode path and works correctly.
- **Variant tab strip** — renders in `RecipeSection` when `hasVariants` is true.
  `variantLabel()` resolves car name from `carsStore`.
- **Tune preset system** — `api.listTuningPresets()` / `api.saveTuningPreset()` /
  `api.deleteTuningPreset()` store named presets (slider key → value map) in the
  DB. Applying a preset in `TuningAdjustments` patches `localRows` values. The UI
  lives in `TuningAdjustments.vue` (search `// ── Tuning presets`).
- **NewCardModal** — creates a card with one `newCarId` + one livery per import
  batch. After the import log fades the modal stays open and can accept another
  batch. Currently has no awareness of multiple carIds across batches.

### Key files
- `frontend/src/components/NewCardModal.vue` — the new card creation flow
- `frontend/src/components/RecipeSection.vue` — variant logic, auto-propose banner
- `frontend/src/components/TuningAdjustments.vue` — preset apply logic (reference)
- `frontend/src/types.ts` — `CardVariant`, `ForzaRecipeSection`
- `frontend/src/api.ts` — `listTuningPresets`, `saveTuningPreset`
- `frontend/src/stores/cards.ts` — `save()`, card shape

---

## What needs building

### Step 1 — Discipline preset base values (data, not code)

The preset system exists but has no pre-populated defaults. Before the discipline
dropdown is useful, author the base values for each discipline using the
TuningAdjustments preset save UI (edit any card with a full recipe, dial in the
base values, save as "Race", "Rally", "Drift", "Street", "Drag"). Do this first so
Step 3 has real data to show.

Discipline names to create (exact names matter — the dropdown will match on these):
- **Race** — stiff ARBs, low ride height, high camber, locked diff
- **Rally** — mid ARBs, raised ride height, moderate camber, open diff
- **Drift** — soft front ARB, stiff rear, high camber front, 100% rear diff accel
- **Street** — near-stock everything, modest camber, moderate tire pressure
- **Drag** — max tire pressure, zero camber, max rear diff accel, 0 decel

These are saved via the existing UI and stored in the DB. No code change needed.

---

### Step 2 — NewCardModal: track imported carIds, show passive button

**Goal:** after a second import batch with a different carId, a passive button
appears at the bottom of the form. It does not interrupt. The user can keep adding
more cars. The button stays visible until resolution.

**Changes to `NewCardModal.vue` only:**

Add a ref to track all carIds imported this session:
```ts
const importedCarIds = ref<{ carId: string; label: string }[]>([])
```

At the end of a successful import batch (after the log fades), push the current
carId + livery name into `importedCarIds` if not already present:
```ts
if (newCarId.value && !importedCarIds.value.some(e => e.carId === newCarId.value)) {
  const car = carsStore.byId(newCarId.value)
  importedCarIds.value.push({
    carId: newCarId.value,
    label: car ? `${car.year ?? ''} ${car.make} ${car.model}`.trim() : newCarId.value
  })
}
```

Computed to detect multi-car state:
```ts
const isMultiCar = computed(() => importedCarIds.value.length >= 2)
const variantsResolved = ref(false)
```

Show the passive button when `isMultiCar && !variantsResolved`:
```html
<div v-if="isMultiCar && !variantsResolved" class="ncm-multicar-notice">
  <span>{{ importedCarIds.length }} cars in this card</span>
  <button type="button" @click="openVariantResolution">Set up tune tabs →</button>
</div>
```

Also gate the final Close/Done action — if `isMultiCar && !variantsResolved`, show
a prompt instead of closing directly:
```ts
function onDone() {
  if (isMultiCar.value && !variantsResolved.value) {
    showResolutionPrompt.value = true
    return
  }
  close()
}
```

---

### Step 3 — Resolution dialog

A lightweight modal (or inline panel within NewCardModal) triggered by either the
passive button or the save-intercept.

**State:**
```ts
const showResolutionPrompt = ref(false)
const resolutionChoice = ref<'tabs' | 'notabs' | null>(null)
// Per-variant discipline selection, keyed by carId
const variantDiscipline = ref<Record<string, string>>({})
```

**Layout:**
```
┌─────────────────────────────────────────┐
│  Multiple cars detected                  │
│  Smokin  ·  FD Corvette  ·  Austin Healey│
│                                          │
│  Set up tune tabs?                       │
│                                          │
│  [Yes, set up tabs]   [No, skip tunes]  │
│                                          │
│  — if Yes tabs chosen: ——               │
│  ┌────────────────────────────────────┐  │
│  │ 599D GTO           [Discipline ▼] │  │
│  │ FD Corvette #777   [Discipline ▼] │  │
│  │ Austin Healey Sprite [Discipline ▼]│  │
│  └────────────────────────────────────┘  │
│                           [Apply & Done] │
└─────────────────────────────────────────┘
```

**Discipline dropdown** — populated from `api.listTuningPresets()`. Each car row
gets its own `<select>`. Includes a "No tune" option at the top for cars you don't
have a tune for yet.

**On "Apply & Done":**
1. For each variant with a discipline selected: fetch the preset, build a
   `CardVariant` with `adjustments` seeded from preset values. For sliders not in
   the preset, use the existing card recipe's values (or stock defaults).
2. Patch the created card's recipe section: `PUT /api/cards/:id` with
   `sections[recipeIdx].variants` set to the built array.
3. Set `variantsResolved.value = true`, close the dialog, then close NewCardModal.

**On "No, skip tunes":**
Set `variantsResolved.value = true`, close dialog, proceed to close NewCardModal.
The card stays single-tune; RecipeSection's auto-propose will offer tabs again
next time the card is opened in edit mode.

---

### Step 4 — `makeVariantFromPreset` helper

Add to `NewCardModal.vue` (or extract to a util if reused elsewhere):

```ts
async function makeVariantFromPreset(carId: string, presetId: number): Promise<CardVariant> {
  const presets = await api.listTuningPresets()
  const preset = presets.find(p => p.id === presetId)
  // Build adjustments from existing recipe rows, patching values from preset
  const baseRows: AdjustmentRow[] = existingRecipeAdjustments() // from the created card
  const rows = baseRows.map(r => ({
    ...r,
    value: preset?.values[r.key] ?? r.stock,
    ...(preset?.values[r.key + ':min'] !== undefined ? { min: preset.values[r.key + ':min'] } : {}),
    ...(preset?.values[r.key + ':max'] !== undefined ? { max: preset.values[r.key + ':max'] } : {}),
  }))
  return {
    carId,
    tuneName: preset?.name ?? '',
    shareCode: '',
    coreSpecs: {},
    upgrades: [],
    adjustments: rows,
    liveryId: 0,
    tuneId: 0,
  }
}
```

---

## Gotchas

- **`RecipeSection` auto-propose** already exists for edit mode. The new flow
  is specifically for NewCardModal (creation time). Don't duplicate the logic —
  after resolution, the card will have `variants` set and RecipeSection will render
  the tab strip correctly without any additional wiring.
- **Preset values are sparse** — a preset only stores keys the user explicitly set.
  Always fall back to `r.stock` for missing keys, not `r.value`.
- **Card save after variant patch** — the card already exists in the DB by the time
  resolution runs (it was created during the first import). Use `cardsStore.save(id)`
  or a direct PUT. Read the card back from the store first to get the current
  sections array before patching.
- **`variantsResolved` resets on modal close** — it's session state, not persisted.
  If the user closes without resolving and reopens, RecipeSection's auto-propose
  banner handles it from that point on.
- **Do not touch RecipeSection** in this work — it's already correct for edit mode.
  All changes are in NewCardModal only.

---

## Order of work

1. Author the 5 discipline presets via the existing UI (no code)
2. Step 2 — carId tracking + passive button in NewCardModal
3. Step 3 — resolution dialog (inline within NewCardModal, not a separate component)
4. Step 4 — `makeVariantFromPreset` helper
5. Test end-to-end: create a new card with 2+ cars, verify resolution sets variants,
   verify RecipeSection renders tabs on next edit open
