# Upgrades ↔ Tuning Link — Plan

Connect the upgrades parts list to the tuning adjustment sliders so that moving
a slider off-stock automatically implies the appropriate race-quality upgrade part
in the corresponding category. The gearing tab is the single exception where the
relationship runs the other direction.

---

## The rule

**Tuning drives upgrades (all tabs except gearing):**
Any slider with `value !== stock` implies the race-quality part for that tab's
upgrade category. The auto-populate adds that part to the card's upgrades list
if it isn't already there.

**Upgrades drive tuning (gearing tab only):**
The transmission is adjustable at stock, so slider position alone can't imply an
upgrade. Instead, selecting a Sport or Race transmission in the Drivetrain upgrade
category sets the appropriate Transmission entry. The upgrade choice is the
meaningful signal.

---

## Canonical upgrade categories

These replace all current free-form variants. Names mirror the tuning tab
vocabulary exactly. Visual layout does not change — only the stored strings.

### Tuning-connected categories

| Category | Tuning tab(s) | Notes |
|---|---|---|
| **Tires** | tires | Single item; tier varies — Race / Rally / Drift / Sport / Off-Road |
| **Transmission** | gearing | Exception — upgrade drives tuning; named by speed + tier |
| **Alignment** | alignment | Single item |
| **Anti-Roll Bars** | arb | Front and rear are independent — either can be absent |
| **Springs & Dampers** | springs + damping | One upgrade item covers both tabs |
| **Aero** | aero | Front and rear are independent — bumper and wing separately |
| **Brakes** | brakes | Single item; tier varies |
| **Differential** | differential | Single item; tier varies |

### Non-tuning categories (no auto-populate, unchanged)

| Category | Notes |
|---|---|
| **Engine** | Internal parts; no tuning tab connection |
| **Drivetrain** | Clutch, driveline — beyond differential and transmission |
| **Conversions** | Engine swaps, drivetrain conversions, body conversions |

---

## Category string cleanup

Current variants in seed data → canonical replacement:

| Current string | Canonical |
|---|---|
| `Platform & Handling` | split — see parts migration below |
| `Platform and Handling` | split — see parts migration below |
| `Tires & Rims` | `Tires` |
| `Tires and Wheels` | `Tires` |
| `Aero and Appearance` | `Aero` |
| `Bodykits and Conversion` | `Conversions` |
| `Body Kits and Conversions` | `Conversions` |
| `Engine` | `Engine` (no change) |
| `Drivetrain` | `Drivetrain` (no change) |

**Parts migration out of Platform & Handling:**

Current part strings → new category:

| Part string | New category |
|---|---|
| `Race Springs & Dampers` | Springs & Dampers |
| `Rally Springs & Dampers` | Springs & Dampers |
| `Sport Springs and Dampers` | Springs & Dampers |
| `Race Front Anti-Roll Bar` | Anti-Roll Bars |
| `Race Rear Anti-Roll Bar` | Anti-Roll Bars |
| `Sport Front Anti-Roll Bars` | Anti-Roll Bars |
| `Street Front Anti-Roll Bars` | Anti-Roll Bars |
| `Street Rear Anti-Roll Bars` | Anti-Roll Bars |
| `Race Brakes` | Brakes |
| `Street Brakes` | Brakes |
| `Race Chassis Reinforcement` | Platform & Handling → **Chassis** (keep or merge into Conversions — decide) |
| `Street Chassis Reinforcement / Roll Cage` | same |
| `Full Cage` | same |
| `Custom Control Arms (steering angle)` | Alignment |
| `Off-Road Front Bumper` | Aero |
| `Race Front Bumper` | Aero |
| `Race Rear Wing` | Aero |
| `Sport Weight Reduction` | Platform & Handling → **Chassis** or Conversions |

> **Open item:** Chassis reinforcement, roll cage, weight reduction don't map
> cleanly to a tuning tab. Options: keep a `Chassis` category, or fold into
> `Conversions`. Decide before running the migration.

---

## Tab → implied upgrade mapping table

Defined in code (not in the DB). When a slider is off-stock, look up its tab
here to find the category and the canonical part string to insert.

