# Plan: RecipeSection prop-mutation refactor

## Goal

Remove all direct `props.recipe.*` mutations from `RecipeSection.vue`. Replace with a local
reactive copy that emits `update:recipe` to callers. This makes RecipeSection a proper
controlled component and eliminates the last prop-mutation anti-pattern in the recipe flow.

## Background

`TuningAdjustments.vue` was already fixed (now emits `update:adjustments`). `RecipeSection`
still mutates `props.recipe` directly in multiple places. It works today because all callers
pass a reactive Pinia object or local reactive, but it's fragile and violates Vue's one-way
data flow.

## Mutation sites to eliminate (all in `RecipeSection.vue`)

| Line | Site | How |
|------|------|-----|
| 19 | `props.recipe.coreSpecs[k] = ''` | Normalization at setup — move into local copy init |
| 118 | `props.recipe.coreSpecs[key] = value` | `onSpecChange()` — write to local instead |
| 132 | `props.recipe.shareCode = formatted` | `onShareCodeInput()` — write to local instead |
| 165 | `props.recipe.upgrades.splice(...)` | `applyPreset()` — write to local instead |
| 189 | `props.recipe.upgrades.splice(0)` | `clearAllUpgrades()` — write to local instead |
| 207 | `v-model="recipe.tuneName"` | Template — bind to `local.tuneName` instead |
| 239 | `v-model="recipe.coreSpecs[k]"` | Template — bind to `local.coreSpecs[k]` instead |
| 336 | `recipe.adjustments.splice(...)` | TuningAdjustments emit handler — write to local instead |

## Implementation steps

### 1. RecipeSection.vue — introduce local copy + emit

```ts
const emit = defineEmits<{ 'update:recipe': [recipe: ForzaRecipeSection] }>()

// Deep-clone helper (only needed at init and on external prop swap)
function cloneRecipe(r: ForzaRecipeSection): ForzaRecipeSection {
  return JSON.parse(JSON.stringify(r))
}

const local = reactive<ForzaRecipeSection>(cloneRecipe(props.recipe))

// Sync if the parent swaps the recipe entirely (e.g. switching cards)
watch(() => props.recipe, (r) => Object.assign(local, cloneRecipe(r)), { deep: false })

// Emit after every local mutation
function flush() { emit('update:recipe', JSON.parse(JSON.stringify(local))) }
```

Replace every `props.recipe.*` write with `local.*` + call `flush()` after.
Replace every `recipe.*` in the template with `local.*`.

The TuningAdjustments handler becomes:
```html
@update:adjustments="rows => { local.adjustments.splice(0, local.adjustments.length, ...rows); flush() }"
```

### 2. CardView.vue — handle the emit

`CardView` passes `section` (a live Pinia `ForzaRecipeSection`). On emit, update it in place:

```html
<RecipeSection
  v-else-if="section.type === 'forza_recipe'"
  :recipe="section"
  :card-id="card.id"
  @update:recipe="updated => Object.assign(section, updated)"
/>
```

`Object.assign` preserves reactivity on the Pinia object. `markDirty` is already injected
inside RecipeSection so dirty tracking still fires.

### 3. EditCardModal.vue — handle the emit + verify snapshot/restore

`EditCardModal` has a snapshot/restore flow. The recipe comes from:
```ts
const recipeSection = computed<ForzaRecipeSection | undefined>(...)
```

On emit, write back to the card in the Pinia store:
```html
<RecipeSection
  :recipe="recipeSection"
  @update:recipe="updated => { if (recipeSection.value) Object.assign(recipeSection.value, updated) }"
/>
```

**Snapshot/restore** — verify that `recipeSnapshot` (JSON.stringify on edit entry) and the
restore path (lines 172–180, directly mutating `recipe.*` fields) still work correctly after
this change. They should, because they operate on the Pinia object directly, not through
RecipeSection.

### 4. NewCardModal.vue — handle the emit

`NewCardModal` uses a local `reactive<ForzaRecipeSection>` named `recipe`. On emit, merge:

```html
<RecipeSection
  :recipe="recipe"
  @update:recipe="updated => Object.assign(recipe, updated)"
/>
```

`recipe` is read on submit at lines 236–240, so as long as `Object.assign` keeps it
up-to-date, submit works unchanged.

## Risk areas

1. **EditCardModal snapshot/restore** — most likely to have subtle bugs. Test: enter edit
   on a card with a recipe, change tuneName/shareCode/specs/upgrades, then hit Discard. All
   changes should revert cleanly.

2. **Keystroke performance** — `flush()` emits a `JSON.parse(JSON.stringify(...))` clone on
   every tuneName/shareCode keystroke. If this feels sluggish, debounce flush on text fields
   only (a 150ms debounce is invisible to users).

3. **UpgradesPicker** — currently receives `:upgrades="recipe.upgrades"` and may mutate it
   internally. Check whether UpgradesPicker also needs an emit pattern, or whether the
   `watch(() => props.recipe)` in RecipeSection picks up those changes naturally through
   the Pinia reactivity chain.

## Files to change

- `frontend/src/components/RecipeSection.vue` (primary)
- `frontend/src/components/CardView.vue`
- `frontend/src/components/EditCardModal.vue`
- `frontend/src/components/NewCardModal.vue`

## Testing checklist

- [ ] Edit tuneName, shareCode, coreSpecs in card view — saves correctly
- [ ] Edit upgrades (add/remove parts via UpgradesPicker) — saves correctly
- [ ] Apply/clear upgrade preset — works, dirty flag set
- [ ] Edit adjustments (sliders, min/max) — saves correctly
- [ ] EditCardModal: change fields, hit Discard — all fields revert
- [ ] EditCardModal: change fields, hit Save — persists to backend
- [ ] NewCardModal: fill recipe, submit — recipe included in new card payload
- [ ] Switch between cards — local state does not bleed across cards
