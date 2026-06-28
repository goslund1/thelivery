<script setup lang="ts">
import { computed, inject, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { ForzaRecipeSection, UpgradeCategory } from '../types'
import { useUiStore } from '../stores/ui'
import { MarkDirtyKey } from '../keys'
import EditableText from './EditableText.vue'
import UpgradesPicker from './UpgradesPicker.vue'
import rawUpgrades from '../data/fh_upgrades.json'

const props = defineProps<{ recipe: ForzaRecipeSection; initialKitOpen?: boolean }>()
const ui = useUiStore()
const markDirty = inject(MarkDirtyKey, () => {})

const CORE_SPEC_KEYS = ['Drivetrain', 'Engine', 'Transmission', 'Tires', 'Suspension']

// Normalize any null/undefined values to '' so the select binding always gets a string.
for (const k of CORE_SPEC_KEYS) {
  if (props.recipe.coreSpecs[k] == null) props.recipe.coreSpecs[k] = ''
}

const hasNonStockSpecs = computed(() =>
  CORE_SPEC_KEYS.some(k => !!props.recipe.coreSpecs[k]?.trim()),
)
const partCount = computed(() =>
  props.recipe.upgrades.reduce((n, c) => n + c.parts.length, 0),
)

// Full upgrade part list for "Show Stock" view mode and cost tallying
type UpgJPart = { part: string; tiers: string[] | 'stepped' | 'cosmetic'; tierCosts?: Record<string, number> }
type UpgJCat  = { name: string; parts: UpgJPart[] }
const allUpgCats = rawUpgrades.categories as UpgJCat[]
const STEPPED_SET = new Set([
  'Front Tire Width', 'Rear Tire Width',
  'Front Rim Size', 'Rear Rim Size',
  'Front Track Width', 'Rear Track Width',
])
function viewInstalledTier(tiers: string[]): string | null {
  for (const cat of props.recipe.upgrades) {
    const hit = tiers.find(t => cat.parts.includes(t))
    if (hit) return hit
  }
  return null
}
function viewPartLabel(part: string, tiers: string[]): string {
  const tier = viewInstalledTier(tiers)
  if (!tier || tier === 'Stock') return 'Stock ' + part
  return tier
}
function isCustomTier(tiers: string[]): boolean {
  const tier = viewInstalledTier(tiers)
  return !!tier && tier !== 'Stock'
}
function viewSteppedValue(partName: string): number {
  for (const cat of props.recipe.upgrades) {
    const entry = cat.parts.find(p => p.startsWith(partName + ' '))
    if (entry) {
      const n = parseInt(entry.slice(partName.length + 1).trim(), 10)
      return isNaN(n) ? 0 : n
    }
  }
  return 0
}
function viewPartCost(p: UpgJPart): number | null {
  if (!p.tierCosts || !Array.isArray(p.tiers)) return null
  const tier = viewInstalledTier(p.tiers)
  if (!tier || tier === 'Stock') return null
  return p.tierCosts[tier] ?? null
}
function viewSteppedLabel(partName: string): string {
  const v = viewSteppedValue(partName)
  if (v === 0) return `Stock ${partName}`
  return `${partName} ${v > 0 ? '+' : ''}${v}`
}

const totalUpgradeCost = computed(() => {
  let total = 0
  for (const cat of allUpgCats) {
    for (const p of cat.parts) {
      if (!Array.isArray(p.tiers) || !p.tierCosts) continue
      const hit = viewInstalledTier(p.tiers)
      if (hit) total += p.tierCosts[hit] ?? 0
    }
  }
  return total
})

// Dropdown options for each core-spec column.
// Engine list covers the common swap families across FH5 and FH6;
// the Tires list strips the " Tire Compound" suffix to match the stored format.
const SPEC_OPTIONS: Record<string, string[]> = {
  Drivetrain:   ['RWD', 'AWD', 'FWD'],
  Engine: [
    '1.0L I3T', '1.5L I3T', '1.6L I4T', '2.0L I4T', '2.0L I4 TT',
    '2.3L I4T', '2.5L F4T', '3.0L I6T', '3.0L V6-TT', '3.5L V6 TT',
    '3.8L V6 TT', '4.0L F6 TT', '4.5L V8 TT', '5.0L V8', '5.2L V8',
    '5.7L V8', '6.2L V8', '6.2L V8 SC', '7.0L V8', '7.3L V8',
    '6.5L V12', '7.0L V12', 'Electric',
  ],
  Transmission: ['Sport', '5-Speed Race', '6-Speed Race', '7-Speed Race', '8-Speed Race', '9-Speed Race', '10-Speed Race', 'Drift'],
  Tires: ['Street', 'Sport', 'Semi-Slick Race', 'Horizon Semi-Slick Race', 'Slick Race', 'Drift', 'Rally', 'Offroad Race', 'Snow', 'Drag', 'Vintage Race', 'Vintage White Wall'],
  Suspension:   ['Street', 'Sport', 'Race', 'Rally', 'Drift'],
}

function onSpecChange(key: string, e: Event) {
  props.recipe.coreSpecs[key] = (e.target as HTMLSelectElement).value
  markDirty()
}

// Share code — format the raw input as XXX XXX XXX on every keystroke.
function formatShareCode(raw: string): string {
  const d = raw.replace(/\D/g, '').slice(0, 9)
  if (d.length <= 3) return d
  if (d.length <= 6) return `${d.slice(0, 3)} ${d.slice(3)}`
  return `${d.slice(0, 3)} ${d.slice(3, 6)} ${d.slice(6)}`
}
function onShareCodeInput(e: Event) {
  const input = e.target as HTMLInputElement
  const formatted = formatShareCode(input.value)
  props.recipe.shareCode = formatted
  // Rewrite value to insert spaces; cursor goes to end which is acceptable for a code field.
  input.value = formatted
  markDirty()
}

// The Upgrades sub-list follows its own filter checkbox + expand/collapse-all.
const kitOpen = ref(props.initialKitOpen ?? false)
watch(() => ui.upgradesExpanded, (v) => (kitOpen.value = v))
function onKitToggle(e: Event) {
  kitOpen.value = (e.target as HTMLDetailsElement).open
}

// ── Upgrade presets (localStorage) ───────────────────────────────────────────
interface Preset { name: string; upgrades: UpgradeCategory[] }
const STORE_KEY      = 'tl-recipe-presets'
const presets        = ref<Preset[]>([])
const showPresetMenu = ref(false)
const showSaveRow    = ref(false)
const saveNameInput  = ref('')
const activeName     = ref('')
const showStock      = ref(props.recipe.showStock ?? false)
const presetBarEl    = ref<HTMLElement | null>(null)

function loadPresets() {
  try { presets.value = JSON.parse(localStorage.getItem(STORE_KEY) ?? '[]') }
  catch { presets.value = [] }
}
loadPresets()

function persistPresets() { localStorage.setItem(STORE_KEY, JSON.stringify(presets.value)) }

function applyPreset(p: Preset) {
  props.recipe.upgrades.splice(0, props.recipe.upgrades.length, ...JSON.parse(JSON.stringify(p.upgrades)))
  activeName.value = p.name
  markDirty()
  showPresetMenu.value = false
}

function saveAsPreset() {
  const name = saveNameInput.value.trim()
  if (!name) return
  loadPresets()
  presets.value.push({ name, upgrades: JSON.parse(JSON.stringify(props.recipe.upgrades)) })
  persistPresets()
  activeName.value    = name
  saveNameInput.value = ''
  showSaveRow.value   = false
}

function deletePreset(i: number) {
  if (presets.value[i].name === activeName.value) activeName.value = ''
  presets.value.splice(i, 1)
  persistPresets()
}

function clearAllUpgrades() {
  props.recipe.upgrades.splice(0)
  activeName.value = ''
  markDirty()
}

function onPresetDocClick(e: MouseEvent) {
  if (showPresetMenu.value && presetBarEl.value && !presetBarEl.value.contains(e.target as Node)) {
    showPresetMenu.value = false
    showSaveRow.value    = false
  }
}
onMounted(()      => document.addEventListener('mousedown', onPresetDocClick))
onBeforeUnmount(() => document.removeEventListener('mousedown', onPresetDocClick))
</script>

<template>
  <div class="section-body">
    <div class="tune-header">
      <EditableText tag="p" class="tune-name" v-model="recipe.tuneName" />
      <div class="plate">
        SHARE CODE:
        <input
          v-if="ui.isEditing"
          class="share-code-input"
          :value="recipe.shareCode"
          @input="onShareCodeInput"
          placeholder="000 000 000"
          maxlength="11"
          spellcheck="false"
        />
        <b v-else>{{ recipe.shareCode || '—' }}</b>
      </div>
    </div>

    <table v-if="ui.isEditing || hasNonStockSpecs" class="recipe-table">
      <tbody>
        <tr>
          <th v-for="k in CORE_SPEC_KEYS" :key="k">{{ k }}</th>
        </tr>
        <tr>
          <td v-for="k in CORE_SPEC_KEYS" :key="k">
            <template v-if="ui.isEditing">
              <select
                v-if="SPEC_OPTIONS[k]"
                class="spec-select"
                @change="onSpecChange(k, $event)"
              >
                <option value="" :selected="!recipe.coreSpecs[k]">Stock</option>
                <option v-for="opt in SPEC_OPTIONS[k]" :key="opt" :value="opt" :selected="recipe.coreSpecs[k] === opt">{{ opt }}</option>
              </select>
              <EditableText v-else v-model="recipe.coreSpecs[k]" />
            </template>
            <span v-else>{{ recipe.coreSpecs[k] || 'Stock' }}</span>
          </td>
        </tr>
      </tbody>
    </table>

    <!-- :class is-open drives the chevron rotation via catalog.css -->
    <details class="kit-toggle" :class="{ 'is-open': kitOpen }" :open="kitOpen" @toggle="onKitToggle">
      <summary title="Click to expand or collapse the full parts list">
        <span class="kit-label-group">
          <span class="section-label">Upgrades Installed</span> — {{ partCount }} parts<template v-if="totalUpgradeCost > 0"> › CR {{ totalUpgradeCost.toLocaleString() }}</template>
        </span>
        <button class="kit-stock-btn" type="button" :class="{ active: showStock }" @click.stop="showStock = !showStock">{{ showStock ? 'Hide Stock' : 'Show Stock' }}</button>
        <span class="chev"></span>
      </summary>

      <!-- Preset bar: spans the full section width, only visible in edit mode -->
      <div v-if="ui.isEditing" class="kit-preset-bar" ref="presetBarEl">
        <button class="kit-preset-trigger" type="button" @click="showPresetMenu = !showPresetMenu">
          {{ activeName || '— no preset —' }}
        </button>
        <div class="kit-preset-menu" v-show="showPresetMenu">
          <div v-if="presets.length" class="up-preset-list">
            <div v-for="(p, i) in presets" :key="i" class="up-preset-item">
              <button class="up-preset-apply" type="button" @click="applyPreset(p)">{{ p.name }}</button>
              <button class="up-preset-del"   type="button" @click="deletePreset(i)">×</button>
            </div>
            <div class="up-preset-divider"></div>
          </div>
          <div class="up-preset-empty" v-else>No presets saved</div>
          <template v-if="!showSaveRow">
            <button class="up-preset-save-link" type="button" @click="showSaveRow = true">
              + Save current as preset
            </button>
          </template>
          <template v-else>
            <div class="up-preset-save-row">
              <input class="up-preset-name" v-model="saveNameInput" placeholder="Preset name…"
                     @keydown.enter="saveAsPreset" @keydown.escape="showSaveRow = false" />
              <button class="up-preset-confirm" type="button" @click="saveAsPreset">Save</button>
              <button class="up-preset-cancel"  type="button" @click="showSaveRow = false">×</button>
            </div>
          </template>
          <div class="up-preset-divider"></div>
          <button class="up-preset-clear" type="button" @click="clearAllUpgrades(); showPresetMenu = false">Clear All</button>
        </div>
      </div>

      <div class="kit-body">
        <UpgradesPicker v-if="ui.isEditing" :upgrades="recipe.upgrades" :show-stock="showStock" />
        <template v-else-if="showStock">
          <!-- Show Stock: full list at equal visual weight, stock items shown as plain text -->
          <div class="upgrades-grid">
            <div v-for="cat in allUpgCats" :key="cat.name" class="kit-cat">
              <p class="kit-cat-label">{{ cat.name }}</p>
              <ul class="kit-list">
                <template v-for="p in cat.parts" :key="p.part">
                  <li v-if="Array.isArray(p.tiers)" :class="{ 'kit-item--buy': isCustomTier(p.tiers) }">
                    {{ viewPartLabel(p.part, p.tiers) }}<span v-if="viewPartCost(p) !== null" class="kit-item-cost"> · CR {{ viewPartCost(p)!.toLocaleString() }}</span>
                  </li>
                  <li v-else-if="p.tiers === 'stepped' && STEPPED_SET.has(p.part)" :class="{ 'kit-item--buy': viewSteppedValue(p.part) !== 0 }">
                    {{ viewSteppedLabel(p.part) }}
                  </li>
                </template>
              </ul>
            </div>
          </div>
        </template>
        <template v-else>
          <!-- Default: only show installed (non-stock) upgrades -->
          <div class="upgrades-grid">
            <div v-for="(cat, ci) in recipe.upgrades" :key="ci" class="kit-cat">
              <p class="kit-cat-label">{{ cat.category }}</p>
              <ul class="kit-list">
                <li v-for="(part, pi) in cat.parts" :key="pi">{{ part }}</li>
              </ul>
            </div>
            <p v-if="!recipe.upgrades.length" class="kit-cat-label" style="opacity:0.35">No upgrades recorded</p>
          </div>
        </template>
      </div>
    </details>

    <p class="kit-cat-label adj-label">Tune Adjustments</p>
    <div class="adjustments-box">
      <ul class="recipe-adjustments">
        <li v-for="(adj, ai) in recipe.adjustments" :key="ai">
          <EditableText tag="b" v-model="adj.name" /> —
          <EditableText tag="span" v-model="adj.description" />
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
/* Dropdown for each core-spec cell */
.spec-select {
  background: none;
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: inherit;
  font-family: inherit;
  font-size: inherit;
  padding: 1px 4px;
  cursor: pointer;
  width: 100%;
  box-sizing: border-box;
}
.spec-select:focus {
  outline: 1px solid var(--gold);
  outline-offset: 1px;
}
.kit-item-cost {
  color: var(--gold);
  font-family: 'JetBrains Mono', monospace;
  font-size: 9px;
  letter-spacing: 0.02em;
  opacity: 0.8;
}
/* Share code input: invisible field, styled to match the <b> it replaces */
.share-code-input {
  background: none;
  border: none;
  border-bottom: 1px solid var(--panel-edge);
  color: inherit;
  font-family: inherit;
  font-size: inherit;
  font-weight: bold;
  letter-spacing: inherit;
  padding: 0 2px;
  width: 7.5em;
}
.share-code-input:focus {
  outline: none;
  border-bottom-color: var(--gold);
}
.share-code-input::placeholder { opacity: 0.35; font-weight: normal; }

/* Kill catalog.css's multi-column on kit-body so our grid controls layout */
.kit-body {
  column-width: auto;
  column-count: auto;
}

.upgrades-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
  align-items: start;
}

