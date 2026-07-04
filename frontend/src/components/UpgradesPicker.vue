<script setup lang="ts">
import { inject } from 'vue'
import { MarkDirtyKey } from '../keys'
import type { UpgradeCategory } from '../types'
import rawData from '../data/fh_upgrades.json'

const props = defineProps<{ upgrades: UpgradeCategory[]; showStock: boolean; impliedParts?: Set<string> }>()
const markDirty = inject(MarkDirtyKey, () => {})

type UpgradeTiers = string[] | 'stepped' | 'cosmetic'
type JsonPart     = { part: string; tiers: UpgradeTiers; specialTiers?: string[]; tierCosts?: Record<string, number> }
type JsonCategory = { name: string; parts: JsonPart[] }

const categories = rawData.categories as JsonCategory[]

const CAT_ORDER = [
  'Body Kits and Conversions',
  'Engine',
  'Drivetrain',
  'Platform and Handling',
  'Aero and Appearance',
  'Tires and Wheels',
]
const allCats = CAT_ORDER.map(n => categories.find(c => c.name === n)).filter(Boolean) as JsonCategory[]

// Which cats force a new column at each breakpoint
const COL3_BREAK = new Set(['Drivetrain', 'Tires and Wheels'])
const COL2_BREAK = new Set(['Platform and Handling'])

const STEPPED = new Set([
  'Front Tire Width', 'Rear Tire Width',
  'Front Rim Size',   'Rear Rim Size',
  'Front Track Width','Rear Track Width',
])

const STEP_PRESETS = [-1, 0, 1, 2, 3, 5]

function cleanEmpty() {
  for (let i = props.upgrades.length - 1; i >= 0; i--) {
    if (props.upgrades[i].parts.length === 0) props.upgrades.splice(i, 1)
  }
}

function getOrCreate(categoryName: string): UpgradeCategory {
  let cat = props.upgrades.find(u => u.category === categoryName)
  if (!cat) {
    cat = { category: categoryName, parts: [] }
    props.upgrades.push(cat)
  }
  return cat
}

function getInstalledTier(tiers: string[], specialTiers?: string[]): string | null {
  const all = specialTiers ? [...tiers, ...specialTiers] : tiers
  for (const cat of props.upgrades) {
    const hit = all.find(t => cat.parts.includes(t))
    if (hit) return hit
  }
  return null
}

function onTierChange(categoryName: string, allTiers: string[], e: Event, specialTiers?: string[]) {
  const newTier = (e.target as HTMLSelectElement).value
  const current = getInstalledTier(allTiers, specialTiers)
  if (current) {
    for (const cat of props.upgrades) cat.parts = cat.parts.filter(p => p !== current)
  }
  if (newTier) getOrCreate(categoryName).parts.push(newTier)
  cleanEmpty()
  markDirty()
}

function getSteppedValue(partName: string): number {
  for (const cat of props.upgrades) {
    const entry = cat.parts.find(p => p.startsWith(partName + ' '))
    if (entry) {
      const n = parseInt(entry.slice(partName.length + 1).trim(), 10)
      return isNaN(n) ? 0 : n
    }
  }
  return 0
}

function getPresetIndex(partName: string): number {
  const v = getSteppedValue(partName)
  const exact = STEP_PRESETS.indexOf(v)
  if (exact !== -1) return exact
  let best = 0
  let bestDist = Math.abs(STEP_PRESETS[0] - v)
  for (let i = 1; i < STEP_PRESETS.length; i++) {
    const d = Math.abs(STEP_PRESETS[i] - v)
    if (d < bestDist) { bestDist = d; best = i }
  }
  return best
}

function nudgeStepped(categoryName: string, partName: string, dir: 1 | -1) {
  const next = STEP_PRESETS[Math.max(0, Math.min(STEP_PRESETS.length - 1, getPresetIndex(partName) + dir))]
  for (const cat of props.upgrades) {
    cat.parts = cat.parts.filter(p => !p.startsWith(partName + ' '))
  }
  if (next !== 0) getOrCreate(categoryName).parts.push(`${partName} ${next > 0 ? '+' : ''}${next}`)
  cleanEmpty()
  markDirty()
}

function isImplied(partName: string, tiers: string[], specialTiers?: string[]): boolean {
  if (!props.impliedParts?.has(partName)) return false
  const tier = getInstalledTier(tiers, specialTiers)
  return !!tier && tier !== 'No Upgrade' && tier !== 'Not Available'
}

function displayName(s: string): string {
  return s.replace(/^Chassis /i, '').replace(/^Intake (?=Manifold)/i, '')
}

function steppedLabel(partName: string): string {
  const v = getSteppedValue(partName)
  return v === 0 ? 'stock' : (v > 0 ? '+' : '') + v
}

