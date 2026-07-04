# Plan: Car Identity

## Problem

Cards currently have no structured car metadata. `name` and `subtitle` are freeform.
A Photo Safari card and a Redux card for the same car have no machine-readable link.
This is a prerequisite for Submit Tune (a submitted tune must anchor to a car, not just
a card ID) and for any future filtering/grouping by vehicle.

## Data model

### New `cars` table

| column        | type    | notes                                      |
|---------------|---------|--------------------------------------------|
| `id`          | TEXT PK | slug, e.g. `fh6-nissan-skyline-gtr-r34`   |
| `make`        | TEXT    | e.g. `Nissan`                              |
| `model`       | TEXT    | e.g. `Skyline GT-R V-Spec`                |
| `year`        | INTEGER | manufacture year as listed in game         |
| `game`        | TEXT    | `FH5` or `FH6`                            |
| `class`       | TEXT    | stock class: `D/C/B/A/S1/S2/X`           |
| `pi`          | INTEGER | stock PI rating (FH5 only from wiki data) |
| `drivetrain`  | TEXT    | `FWD/RWD/AWD` (FH6 only from scraper)    |
| `category`    | TEXT    | e.g. `Modern Sports` (FH6 only)           |
| `dlc`         | TEXT    | DLC pack name, or null for base game       |

### `cards` table change

Add nullable `car_id TEXT REFERENCES cars(id)` — lives on the **card**, not the
recipe section, because the whole card represents one car (livery, photos, tune
are all one vehicle). Nullable so existing cards don't break; backfill manually.

### Welcome Pack note

Welcome Pack cars are stored as separate entries (distinct `dlc` value) since it's
unconfirmed whether share codes cross between Welcome Pack and base game versions of
the same car. If confirmed compatible, they can be collapsed in the UI later; the
data stays safe either way.

## Scraper tool (`tools/cars/`) — DONE

✓ `backend/seed/cars.json` — 1,748 cars (896 FH6 + 852 FH5 after dedup)
✓ FH6 from forzahorizoncar.com — make/model/year/class/drive/category/country
✓ FH5 from Forza Fandom MediaWiki API — make/model/year/class/pi/dlc
✓ Additive upsert — safe to re-run; `npm run scrape:fh6` for FH6 refreshes
✓ `fh5-` / `fh6-` ID prefixes disambiguate same car across games

## Backend

- New migration: `cars` table + `car_id` column on `cards`
- Seeding: on startup, if `cars` is empty, import `seed/cars.json`
- New endpoints:
  - `GET /api/cars` — full list (for picker autocomplete)
  - `GET /api/cars?game=FH6` — filtered by game
  - `POST /api/cars` (admin) — add a single car manually if scraper misses one
- Existing card endpoints pass `car_id` through transparently (part of the card
  JSON body once added to the type)

## Frontend

### Types
- New `Car` type in `types.ts`
- `Card` gets optional `carId?: string`

### Car picker — inline in RecipeSection, next to share code

The car identity lives **contextually next to the share code field** — that's where
the car-specificity of a tune code is most obvious.

**Unset state** (in edit mode):
```
Share code   [ 123 456 789 ]   [+ FH5] [+ FH6]
```

**Set state** — buttons collapse to a chip:
```
Share code   [ 123 456 789 ]   [FH6 · 2022 Porsche 911 GT3  ×]
```

`×` clears and restores the buttons. `car_id` written to the card on chip selection.

### Picker interaction (keyboard-first, Jobs-ian)

1. Click `[+ FH5]` or `[+ FH6]` — sets game context, opens search input
2. Type to filter — make, model, year all searched; results update live
3. Arrow up/down to navigate results; Enter to select; Escape to dismiss
4. No mode switching after step 1 — pure keyboard from there

Results show: `[badge] year Make Model` — e.g. `[FH6] 2022 Porsche 911 GT3`

FH5 and FH6 are kept separate pools (set by the button click), so results are
never mixed and there's no need for game-filter keywords in the search string.

### Card view display

The linked car identity is **visible on the card in view mode** — not just in
edit. Exact placement TBD when building, but somewhere in CardMeta or at the top
of the RecipeSection, showing `[FH6] 2022 Porsche 911 GT3` as a non-interactive
badge. Makes the card immediately legible as "this is what this car is."

## Build order

1. ✓ Scraper — `seed/cars.json` done
2. Backend migration + seeding + endpoints
3. Frontend `Car` type + `carId` on `Card`
4. Car picker component (search-to-select, keyboard-first)
5. Wire into RecipeSection (next to share code) + card view display badge
6. Backfill existing cards manually via EditCardModal

## Relationship to Submit Tune

A submitted tune references `car_id`, not `card_id`. One car can have many cards
(Photo Safari, Redux, Street build) — a submitted tune is relevant to all of them.
Jason sees the car identity clearly when reviewing the submission queue.

## UI registry (deferred)

A browseable car registry view is not needed yet. The inline picker is the only
UI surface required for now. A full registry browser comes later when Submit Tune
needs it.

## Future: photo → car association (deferred, separate planning doc)

Photos are currently associated with cards. A richer model would associate photos
directly with car IDs — so a car's photo history spans multiple cards, and a future
gallery could show "all photos of this Porsche across every card." This is a
meaningful data model change (junction table or `car_id` on the image row) and
needs its own planning session. It does not block current car identity work.
