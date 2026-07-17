# Design: Car Identity, Livery & Tune Data Model

Decided in session 2026-07-05. Updated same session with all open-item resolutions.
This doc is the foundation for the next build phase. Do not start schema
migrations or UI work until this is agreed.

---

## The Three Identity Layers

A car in the catalog has three independently variable layers:

| Layer | What it is | Example | Table |
|---|---|---|---|
| **Base car** | Physical car from the game. Fixed — can't customise below this. | 2001 Nissan Skyline GT-R R34, FH6 | `cars` (exists) |
| **Livery** | Paint + vinyl design applied to the car. | "Bayside Blue Factory" / "JDM Dreams" | `liveries` (new) |
| **Tune** | Performance config: upgrades, adjustments, share code. | "Street Build" / "Track Setup" | `tunes` (new) |

A **card** presents one or more (car + livery + tune) combinations. An **image**
belongs to a specific livery (and optionally a specific tune).

---

## Serial Number Format

Every entity in the catalog gets a structured serial that encodes its full
lineage. Human-readable and machine-sortable.

```
[GAME] - [CAR-CODE] - L[###] - T[###] - [TYPE][###]

FH6 - NISR34 - L001 - T001 - C042    ← a card
FH6 - NISR34 - L001 - T001 - I003    ← an image
FH6 - NISR34 - L000                  ← factory livery (no tune yet)
FH6 - NISR34 - L001 - T002           ← second tune on the same livery
```

### Segment rules

| Segment | Scope | Notes |
|---|---|---|
| `GAME` | Global | `FH5` or `FH6` |
| `CAR-CODE` | Per game | Curated field on `cars` table (`cars.code`). Auto-suggested at seed time from make + model abbreviation (`NIS` + `R34` → `NISR34`); collision resolution appends year then a digit. `UNIQUE` DB constraint. See Car Code section. |
| `L###` | Per car-game | `L000` reserved for factory/stock liveries (one per factory color). `L001`+ for custom. 3 digits. |
| `T###` | Per livery | `T001`+ for each tune under a livery. 3 digits. |
| `C###` | Global | Existing `catalogNumber`. 3 digits. |
| `I###` | Per card | Image index within a card. 3 digits. |

---

## Tables

### `cars` (exists — gains `code` field)

```sql
id TEXT PRIMARY KEY   -- 'fh6-nissan-skyline-gtr-r34'
game TEXT             -- 'FH5' | 'FH6'
make, model, year, class, pi, drive, country, category, decade, status, dlc
code TEXT UNIQUE      -- 'NISR34', 'NISR34-2002' — curated, collision-checked
```

#### Car Code generation

`code` is a curated field auto-suggested at seed time, frozen after creation.

**Generation algorithm:**
1. Candidate = first 3 chars of make (uppercased) + model abbreviation (stripped of spaces/punctuation, uppercased). E.g. `Nissan` + `Skyline GT-R R34` → `NIS` + `R34` → `NISR34`.
2. Check `UNIQUE` constraint — if clear, insert.
3. On collision: append 2-digit year → `NISR34-02` (2002). Re-check.
4. Still collides: append sequential digit → `NISR34-02-2`. Re-check.
5. After 3 failed attempts: flag for manual review, do not insert.

**Enforcement:**
- `UNIQUE` constraint on `cars.code` at DB level — hard rejection of duplicates regardless of source.
- Seed script runs the algorithm, logs any manual-review flags, inserts clean ones.
- New car API endpoint returns `409 Conflict` with the suggested alternative code if a collision is detected, so you can pick a better one before committing.

**`car_colors` data source:** scraped by us from Forza wikis and fan databases
(same approach as the cars DB). Geoff is not involved — this is our own scrape
and seed.

---

### `car_colors` (new — seeded from scrape)

Factory paint options per car. Scraped from Forza wikis/fan DBs. One-time seed,
same approach as the cars DB.

```sql
id          INTEGER PRIMARY KEY AUTOINCREMENT
car_id      TEXT NOT NULL REFERENCES cars(id)
name        TEXT NOT NULL   -- "Bayside Blue", "Midnight Purple"
hex_approx  TEXT            -- rough hex for color filtering, e.g. '#1a3a6e'
filter_color TEXT           -- maps to color taxonomy: 'Blue', 'Purple', etc.
```

In the livery creation flow, selecting a factory livery shows a dropdown of
this car's known factory colors instead of AI assessment.

---

### `liveries` (new)

