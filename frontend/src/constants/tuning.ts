import type { AdjustmentRow, UpgradeCategory } from '../types'

// ── Slider → upgrade mapping ──────────────────────────────────────────────────
// Tires tab omitted — always unlocked, no upgrade implied.
// Gearing omitted — upgrade drives tuning, handled by the reverse direction.

interface SlotMapping {
  category: string
  subcategory: string | null
  impliedTier: string | null  // null = any tier, user corrects if needed
}

export const SLIDER_UPGRADE_MAP: Record<string, SlotMapping> = {
  alignment: { category: 'Platform and Handling', subcategory: 'Springs and Dampers', impliedTier: null },
  springs:   { category: 'Platform and Handling', subcategory: 'Springs and Dampers', impliedTier: null },
  damping:   { category: 'Platform and Handling', subcategory: 'Springs and Dampers', impliedTier: null },
  brakes:    { category: 'Platform and Handling', subcategory: 'Brakes',              impliedTier: null },
  arb:       { category: 'Platform and Handling', subcategory: null, impliedTier: null }, // front/rear handled below
  aero:      { category: 'Aero and Appearance',   subcategory: null, impliedTier: null }, // front/rear handled below
  differential: { category: 'Drivetrain',         subcategory: 'Differential',         impliedTier: null },
}

// Groups whose sliders map to a specific subcategory side (ARB / Aero)
const ARB_FRONT_GROUPS  = new Set(['Front Anti-Roll Bar'])
const ARB_REAR_GROUPS   = new Set(['Rear Anti-Roll Bar'])
const AERO_FRONT_GROUPS = new Set(['Front Downforce', 'Front Bumper'])
const AERO_REAR_GROUPS  = new Set(['Rear Downforce', 'Rear Wing'])

// ── Public API ────────────────────────────────────────────────────────────────

export interface ImpliedUpgradesResult {
  toAdd: { category: string; part: string }[]
  needsSpringsDialog: boolean
}

function hasSubcategory(upgrades: UpgradeCategory[], subcategory: string): boolean {
  return upgrades.some(cat => cat.parts.some(p => p.includes(subcategory)))
}

function hasPart(upgrades: UpgradeCategory[], part: string): boolean {
  return upgrades.some(cat => cat.parts.includes(part))
}

function isOffStock(row: AdjustmentRow): boolean {
  return row.value !== row.stock
}

/**
 * Given the current adjustment rows and installed upgrades, returns which
 * upgrade parts should be added and whether the Springs and Dampers dialog
 * needs to fire (because the tier is ambiguous).
 *
 * Pure — does not mutate anything.
 */
export function impliedUpgrades(
  adjustments: AdjustmentRow[],
  upgrades: UpgradeCategory[],
): ImpliedUpgradesResult {
  const toAdd: { category: string; part: string }[] = []
  let needsSpringsDialog = false

  const offStock = adjustments.filter(isOffStock)
  const tabs = new Set(offStock.map(r => r.tab))

  for (const tab of tabs) {
    if (tab === 'tires' || tab === 'gearing') continue

    const mapping = SLIDER_UPGRADE_MAP[tab]
    if (!mapping) continue

    // ARB — check front/rear groups independently
    if (tab === 'arb') {
      const arbRows = offStock.filter(r => r.tab === 'arb')
      const frontOff = arbRows.some(r => ARB_FRONT_GROUPS.has(r.group))
      const rearOff  = arbRows.some(r => ARB_REAR_GROUPS.has(r.group))
      if (frontOff && !hasPart(upgrades, 'Front Anti-Roll Bars')) {
        toAdd.push({ category: 'Platform and Handling', part: 'Front Anti-Roll Bars' })
      }
      if (rearOff && !hasPart(upgrades, 'Rear Anti-Roll Bars')) {
        toAdd.push({ category: 'Platform and Handling', part: 'Rear Anti-Roll Bars' })
      }
      continue
    }

    // Aero — check front/rear groups independently
    if (tab === 'aero') {
      const aeroRows = offStock.filter(r => r.tab === 'aero')
      const frontOff = aeroRows.some(r => AERO_FRONT_GROUPS.has(r.group))
      const rearOff  = aeroRows.some(r => AERO_REAR_GROUPS.has(r.group))
      if (frontOff && !hasPart(upgrades, 'Front Bumper')) {
        toAdd.push({ category: 'Aero and Appearance', part: 'Front Bumper' })
      }
      if (rearOff && !hasPart(upgrades, 'Rear Wing')) {
        toAdd.push({ category: 'Aero and Appearance', part: 'Rear Wing' })
      }
      continue
    }

    // Springs/Dampers group (alignment, springs, damping) — ambiguous tier
    if (tab === 'alignment' || tab === 'springs' || tab === 'damping') {
      if (!hasSubcategory(upgrades, 'Springs and Dampers')) {
        needsSpringsDialog = true
      }
      continue
    }

    // All other tabs: single subcategory, any tier
    if (mapping.subcategory && !hasSubcategory(upgrades, mapping.subcategory)) {
      toAdd.push({ category: mapping.category, part: mapping.subcategory })
    }
  }

  return { toAdd, needsSpringsDialog }
}

/**
 * Apply a Springs and Dampers tier choice to the upgrades list.
 * Called after the user picks Race / Rally / Drift in the dialog.
 */
export function applySpringsChoice(
  upgrades: UpgradeCategory[],
  tier: 'Race' | 'Rally' | 'Drift',
): void {
  const part = `${tier} Springs and Dampers` // matches fh_upgrades.json tier strings
  if (hasPart(upgrades, part)) return
  let cat = upgrades.find(u => u.category === 'Platform and Handling')
  if (!cat) {
    cat = { category: 'Platform and Handling', parts: [] }
    upgrades.push(cat)
  }
  cat.parts.push(part)
}

/**
 * Apply implied upgrades to the upgrades list in place.
 * Skips entries that are already present.
 */
export function applyImpliedUpgrades(
  upgrades: UpgradeCategory[],
  toAdd: { category: string; part: string }[],
): void {
  for (const { category, part } of toAdd) {
    if (hasPart(upgrades, part)) continue
    let cat = upgrades.find(u => u.category === category)
    if (!cat) {
      cat = { category, parts: [] }
      upgrades.push(cat)
    }
    cat.parts.push(part)
  }
}
