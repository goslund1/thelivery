<script setup lang="ts">
import { inject } from 'vue'
import { MarkDirtyKey } from '../keys'
import type { UpgradeCategory } from '../types'
import rawData from '../data/fh_upgrades.json'

const props = defineProps<{ upgrades: UpgradeCategory[]; showStock: boolean }>()
const markDirty = inject(MarkDirtyKey, () => {})

type UpgradeTiers = string[] | 'stepped' | 'cosmetic'
type JsonPart     = { part: string; tiers: UpgradeTiers; tierCosts?: Record<string, number> }
type JsonCategory = { name: string; parts: JsonPart[] }

const categories = rawData.categories as JsonCategory[]

const STEPPED: Record<string, { min: number; max: number }> = {
  'Front Tire Width':  { min: -1, max: 7 },
  'Rear Tire Width':   { min: -1, max: 7 },
  'Front Rim Size':    { min: -1, max: 5 },
  'Rear Rim Size':     { min: -1, max: 5 },
  'Front Track Width': { min: -1, max: 5 },
  'Rear Track Width':  { min: -1, max: 5 },
}

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

function getInstalledTier(tiers: string[]): string | null {
  for (const cat of props.upgrades) {
    const hit = tiers.find(t => cat.parts.includes(t))
    if (hit) return hit
  }
  return null
}

function onTierChange(categoryName: string, allTiers: string[], e: Event) {
  const newTier = (e.target as HTMLSelectElement).value
  const current = getInstalledTier(allTiers)
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

function partCost(p: JsonPart): number | null {
  if (!p.tierCosts) return null
  const installed = getInstalledTier(p.tiers as string[])
  if (!installed) return null
  return p.tierCosts[installed] ?? null
}

function adjustStepped(categoryName: string, partName: string, delta: number) {
  const cfg = STEPPED[partName]
  if (!cfg) return
  const next = Math.max(cfg.min, Math.min(cfg.max, getSteppedValue(partName) + delta))
  for (const cat of props.upgrades) {
    cat.parts = cat.parts.filter(p => !p.startsWith(partName + ' '))
  }
  if (next !== 0) getOrCreate(categoryName).parts.push(`${partName} ${next > 0 ? '+' : ''}${next}`)
  cleanEmpty()
  markDirty()
}

</script>

<template>
  <div class="up-picker">
    <div
      v-for="cat in categories"
      :key="cat.name"
      class="kit-cat"
    >
      <p class="kit-cat-label">{{ cat.name }}</p>
      <ul class="kit-list">
        <template v-for="p in cat.parts" :key="p.part">
          <template v-if="p.tiers === 'cosmetic'" />

          <li
            v-else-if="p.tiers === 'stepped' && STEPPED[p.part]"
            class="up-step-row"
          >
            <span class="up-step-lbl">{{ p.part }}</span>
            <button
              class="up-step-btn" type="button"
              :disabled="getSteppedValue(p.part) <= STEPPED[p.part].min"
              @click="adjustStepped(cat.name, p.part, -1)"
            >−</button>
            <span class="up-step-val">
              {{ getSteppedValue(p.part) === 0
                  ? 'stock'
                  : (getSteppedValue(p.part) > 0 ? '+' : '') + getSteppedValue(p.part) }}
            </span>
            <button
              class="up-step-btn" type="button"
              :disabled="getSteppedValue(p.part) >= STEPPED[p.part].max"
              @click="adjustStepped(cat.name, p.part, +1)"
            >+</button>
          </li>

          <li v-else-if="Array.isArray(p.tiers)">
            <select
              class="up-inline-select"
              :class="{ 'up-inline-set': !!getInstalledTier(p.tiers) || props.showStock }"
              :value="getInstalledTier(p.tiers) ?? ''"
              @change="onTierChange(cat.name, p.tiers, $event)"
            >
              <option value="">Stock {{ p.part }}</option>
              <option v-for="tier in p.tiers" :key="tier" :value="tier">{{ tier }}</option>
            </select>
            <span v-if="partCost(p) !== null" class="up-cost">CR {{ partCost(p)!.toLocaleString() }}</span>
          </li>
        </template>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.up-picker {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
  align-items: start;
  padding: 0;
}

/* Replace bullet dots with dropdown indicator */
.up-picker :deep(.kit-list) {
  list-style: none;
  padding-left: 0;
}
.up-picker :deep(.kit-list > li:not(.up-step-row)) {
  position: relative;
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 0 4px;
}
.up-picker :deep(.kit-list > li:not(.up-step-row))::before {
  content: '▾';
  color: var(--gold);
  opacity: 0.6;
  font-size: 10px;
  flex-shrink: 0;
  line-height: 1;
}
/* Gradient underline: solid from left edge of text, fades before the native select arrow */
.up-picker :deep(.kit-list > li:not(.up-step-row))::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 14px;
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
  color: var(--text);
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
.up-inline-select:focus { outline: 1px solid var(--gold); border-radius: 2px; outline-offset: 1px; }
.up-inline-set { color: var(--gold); opacity: 1; }

.up-cost {
  font-family: 'JetBrains Mono', monospace;
  font-size: 9px;
  color: var(--text);
  opacity: 0.38;
  flex-shrink: 0;
  white-space: nowrap;
  letter-spacing: 0.02em;
}

.up-step-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 0;
  list-style: none;
}
.up-step-lbl { flex: 1; opacity: 0.6; }
.up-step-btn {
  background: color-mix(in srgb, var(--panel-edge) 35%, transparent);
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: var(--text);
  font-size: 13px;
  line-height: 1;
  width: 20px;
  height: 20px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.up-step-btn:hover:not(:disabled) { border-color: var(--gold); color: var(--gold); }
.up-step-btn:disabled { opacity: 0.2; cursor: default; }
.up-step-val {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--gold);
  min-width: 30px;
  text-align: center;
}
</style>
