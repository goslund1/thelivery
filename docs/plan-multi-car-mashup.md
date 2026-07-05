# Plan: Multi-Car Mashup Card

## What it is

A single card that covers multiple cars — each with its own photos, tune, upgrades, and adjustments. The user switches between cars via a tab strip inside the recipe section. The gallery filters to show the active car's photos.

Single-car cards are untouched — the new structure is additive.

---

## Data model

### Images table (new migration)

Currently photos are just file paths embedded in the card's JSON body. Promoting them to DB records gives stable IDs, queryable car associations, and useful metadata captured at import time.

```sql
CREATE TABLE images (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  card_id     TEXT NOT NULL,
  path        TEXT NOT NULL,
  thumb_path  TEXT,
  car_id      TEXT,            -- FK to cars.id (nullable for untagged)
  filename    TEXT,            -- user-customized display name
  alt_text    TEXT,
  sort_order  INTEGER NOT NULL DEFAULT 0,
  created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
```

`CardImage` in `types.ts` gains `carId?: string` and `filename?: string`. Existing cards migrate lazily — on first save, the card's `images` array is written to the `images` table.

### Recipe variants

`ForzaRecipeSection` gains an optional `variants` array. When present and has 2+ entries, the tab strip renders. Single-car (no `variants`) works exactly as today.

```ts
interface CarVariant {
  carId: string
  tuneName: string
  shareCode: string
  coreSpecs: Record<string, string>
  upgrades: UpgradeCategory[]
  adjustments: AdjustmentRow[]
}

interface ForzaRecipeSection {
  // ... all existing fields unchanged (single-car path)
  variants?: CarVariant[]
}
```

---

## Import flow upgrade

At upload time (ImagePicker), each photo gets:

1. **CarPicker** — tap the make/model DB to associate a car
2. **Filename input** — defaults to the card name, editable (used as display name + informs sort)

The existing PhotoDetail shadowbox (post-import editing of carId + alt text) stays as a correction path.

Backend `POST /api/images` returns the full image record (id, path, carId, etc.) instead of just `{ path }`.

---

## UI: recipe tab strip

Inside `RecipeSection.vue`, when `props.recipe.variants` has 2+ entries:

- A tab strip renders above the tune content, showing each car's make/model name (resolved from `carsStore`)
- Active tab drives which variant's data is shown below
- Single-car path (`variants` absent or length 1): no tab strip, existing layout unchanged

```
[ Tune / Build Parts ]
  ┌──────────────────────────────────┐
  │ [Nissan 350Z ▾] [Honda NSX ▾]  │  ← tab strip, only when 2+ variants
  ├──────────────────────────────────┤
  │  Tune name / share code          │
  │  Core specs                      │
  │  Upgrades                        │
  │  Sliders                         │
  └──────────────────────────────────┘
```

---

## UI: gallery filtering

When the active car tab changes, Gallery filters its displayed photos to those with a matching `carId`. A "All" tab or chip shows all photos regardless. Photos with no `carId` always show.

---

## Edit flow

In EditCardModal / NewCardModal, when in multi-car mode:

- "Add car" button in the recipe section header adds a new variant (opens CarPicker)
- Each variant tab is editable independently
- Removing a variant (×) prompts if it has non-empty data

Single-car cards can be promoted to multi-car by adding a second variant — the existing top-level recipe fields become variant[0].

---

## Build order

1. **Migration**: `images` table + update `POST /api/images` to return full record
2. **Import flow**: CarPicker + filename input in ImagePicker at upload time
3. **Types**: `CarVariant`, updated `ForzaRecipeSection`, updated `CardImage`
4. **RecipeSection**: variant tab strip, active-variant state, edit controls
5. **Gallery**: carId-based filtering, responds to active variant
6. **EditCardModal / NewCardModal**: add/remove variant UI
7. **Lazy migration**: on card save, sync `images` array → `images` table

---

## Out of scope (for now)

- Cross-card car queries ("all photos of this car")
- Shared tune fields across variants
- Variant-level history / diff