</script>

<template>
  <div class="up-picker">
    <div
      v-for="cat in allCats"
      :key="cat.name"
      class="kit-cat"
      :class="{
        'up-col3-break': COL3_BREAK.has(cat.name),
        'up-col2-break': COL2_BREAK.has(cat.name),
      }"
    >
      <p class="kit-cat-label">{{ cat.name }}</p>
      <ul class="kit-list">
        <template v-for="p in cat.parts" :key="p.part">
          <template v-if="p.tiers === 'cosmetic'" />

          <li
            v-else-if="p.tiers === 'stepped' && STEPPED.has(p.part)"
            class="up-step-row"
          >
            <button
              class="up-step-btn" type="button"
              :disabled="getPresetIndex(p.part) <= 0"
              @click="nudgeStepped(cat.name, p.part, -1)"
            >‹</button>
            <span class="up-step-val" :class="{ 'up-step-set': getSteppedValue(p.part) !== 0 }">
              {{ steppedLabel(p.part) }}
            </span>
            <button
              class="up-step-btn" type="button"
              :disabled="getPresetIndex(p.part) >= STEP_PRESETS.length - 1"
              @click="nudgeStepped(cat.name, p.part, +1)"
            >›</button>
            <span class="up-step-lbl">{{ displayName(p.part) }}</span>
          </li>

          <li v-else-if="Array.isArray(p.tiers)">
            <select
              class="up-inline-select"
              :class="{ 'up-inline-set': !!getInstalledTier(p.tiers, p.specialTiers) || props.showStock }"
              :value="getInstalledTier(p.tiers, p.specialTiers) ?? ''"
              @change="onTierChange(cat.name, p.tiers, $event, p.specialTiers)"
            >
              <option value="">Stock {{ displayName(p.part) }}</option>
              <option v-for="tier in p.tiers" :key="tier" :value="tier">{{ displayName(tier) }}</option>
              <template v-if="p.specialTiers?.length">
                <optgroup label="─────────" />
                <option v-for="tier in p.specialTiers" :key="tier" :value="tier">{{ tier }}</option>
              </template>
            </select>
            <span
              v-if="isImplied(p.part, p.tiers, p.specialTiers)"
              class="up-implied-badge"
              title="Auto-populated from tuning sliders"
            >⚡</span>
          </li>
        </template>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.up-picker {
  columns: 3;
  column-gap: 16px;
  padding: 0;
}
.kit-cat {
  break-inside: avoid;
  margin-bottom: 16px;
  min-width: 0;
}
/* At 3 cols: Drivetrain and Tires start a new column */
.up-col3-break { break-before: column; }

@media (max-width: 800px) {
  .up-picker { columns: 2; }
  /* Remove 3-col breaks, add 2-col break at Platform */
  .up-col3-break { break-before: auto; }
  .up-col2-break { break-before: column; }
}
@media (max-width: 540px) {
  .up-picker { columns: 1; }
  .up-col2-break { break-before: auto; }
}

.up-picker :deep(.kit-list > li:not(.up-step-row)) {
  position: relative;
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 0 4px;
}
.up-picker :deep(.kit-list > li:not(.up-step-row))::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(to right,
    color-mix(in srgb, var(--panel-edge) 75%, transparent) 0%,
    transparent 68%
  );
  pointer-events: none;
}

.up-inline-select {
  flex: 1;
  background: none;
  border: none;
  color: inherit;
  font-family: inherit;
  font-size: inherit;
  padding: 0;
  margin: 0;
  cursor: pointer;
  opacity: 0.65;
  min-width: 0;
  -webkit-appearance: auto;
  appearance: auto;
}
.up-inline-select:hover { opacity: 1; }
.up-inline-select:focus { outline: 1px solid var(--accent); border-radius: 2px; outline-offset: 1px; }
.up-inline-set { color: var(--accent); opacity: 1; }

.up-step-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 0;
  list-style: none;
}
.up-step-lbl { flex: 1; opacity: 0.6; margin-left: 2px; }
.up-step-btn {
  background: color-mix(in srgb, var(--panel-edge) 35%, transparent);
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: var(--fg);
  font-size: 13px;
  line-height: 1;
  width: 20px;
  height: 20px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.up-step-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.up-step-btn:disabled { opacity: 0.2; cursor: default; }
.up-step-val {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: inherit;
  opacity: 0.65;
  min-width: 30px;
  text-align: center;
}
.up-step-set { color: var(--accent); opacity: 1; }

.up-implied-badge {
  flex-shrink: 0;
  font-size: 9px;
  opacity: 0.5;
  cursor: default;
  line-height: 1;
}
.up-implied-badge:hover { opacity: 0.9; }
</style>
