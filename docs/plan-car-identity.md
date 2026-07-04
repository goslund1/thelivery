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
| `id`          | TEXT PK | slug, e.g. `fh5-2020-porsche-911-gt3`     |
| `make`        | TEXT    | e.g. `Porsche`                             |
| `model`       | TEXT    | e.g. `911 GT3`                             |
| `year`        | INTEGER | manufacture year as listed in game         |
| `game`        | TEXT    | `FH5` or `FH6`                             |
| `class`       | TEXT    | stock class: `D/C/B/A/S1/S2/X`            |
| `pi`          | INTEGER | stock PI rating                            |
| `drivetrain`  | TEXT    | `FWD/RWD/AWD`                             |

### `cards` table change

Add nullable `car_id TEXT REFERENCES cars(id)` — nullable so all existing cards
remain valid. New cards should always set it; old cards can be backfilled manually.

## Scraper tool (`tools/cars/`)

Node script, same pattern as `tools/extract/`. Pulls canonical car lists from the
Forza wiki or a community source (e.g. forza.fandom.com car category pages).

- **FH5** — one-shot; content is final
- **FH6** — re-runnable; new cars drop over the title's run

Re-runs are **additive upserts** — insert new cars, update changed fields, never
delete. Existing `car_id` FKs on cards are never broken by a re-run.

Output: `backend/seed/cars.json` (same pattern as `seed/cards.json`).

## Backend

- New migration: `cars` table + `car_id` column on `cards`
- Seeding: on startup, if `cars` is empty, import `seed/cars.json`
- New endpoints:
  - `GET /api/cars` — full list (for picker autocomplete)
  - `GET /api/cars?game=FH6` — filtered by game
  - `POST /api/cars` (admin) — add a single car manually if scraper misses one
- Existing card endpoints pass `car_id` through transparently (it's part of the card JSON body already once added to the type)

## Frontend

- New `Car` type in `types.ts`
- `Card` gets optional `carId?: string`
- **Car picker component** — search-to-select, used in NewCardModal and EditCardModal:
  - Text input filters by make/model/year in real time
  - Shows game badge (FH5 / FH6) next to each result
  - "New card" paths: pick existing car, or flag for manual entry if missing
  - Selected car displayed as a chip with make/model/year/game

## Build order

1. Scraper tool — produces `seed/cars.json`
2. Backend migration + seeding + endpoints
3. Frontend types
4. Car picker component
5. Wire into NewCardModal / EditCardModal
6. Backfill existing cards manually via EditCardModal

## Relationship to Submit Tune

A submitted tune will reference `car_id`, not `card_id`. This means:
- One car can have many cards (Photo Safari, Redux, Street build)
- A submitted tune is relevant to all cards sharing that car
- Jason sees the car identity clearly when reviewing submissions

## UI (deferred)

A browseable car registry view is not needed yet. The picker in card create/edit
is the only UI surface required for now. A full registry browser can come later
when Submit Tune needs it.