One row per distinct paint/vinyl design on a specific car.

```sql
id            INTEGER PRIMARY KEY AUTOINCREMENT
car_id        TEXT NOT NULL REFERENCES cars(id)
serial        TEXT UNIQUE NOT NULL   -- 'FH6-NISR34-L001'
name          TEXT NOT NULL          -- "JDM Dreams" / "Bayside Blue (Factory)"
is_factory    BOOLEAN NOT NULL DEFAULT 0
car_color_id  INTEGER REFERENCES car_colors(id)   -- set when is_factory=true
share_code    TEXT                   -- Forza livery/cosmetic share code (if shared)
color_primary TEXT                   -- from color taxonomy (AI-assessed for custom)
color_secondary TEXT                 -- optional second dominant color
created_at    TEXT NOT NULL DEFAULT (datetime('now'))
```

**Factory liveries:** `is_factory=1`, `car_color_id` set, color from the
`car_colors` table. `L000` is the first factory color; additional factory
colors are `L001`, `L002` etc — they are still L-numbered but flagged as
factory.

**Custom liveries:** `is_factory=0`, color assessed by Claude vision API from
the lead image at import time.

---

### `tune_types` (new — managed list)

Seeded with known types; new entries can be added via admin UI.

```sql
id    INTEGER PRIMARY KEY AUTOINCREMENT
name  TEXT UNIQUE NOT NULL   -- 'Drift', 'Race', 'Rally', 'Offroad',
                             -- 'Stunt', 'Drag', 'Gimmick', 'Overpowered'
sort_order INTEGER DEFAULT 0
```

Initial seed: Drift, Race, Rally, Offroad, Stunt, Drag, Gimmick, Overpowered.

---

### `tunes` (new)

One row per performance configuration for a specific livery.

```sql
id            INTEGER PRIMARY KEY AUTOINCREMENT
livery_id     INTEGER NOT NULL REFERENCES liveries(id)
car_id        TEXT NOT NULL REFERENCES cars(id)   -- denorm for fast lookup
serial        TEXT UNIQUE NOT NULL   -- 'FH6-NISR34-L001-T001'
official_name TEXT                   -- exact name as shared on FH servers (immutable)
type_id       INTEGER REFERENCES tune_types(id)
share_code    TEXT                   -- Forza performance share code
core_specs    TEXT                   -- JSON: Record<string, string>
upgrades      TEXT                   -- JSON: UpgradeCategory[]
adjustments   TEXT                   -- JSON: AdjustmentRow[]
created_at    TEXT NOT NULL DEFAULT (datetime('now'))
```

**Official name vs display title:** `official_name` is the name registered on
Forza servers, tied to the share code — never edited after creation. The card
title is a display layer that can diverge freely.

---

### `images` (migrated in 0010 — gains livery/tune refs)

```sql
-- add in next migration:
livery_id   INTEGER REFERENCES liveries(id)
tune_id     INTEGER REFERENCES tunes(id)
serial      TEXT   -- 'FH6-NISR34-L001-T001-I003'
```

---

### `cards` (exists — gains livery/tune refs for single-combo cards)

```sql
-- add in next migration:
livery_id   INTEGER REFERENCES liveries(id)   -- null for multi-combo cards
tune_id     INTEGER REFERENCES tunes(id)      -- null for multi-combo cards
serial      TEXT   -- 'FH6-NISR34-L001-T001-C042' (single-combo only)
```

Multi-combo cards (tasting menus, multi-tune) reference via the variants
array — each variant carries a `liveryId` + `tuneId`.

---

## Variant Model (redesign of current CarVariant)

Current `CarVariant` interface stores tune data inline. Replace with DB refs:

```ts
interface CardVariant {
  liveryId: number      // → liveries table
  tuneId:   number      // → tunes table
  // resolved display data (populated from DB at load time, not stored in card JSON):
  carId?:      string   // from livery.car_id
  carName?:    string   // year make model
  liveryName?: string
  tuneName?:   string
  tuneType?:   string
  shareCode?:  string
  coreSpecs?:  Record<string, string>
  upgrades?:   UpgradeCategory[]
  adjustments?: AdjustmentRow[]
}
```

### Tab strip modes

Mode is **derived from the variants array** — no explicit flag on the card.

| Condition | Mode | Tabs show |
|---|---|---|
| All variants share the same `carId` | **Multi-tune** | Tune name / type |
| Variants have different `carId` values | **Multi-car** | Car name |