/* ── Preset bar ─────────────────────────────────────────────────────────────── */
.kit-preset-bar {
  position: relative;
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 6px 10px 7px;
  border-bottom: 1px solid var(--panel-edge);
  background: color-mix(in srgb, var(--panel) 60%, transparent);
}
.kit-preset-trigger {
  flex: 1;
  background: none;
  border: none;
  color: var(--steel);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.04em;
  padding: 0;
  text-align: left;
  cursor: pointer;
  opacity: 0.5;
  transition: opacity 0.12s, color 0.12s;
}
.kit-preset-trigger:hover { opacity: 1; color: var(--gold); }
/* Make the label take all available space so the button + chevron sit together on the right */
.kit-label-group { flex: 1; }

.kit-stock-btn {
  background: none;
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: var(--steel);
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  padding: 2px 7px;
  margin-right: 10px;
  cursor: pointer;
  opacity: 0.55;
  transition: color 0.12s, opacity 0.12s, border-color 0.12s;
  flex-shrink: 0;
}
.kit-stock-btn:hover { opacity: 1; }
.kit-stock-btn.active { color: var(--gold); border-color: var(--gold); opacity: 1; }

/* Preset dropdown menu */
.kit-preset-menu {
  position: absolute;
  top: calc(100% + 2px);
  left: 0;
  z-index: 200;
  min-width: 210px;
  background: var(--panel);
  border: 1px solid var(--panel-edge);
  border-radius: 6px;
  padding: 6px 0;
  box-shadow: 0 6px 20px rgba(0,0,0,0.45);
}
.up-preset-list { padding: 0 0 4px; }
.up-preset-item { display: flex; align-items: center; gap: 4px; padding: 0 8px; }
.up-preset-apply {
  flex: 1; background: none; border: none; text-align: left;
  color: var(--steel); font-size: 12px; padding: 5px 4px; cursor: pointer; border-radius: 3px;
}
.up-preset-apply:hover { color: var(--gold); }
.up-preset-del {
  background: none; border: none; color: var(--steel); opacity: 0.4;
  font-size: 14px; cursor: pointer; padding: 2px 4px; line-height: 1;
}
.up-preset-del:hover { opacity: 1; color: #e03030; }
.up-preset-divider { height: 1px; background: var(--panel-edge); margin: 4px 8px; }
.up-preset-empty { font-size: 11px; color: var(--steel); opacity: 0.4; padding: 4px 12px 8px; }
.up-preset-save-link {
  background: none; border: none; color: var(--gold); font-size: 11px;
  padding: 4px 12px; cursor: pointer; width: 100%; text-align: left; opacity: 0.8;
}
.up-preset-save-link:hover { opacity: 1; }
.up-preset-save-row { display: flex; align-items: center; gap: 4px; padding: 4px 8px; }
.up-preset-name {
  flex: 1;
  background: color-mix(in srgb, var(--panel) 70%, #000);
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: var(--paper); font-size: 11px; padding: 3px 6px;
}
.up-preset-confirm {
  background: none; border: 1px solid var(--gold); border-radius: 3px;
  color: var(--gold); font-size: 10px; padding: 3px 8px; cursor: pointer;
}
.up-preset-cancel {
  background: none; border: none; color: var(--steel); opacity: 0.5;
  font-size: 14px; padding: 2px 4px; cursor: pointer; line-height: 1;
}
.up-preset-clear {
  background: none; border: none; color: var(--steel); font-size: 11px;
  opacity: 0.4; cursor: pointer; padding: 5px 12px; width: 100%; text-align: left;
}
.up-preset-clear:hover { opacity: 0.9; color: #e03030; }

.kit-item--buy { color: var(--gold); }
</style>
