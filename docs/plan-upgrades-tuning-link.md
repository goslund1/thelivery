# Upgrades ↔ Tuning Link — Plan

Connect the upgrades parts list to the tuning adjustment sliders so that moving
a slider off-stock automatically implies the appropriate upgrade part in the
corresponding category. The gearing tab is the exception where the relationship
runs the other direction.

---

## Canonical upgrade hierarchy

From `FH5 & FH6 Upgrades Hierarchy.md`. Top-level categories and their
subcategories as the game presents them. This is the ground truth for all
data model and UI decisions.

### Engine
Intake, Intake Manifold/Throttle Body, Carburetor, Ignition, Exhaust,
Camshaft, Valves, Displacement, Pistons/Compression, Single Turbo, Twin Turbo,
Intercooler, Oil/Cooling, Flywheel

### Platform and Handling
Brakes, Springs and Dampers, Front Anti-Roll Bars, Rear Anti-Roll Bars,
Chassis Reinforcement / Roll Cage, Weight Reduction

### Drivetrain
Clutch (Street / Sport / Race / None), Transmission, Driveline, Differential

### Tires and Wheels
Tire Compound, Front Tire Width, Rear Tire Width, Rim Style, Front/Rear Rim
Size, Front/Rear Track Width

### Aero and Appearance
Front Bumper, Rear Wing, Rear Bumper

### Body Kits and Conversions
Engine Swap, Drivetrain Swap, Aspiration

---

## Which upgrades unlock which sliders

Tabs are always visible. Sliders within a tab are locked at stock until the
enabling upgrade is installed. Moving a slider off-stock is proof the enabling
upgrade is present — this is the signal the auto-populate uses.

| Tuning tab | Enabled by | Tier requirement |
|---|---|---|
| Tires | **Always unlocked** — tire pressure adjustable on any compound | — |
| Alignment | Springs and Dampers | Race, Rally, or Drift only |
| Springs | Springs and Dampers | Race, Rally, or Drift only |
| Damping | Springs and Dampers | Race, Rally, or Drift only |
| ARB | Front Anti-Roll Bars / Rear Anti-Roll Bars | Any tier; sides are independent |
| Aero | Front Bumper / Rear Wing | Any tier; sides are independent |
| Brakes | Brakes | Any tier |
| Differential | Differential | Any tier |
| Gearing — Final Drive | Transmission | Sport, Race, or Drift |
| Gearing — individual gear ratios | Transmission | Race or Drift only |

**Key notes:**
- Springs and Dampers is the single upgrade that unlocks three tuning tabs at
  once (Alignment, Springs, Damping). Street and Sport tiers do not unlock them.
- ARB front and rear sliders are independent — either side can be absent.
- Aero front bumper and rear wing are independent — each unlocks its own slider.
- Tire pressure is always adjustable regardless of compound grade.
- Chassis Reinforcement / Roll Cage and Weight Reduction (both subcategories of
  Platform and Handling) do not enable any tuning sliders.
- Clutch (Drivetrain subcategory) does not enable any tuning sliders.

---

## The auto-populate rules

**Tuning drives upgrades (all tabs except gearing):**
Any slider where `value !== stock` implies the enabling upgrade is installed.
Auto-populate adds the appropriate part to the upgrades list if not already present.
Tier inference: Springs and Dampers defaults to "Race" (the minimum tier that
unlocks the sliders). All other categories: any tier suffices; user corrects
the specific tier manually if needed.

**Upgrades drive tuning (gearing tab):**
Final Drive slider unlocks when Sport, Race, or Drift transmission is listed.
Individual gear ratio sliders unlock when Race or Drift transmission is listed.
The upgrade entry is the meaningful signal — slider position alone cannot
determine which transmission tier is installed.

**Auto-populate does not remove parts** when a slider returns to stock.
The user may have installed the upgrade for reasons the tuner doesn't touch.

---

## Implied upgrade mapping