```typescript
const TAB_UPGRADE_MAP: Record<string, { category: string; part: string }> = {
  tires:        { category: 'Tires',            part: 'Race Tire Compound' },
  alignment:    { category: 'Alignment',         part: 'Race Control Arms' },
  arb:          { category: 'Anti-Roll Bars',    part: 'Race Anti-Roll Bars' },
  springs:      { category: 'Springs & Dampers', part: 'Race Springs & Dampers' },
  damping:      { category: 'Springs & Dampers', part: 'Race Springs & Dampers' },
  aero:         { category: 'Aero',              part: '' }, // front/rear handled separately
  brakes:       { category: 'Brakes',            part: 'Race Brakes' },
  differential: { category: 'Differential',      part: 'Race Differential' },
  // gearing: handled by upgrade → tuning direction only
}
```

**ARB and Aero** have front/rear parts that can be installed independently.
Auto-populate for these checks which groups are off-stock:
- ARB front group off-stock → add `Race Front Anti-Roll Bar`
- ARB rear group off-stock → add `Race Rear Anti-Roll Bar`
- Aero front group off-stock → add `Race Front Bumper`
- Aero rear group off-stock → add `Race Rear Wing`

---

## Structured adjustments requirement

**The auto-populate feature only works on cards with structured adjustment rows**
(full `AdjustmentRow` shape: `tab`, `value`, `stock`, `min`, `max`, etc.).

Most existing cards still have the old free-text format:
```json
{ "name": "Front anti-roll bar", "description": "softened two clicks from stock" }
```

Old-format cards are unaffected by auto-populate until their adjustments are
migrated to the structured format. This is intentional — no data is lost or
changed on old cards without explicit action.

**Only Smokin currently has fully structured adjustments.** New cards built
with the slider UI will have structured adjustments automatically.

Migration of old cards to structured format is a separate tool — see
`plan-card-migration-tool.md`.

---

## Auto-populate behaviour

- Fires when a slider value changes (`value !== stock`)
- Checks the card's existing upgrades list first — does not add duplicates
- Adds the implied part to the correct category; creates the category if absent
- Does **not** remove parts when a slider returns to stock — the user may have
  installed the upgrade for reasons beyond what the tuner touches
- Front/rear parts (ARB, Aero) are added independently based on which side
  is off-stock
- Gearing: when a Sport or Race transmission is added to Drivetrain upgrades,
  the gearing tab unlocks (tab mode switches from static to adjustable)

---

## Implementation phases

### Phase 0 — Seed data cleanup
- Migrate all upgrade categories to canonical names in `seed/cards.json`
- Split Platform & Handling parts into their correct categories
- Resolve the Chassis open item before running
- Re-import via Admin → Reload from Seed; verify visually

### Phase 1 — Mapping table + store logic
- Add `TAB_UPGRADE_MAP` constant (likely `src/constants/tuning.ts`)
- Add `impliedUpgrades(adjustments, upgrades)` pure function — computes
  what should be added without side effects; testable in isolation
- Add `syncUpgradesFromTuning(cardId)` action to cards store — calls
  `impliedUpgrades`, patches the card's upgrades list, marks card dirty

### Phase 2 — Wire into TuningAdjustments
- Call `syncUpgradesFromTuning` when any slider value changes
- Gearing direction: watch Drivetrain upgrade category for transmission
  entries; toggle the gearing tab mode accordingly

### Phase 3 — Visual feedback (optional)
- In the upgrades list, show a small indicator on auto-populated parts
  so it's clear they were implied by tuning, not manually entered
- Allows easy review and manual override

---

## Open items

- **Chassis category**: decide whether chassis reinforcement / roll cage /
  weight reduction gets its own `Chassis` category or folds into `Conversions`
- **Tier variants**: the implied part is always "Race" quality. If a card
  uses Rally or Off-Road springs, the auto-populate would suggest Race —
  confirm whether to override or leave the existing entry alone
- **Gearing unlock threshold**: does selecting any transmission (Sport or
  Race) unlock the gearing tab, or only Race?
