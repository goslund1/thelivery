# Plan: Three-Layer Preset System (Baseline + Build)

## What this solves

The current preset system is flat — every preset is just a saved set of slider values. There's no concept of a "starting configuration" for a build type, so every new tune starts from scratch: sliders, upgrades, specs all blank.

This plan wires in a proper three-layer model:

```
Stock  →  Upgrade Baseline  →  Build Preset (tuned adjustments)
```

- **Stock** — pure game defaults. No parts, all sliders at zero. Implicit; no data needed.
- **Upgrade Baseline** (`kind: 'baseline'`) — the floor for a particular discipline/upgrade package. Carries both slider starting values AND the upgrade parts that define them. Applying it seeds sliders (and resets the stock reference to baseline values) + populates the parts list. Saved by Jason with the Baseline checkbox.
- **Build Preset** (`kind: 'build'`) — Jason's tuned adjustments on top of a baseline. References which baseline it was built from (`baselineId`). "Reset" goes back to baseline, not stock.

**Autofill trigger:** when a tune tab is named after a discipline that matches a baseline (e.g. a tab named "Drift" finds the "Drift" baseline), the system prompts to apply it — seeding both sliders and parts in one shot.

---

## What already exists

- `tuning_presets` table: `id, name, body (JSON { key: number }), kind, created_at` (migration 0016 added `kind`)
- `applyPresetValues(values, kind)` in `TuningAdjustments.vue` — already handles `kind === 'baseline'` by setting `updated.stock = values[r.key]` (resets the stock reference to baseline values)
- `executeApplyPreset()` — calls `applyImpliedTransmission()` after applying slider values
- Save dialog — `presetKind` ref (`'build' | 'baseline'`) + Baseline checkbox already present
- `◆` prefix on baseline entries in dropdown
- `onImpliedUpgrades` handler in `RecipeSection.vue` — already wires `applyImpliedUpgrades()` from `TuningAdjustments` emit

## What's missing

1. Baseline presets don't carry an upgrades list (body is sliders only)
2. Build presets don't reference their parent baseline
3. Applying a baseline doesn't seed the upgrades/parts list
4. No autofill trigger from tune tab discipline name
5. No "active baseline" tracking (which baseline is currently in effect)

---

## Implementation

### Step 1 — Migration 0017

```sql
ALTER TABLE tuning_presets ADD COLUMN upgrades TEXT;       -- JSON: UpgradeCategory[]
ALTER TABLE tuning_presets ADD COLUMN baseline_id INTEGER REFERENCES tuning_presets(id);
```

`upgrades` is null for build presets. `baseline_id` is null for baseline presets and for build presets not yet linked to one.

---

### Step 2 — Backend

**`list_tuning_presets`** — add `upgrades` and `baselineId` to each row in the response:
```rust
"upgrades":   r.get::<Option<String>, _>("upgrades")
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .unwrap_or(json!([])),
"baselineId": r.get::<Option<i64>, _>("baseline_id"),
```

**`create_tuning_preset`** — extend `CreatePresetReq` to accept `upgrades` and `baselineId`:
```rust
struct CreatePresetReq {
  name: String,
  values: Value,
  kind: String,
  upgrades: Option<Value>,     // ← new
  baseline_id: Option<i64>,    // ← new
}
```
Insert both new columns. Return them in the response.

**No new endpoints needed** — delete stays as-is.

---

### Step 3 — Frontend types + API

**`TuningAdjustments.vue`** local type:
```ts
type TuningPreset = {
  id: number
  name: string
  values: Record<string, number>
  kind: 'build' | 'baseline'
  upgrades?: UpgradeCategory[]     // ← new; baseline presets only
  baselineId?: number | null       // ← new; build presets only
  createdAt: string
}
```

**`api.ts`** — extend `createTuningPreset` payload:
```ts
createTuningPreset: (payload: {
  name: string
  values: Record<string, number>
  kind?: string
  upgrades?: UpgradeCategory[]     // ← new
  baselineId?: number | null       // ← new
}) => ...
```

---

### Step 4 — Save dialog: capture upgrades + baselineId

In `saveAsPreset()` in `TuningAdjustments.vue`:

```ts
async function saveAsPreset() {
  const name = presetNameInput.value.trim()
  const values = getAdjustments()  // existing

  // NEW: when saving as baseline, capture the current upgrades list via a new prop/emit
  // When saving as build, capture the active baseline id
  const payload: CreatePresetPayload = {
    name,
    values,
    kind: presetKind.value,
    upgrades: presetKind.value === 'baseline' ? props.upgrades : undefined,
    baselineId: presetKind.value === 'build' ? activeBaselineId.value : undefined,
  }
  const created = await api.createTuningPreset(payload)
  // ...
}
```