```typescript
// Defined in src/constants/tuning.ts
// Tires tab omitted — always unlocked, no upgrade implied.
// Gearing omitted — upgrade drives tuning, handled separately.

const SLIDER_UPGRADE_MAP = {
  alignment:    { category: 'Platform and Handling', subcategory: 'Springs and Dampers', impliedTier: 'Race' },
  springs:      { category: 'Platform and Handling', subcategory: 'Springs and Dampers', impliedTier: 'Race' },
  damping:      { category: 'Platform and Handling', subcategory: 'Springs and Dampers', impliedTier: 'Race' },
  arb:          { category: 'Platform and Handling', subcategory: null, impliedTier: null }, // front/rear handled separately
  brakes:       { category: 'Platform and Handling', subcategory: 'Brakes',              impliedTier: null },
  differential: { category: 'Drivetrain',            subcategory: 'Differential',         impliedTier: null },
  aero:         { category: 'Aero and Appearance',   subcategory: null, impliedTier: null }, // front/rear handled separately
}
```

ARB and Aero front/rear are handled by checking which slider groups are off-stock:
- ARB front group off-stock → imply `Front Anti-Roll Bars`
- ARB rear group off-stock → imply `Rear Anti-Roll Bars`
- Aero front group off-stock → imply `Front Bumper`
- Aero rear group off-stock → imply `Rear Wing`

---

## Category string cleanup

Current seed data has inconsistent category names. Canonical replacements:

| Current string | Canonical |
|---|---|
| `Platform & Handling` | `Platform and Handling` |
| `Platform and Handling` | `Platform and Handling` (correct) |
| `Tires & Rims` | `Tires and Wheels` |
| `Tires and Wheels` | `Tires and Wheels` (correct) |
| `Aero and Appearance` | `Aero and Appearance` (correct) |
| `Bodykits and Conversion` | `Body Kits and Conversions` |
| `Body Kits and Conversions` | `Body Kits and Conversions` (correct) |
| `Engine` | `Engine` (correct) |
| `Drivetrain` | `Drivetrain` (correct) |

This is a string normalization only. Visual layout does not change.

---

## Structured adjustments requirement

Auto-populate only works on cards with structured `AdjustmentRow` format
(`tab`, `value`, `stock`, `min`, `max`, etc.). Most existing cards still
have the old free-text format:

```json
{ "name": "Front anti-roll bar", "description": "softened two clicks from stock" }
```

Old-format cards are unaffected until migrated. Only Smokin currently has
fully structured adjustments. New cards built with the slider UI get structured
adjustments automatically.

Migration of old cards → see `plan-card-migration-tool.md`.

---

## Implementation phases

### Phase 0 — Seed data cleanup
- Normalize category name strings in `seed/cards.json`
- Re-import via Admin → Reload from Seed; verify visually

### Phase 1 — Mapping constant + pure logic
- Add `SLIDER_UPGRADE_MAP` to `src/constants/tuning.ts`
- Add `impliedUpgrades(adjustments, upgrades)` pure function
- Add `syncUpgradesFromTuning(cardId)` action to cards store

### Phase 2 — Wire into TuningAdjustments
- Call `syncUpgradesFromTuning` when any slider value changes
- Gearing direction: watch Drivetrain subcategory for transmission entries;
  determine which gearing sliders to unlock based on tier

### Phase 3 — Visual feedback (optional)
- Indicator on auto-populated parts in the upgrades list
- Makes implied vs manually entered parts distinguishable

---

## Open items

- **Springs and Dampers tier variants**: if a card uses Rally or Drift springs,
  auto-populate would suggest Race. Confirm whether to leave existing entries
  alone or update the tier when sliders change.
- **Gearing unlock threshold**: confirm whether Drift transmission unlocks
  individual gear ratios the same as Race, or only Final Drive.
- **None option for Clutch**: Drivetrain → Clutch should support a None entry
  for cars that lack a clutch upgrade slot.