Constraint: the same car cannot appear twice in a multi-car card. Each car
gets exactly one slot.

The `+ Add` button in the tab strip expands to offer:
- **Add Car** — adds a new (car + livery + tune) variant with a different car
- **Add Tune** — adds a new tune variant for the same car/livery

### Suggested tune tab

When a community suggestion is promoted to official, it auto-creates a tune
record and a read-only variant tab appears alongside the published tune tab.

- **Published tab** (yours): interactive sliders, works as today
- **Suggested tab**: read-only sliders in their recommended positions, colored
  to show deviation from FH stock — same display as SuggestionViewer, but
  inline on the card. Cannot be adjusted by visitors.
- Suggested variants are flagged: `variant.isSuggested = true`
- No "Promote" action needed — it was already promoted to get this tab

### Migration strategy for existing cards

Existing cards keep inline tune data in the recipe JSON body. `livery_id` and
`tune_id` on `cards` and `images` are nullable FKs — populated lazily when
cards are next edited. No batch migration. Clean up after the feature is
working.

---

## Color Taxonomy (managed list)

Used for filter-axis color classification across all liveries.

**Initial set:** Red, Blue, Green, Yellow, Orange, Purple, Pink, White, Black,
Silver, Grey, Gold, Bronze, Teal, Multi

Extendable via admin UI (same pattern as tune types).

**Assignment rules:**
- Factory livery → color comes from `car_colors.filter_color` (scraped, mapped at seed time)
- Custom livery → Claude vision API reads lead image at import, returns 1-2
  dominant colors from the taxonomy, stored as `color_primary` / `color_secondary`

---

## Real-Time Import Interrupt

When a photo is tagged with a liveryId in PhotoDetail that belongs to a
different car than any existing tagged photo on the card:

1. **First firing only** — triggers once per card. Once the card is already
   multi-car, no further interrupts regardless of how many more cars are added.
2. **Prompt:** "Photos from 2 different cars detected — set this up as a
   multi-car card?"
3. **Accepting:** initiates the variant creation flow for the new car.
4. **Existing tune lookup:** query `tunes` + `liveries` for any existing tune
   data for that carId anywhere in the catalog. If found: "Found tune data for
   [Year Make Model] in [Card Name] — import it?" Accepting copies tune data
   into the new variant.
5. **No match:** variant created empty. User selects an upgrade preset and
   dials in sliders from scratch.

---

## Import Flow (livery creation)

When importing photos, the image import dialog guides you to a livery record:

1. **Make/Model search** → locked to `cars` table, no freeform entry
2. **Factory or custom?**
   - Factory → `car_colors` dropdown for that car → creates/selects `L00X` livery
   - Custom → enter livery name → AI assesses color from first imported photo
3. **Tune association** → link to an existing `tunes` record or create new
4. **Match found?** → "This livery already exists in [Card Name] — connect to it?"
5. **Photo tagged** → `image.livery_id` + `image.tune_id` set

**Future (new single card for existing variant):** when creating a new card and
picking a car, query for existing liveries/tunes for that carId. If found,
offer to pull data in.

---

## Build Order (next session)

1. `cars.code` field: add to DB (migration), update seed script with
   auto-generation algorithm + collision check + manual-review flagging
2. `car_colors` scrape + seed (our own scrape, not Geoff)
3. DB migrations: `car_colors`, `liveries`, `tune_types`, `tunes`; add
   `code`/`livery_id`/`tune_id`/`serial` to `cars`, `images`, `cards`
4. Backend API: CRUD for liveries, tunes, tune_types; serial generator
   utility; 409 on car code collision
5. Update `CardVariant` type to `(liveryId, tuneId)` refs + resolved
   display fields
6. New stores: liveries, tunes, tune_types
7. New component: `LiveryPicker.vue`
   (Make/Model → factory/custom → factory color dropdown or AI assess → name)
8. `RecipeSection.vue`: resolve variant data from stores; Add Car / Add Tune
   button split; suggested tune tab (read-only, `isSuggested` flag)
9. `PhotoDetail.vue`: tag by livery; real-time interrupt on second distinct car
10. Existing tune lookup when creating a new variant
11. AI color assessment endpoint (server-side Claude API call, admin-only)
12. Color + tune-type filter axes in SideBug

**Lazy migration:** existing cards keep inline recipe data. `livery_id` /
`tune_id` on cards/images populate on next edit. No batch migration needed now.