`TuningAdjustments` already receives `upgrades` via props (from `RecipeSection`). `activeBaselineId` is a new `ref<number | null>(null)` set when a baseline is applied.

---

### Step 5 — Apply baseline: seed parts list

In `executeApplyPreset()`, after the existing slider apply logic:

```ts
function executeApplyPreset() {
  const preset = presets.value.find(p => p.id === selectedPresetId.value)
  if (!preset) return

  // existing: apply slider values + transmission
  // ...applyPresetValues / applyImpliedTransmission...

  // NEW: if baseline preset has explicit upgrades, emit them up
  if (preset.kind === 'baseline') {
    activeBaselineId.value = preset.id
    if (preset.upgrades?.length) {
      emit('implied-upgrades', {
        toAdd: preset.upgrades.flatMap(cat =>
          cat.parts.map(part => ({ category: cat.category, part }))
        ),
        needsSpringsDialog: false,
      })
    }
  } else if (preset.baselineId) {
    activeBaselineId.value = preset.baselineId
  }
}
```

`RecipeSection.onImpliedUpgrades` already calls `applyImpliedUpgrades()` — no changes needed there.

---

### Step 6 — Preset bar UI: separate baselines from builds

Split the single dropdown into two visual groups using `<optgroup>`:

```vue
<select v-model="selectedPresetId">
  <optgroup label="Baselines">
    <option v-for="p in baselinePresets" :key="p.id" :value="p.id">◆ {{ p.name }}</option>
  </optgroup>
  <optgroup label="Builds">
    <option v-for="p in buildPresets" :key="p.id" :value="p.id">{{ p.name }}</option>
  </optgroup>
</select>
```

Computed helpers:
```ts
const baselinePresets = computed(() => presets.value.filter(p => p.kind === 'baseline'))
const buildPresets    = computed(() => presets.value.filter(p => p.kind === 'build'))
```

Show the active baseline indicator in the preset bar when `activeBaselineId` is set:
```vue
<span v-if="activeBaseline" class="ta-baseline-label">◆ {{ activeBaseline.name }}</span>
```

---

### Step 7 — Autofill trigger from tune tab name

`TuningAdjustments.vue` already receives the current tune name (or can receive it as a prop). When the tune name changes and a matching baseline exists, show a one-time prompt:

```ts
// New prop
const props = defineProps<{
  // ...existing props...
  tuneName?: string   // ← new; tune tab label
}>()

watch(() => props.tuneName, (name) => {
  if (!name || activeBaselineId.value) return   // already seeded
  const match = baselinePresets.value.find(
    p => p.name.toLowerCase() === name.toLowerCase()
  )
  if (match) {
    suggestBaselineId.value = match.id   // drives a one-time "Apply [Drift] baseline?" banner
  }
})
```

The banner reuses the existing suggest-bar two-surface pattern (secondary surface above primary). Dismiss = session only. Apply = calls `executeApplyPreset` with the matched baseline id.

---

### Step 8 — Stock definition (deferred)

Stock is implicit: all sliders at zero, no parts. Jason can create a "Stock [Car Model]" baseline manually through the existing save flow with sliders at zero — no special UI needed at this stage. Promote to a first-class feature once the backfill pass (Step 7 usage) reveals if it's actually needed.

---

## Implementation order

1. Migration 0017 (5 min)
2. Backend handler updates (30 min)
3. Frontend type + API changes (15 min)
4. `saveAsPreset` captures upgrades + baselineId (20 min)
5. `executeApplyPreset` seeds parts list + tracks `activeBaselineId` (30 min)
6. Preset bar `<optgroup>` split + active baseline label (20 min)
7. Tune name autofill trigger + banner (45 min)

## Test sequence

1. Create a "Drift" baseline: add Race Springs & Dampers + Drift ARB parts to upgrades, tune alignment/arb sliders, save as Baseline. Confirm `upgrades` column populated in DB.
2. Create new tune tab named "Drift". Confirm autofill banner appears. Apply it. Confirm sliders seeded, parts list populated, `activeBaselineId` set.
3. Tweak a few sliders. Save as Build preset. Confirm `baseline_id` references the Drift baseline.
4. Delete tune data. Apply the Build preset. Confirm sliders + parts apply correctly.
5. Verify slider-only build presets (no baseline) still work unchanged.

## Files touched

- `backend/migrations/0017_preset_baseline.sql` — new
- `backend/src/main.rs` — `CreatePresetReq`, `list_tuning_presets`, `create_tuning_preset`
- `frontend/src/api.ts` — `createTuningPreset` payload type
- `frontend/src/components/TuningAdjustments.vue` — types, `executeApplyPreset`, `saveAsPreset`, preset bar UI, autofill watch, `activeBaselineId` ref
