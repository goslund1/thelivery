<script setup lang="ts">
import { computed, inject, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import type { AdjustmentRow } from '../types'
import { useUiStore } from '../stores/ui'
import { MarkDirtyKey } from '../keys'
import { api } from '../api'

const props = defineProps<{ adjustments: AdjustmentRow[]; cardId?: string }>()
const emit = defineEmits<{ 'update:adjustments': [rows: AdjustmentRow[]] }>()
const ui = useUiStore()
const markDirty = inject(MarkDirtyKey, () => {})

// ── Canonical structure ───────────────────────────────────────────────────────
// Defines shape only — tab/group/key/label/unit/step.
// No numeric defaults: min/max/stock/value are per-card, set by the user from
// their in-game screen.

interface CanonicalRow {
  key: string
  label: string
  unit: string
  step: number
  locked?: boolean
  lockReason?: string
  bipolar?: boolean
  centerMark?: boolean
}
interface CanonicalGroup {
  title: string
  axis?: [string, string]
  headerUnit?: string
  rows: CanonicalRow[]
}
interface CanonicalTab {
  id: string
  label: string
  deferred?: boolean
  groups?: CanonicalGroup[]
}

const CANONICAL_TABS: CanonicalTab[] = [
  { id: 'tires', label: 'Tires', groups: [
    { title: 'Tire Pressure', axis: ['Low', 'High'], headerUnit: 'PSI', rows: [
      { key: 'tiresFront', label: 'Front', unit: '', step: 0.5 },
      { key: 'tiresRear',  label: 'Rear',  unit: '', step: 0.5 },
    ]},
  ]},
  { id: 'gearing', label: 'Gearing' },
  { id: 'alignment', label: 'Alignment', groups: [
    { title: 'Camber', axis: ['Negative', 'Positive'], rows: [
      { key: 'camberFront', label: 'Front', unit: '°', step: 0.1, bipolar: true },
      { key: 'camberRear',  label: 'Rear',  unit: '°', step: 0.1, bipolar: true },
    ]},
    { title: 'Toe', axis: ['Negative', 'Positive'], rows: [
      { key: 'toeFront', label: 'Front', unit: '°', step: 0.1, bipolar: true },
      { key: 'toeRear',  label: 'Rear',  unit: '°', step: 0.1, bipolar: true },
    ]},
    { title: 'Caster', axis: ['Less', 'More'], rows: [
      { key: 'casterFront', label: 'Front', unit: '°', step: 0.1 },
    ]},
  ]},
  { id: 'arb', label: 'Antiroll Bars', groups: [
    { title: 'Antiroll Bars', axis: ['Soft', 'Stiff'], rows: [
      { key: 'arbFront', label: 'Front', unit: '', step: 0.01 },
      { key: 'arbRear',  label: 'Rear',  unit: '', step: 0.01 },
    ]},
  ]},
  { id: 'springs', label: 'Springs', groups: [
    { title: 'Springs', axis: ['Soft', 'Stiff'], headerUnit: 'LB/IN', rows: [
      { key: 'springFront', label: 'Front', unit: '', step: 0.5 },
      { key: 'springRear',  label: 'Rear',  unit: '', step: 0.5 },
    ]},
    { title: 'Ride Height', axis: ['Low', 'High'], headerUnit: 'IN', rows: [
      { key: 'rideFront', label: 'Front', unit: '', step: 0.1 },
      { key: 'rideRear',  label: 'Rear',  unit: '', step: 0.1 },
    ]},
  ]},
  { id: 'damping', label: 'Damping', groups: [
    { title: 'Rebound Stiffness', axis: ['Soft', 'Stiff'], rows: [
      { key: 'reboundFront', label: 'Front', unit: '', step: 0.1 },
      { key: 'reboundRear',  label: 'Rear',  unit: '', step: 0.1 },
    ]},
    { title: 'Bump Stiffness', axis: ['Soft', 'Stiff'], rows: [
      { key: 'bumpFront', label: 'Front', unit: '', step: 0.1 },
      { key: 'bumpRear',  label: 'Rear',  unit: '', step: 0.1 },
    ]},
  ]},
  { id: 'aero', label: 'Aero', groups: [
    { title: 'Downforce', axis: ['Speed', 'Cornering'], rows: [
      { key: 'aeroFront', label: 'Front', unit: '', step: 1 },
      { key: 'aeroRear',  label: 'Rear',  unit: '', step: 1 },
    ]},
  ]},
  { id: 'brakes', label: 'Brake', groups: [
    { title: 'Balance',  axis: ['Rear', 'Front'],  rows: [{ key: 'brakeBalance',  label: 'Balance',  unit: '%', step: 1, centerMark: true }] },
    { title: 'Pressure', axis: ['Low',  'High'],   rows: [{ key: 'brakePressure', label: 'Pressure', unit: '%', step: 1, centerMark: true }] },
  ]},
  { id: 'differential', label: 'Differential', groups: [
    { title: 'Front', axis: ['Low', 'High'], rows: [
      { key: 'diffFrontAccel', label: 'Acceleration', unit: '%', step: 1 },
      { key: 'diffFrontDecel', label: 'Deceleration', unit: '%', step: 1 },
    ]},
    { title: 'Rear', axis: ['Low', 'High'], rows: [
      { key: 'diffRearAccel', label: 'Acceleration', unit: '%', step: 1 },
      { key: 'diffRearDecel', label: 'Deceleration', unit: '%', step: 1 },
    ]},
    { title: 'Center', axis: ['Front', 'Rear'], rows: [
      { key: 'centerBalance', label: 'Balance', unit: '%', step: 1, centerMark: true },
    ]},
  ]},
]

// ── Gear count (transmission) ─────────────────────────────────────────────────
function ordinal(n: number) {
  const s = n % 100
  if (s >= 11 && s <= 13) return `${n}th`
  const t = n % 10
  return t === 1 ? `${n}st` : t === 2 ? `${n}nd` : t === 3 ? `${n}rd` : `${n}th`
}

function storedGearCount(): number {
  const gears = props.adjustments.filter(r => r.tab === 'gearing' && /^gear\d+$/.test(r.key))
  if (!gears.length) return 6
  return Math.max(...gears.map(r => parseInt(r.key.replace('gear', ''))))
}

const gearCount = ref(storedGearCount())
const GEAR_OPTIONS = [4, 5, 6, 7, 8, 9, 10]

// ── Local row state (edit mode) ───────────────────────────────────────────────
interface LocalRow extends AdjustmentRow {
  locked?: boolean
  lockReason?: string
  _axis?: [string, string]
  _headerUnit?: string
  _bipolar?: boolean
  _centerMark?: boolean
}

function buildGearRows(): LocalRow[] {
  const s = (k: string) => props.adjustments.find(r => r.key === k)
  const rows: LocalRow[] = []
  const fd = s('finalDrive')
  rows.push({
    tab: 'gearing', group: 'Final Drive', key: 'finalDrive',
    label: 'Final Drive', unit: '', step: 0.01,
    min: fd?.min ?? 0, max: fd?.max ?? 100, stock: fd?.stock ?? 0, value: fd?.value ?? 0,
    _axis: ['Short', 'Long'], _headerUnit: undefined,
  })
  for (let i = 1; i <= gearCount.value; i++) {
    const key = `gear${i}`
    const g = s(key)
    rows.push({
      tab: 'gearing', group: 'Gears', key,
      label: ordinal(i), unit: '', step: 0.01,
      min: g?.min ?? 0, max: g?.max ?? 100, stock: g?.stock ?? 0, value: g?.value ?? 0,
      _axis: ['Short', 'Long'], _headerUnit: undefined,
    })
  }
  return rows
}

// ── Tab static/variable modes ─────────────────────────────────────────────────
// Stored as sentinel rows in props.adjustments with key "__mode_<tabId>".
// Default is variable (false). Only static=true tabs are persisted.
const tabModes = ref<Record<string, boolean>>({})

function initTabModes() {
  const modes: Record<string, boolean> = {}
  for (const r of props.adjustments) {
    if (r?.key?.startsWith('__mode_')) {
      modes[r.key.slice(7)] = r.value === 1
    }
  }
  tabModes.value = modes
}

function isTabStatic(tabId: string) {
  return tabModes.value[tabId] === true
}

function setTabMode(tabId: string, isStatic: boolean) {
  const next = { ...tabModes.value }
  if (isStatic) next[tabId] = true
  else delete next[tabId]
  tabModes.value = next
  flush()
}

function buildLocalRows(): LocalRow[] {
  const result: LocalRow[] = []
  for (const tab of CANONICAL_TABS) {
    if (tab.deferred) continue
    if (tab.id === 'gearing') {
      result.push(...buildGearRows())
      continue
    }
    if (!tab.groups) continue
    for (const group of tab.groups) {
      for (const def of group.rows) {
        const stored = props.adjustments.find(r => r.key === def.key)
        result.push({
          tab:   tab.id,
          group: group.title,
          key:   def.key,
          label: def.label,
          unit:  def.unit,
          step:  def.step,
          min:   stored?.min   ?? 0,
          max:   stored?.max   ?? 100,
          stock: stored?.stock ?? 0,
          value: stored?.value ?? 0,
          locked:     def.locked,
          lockReason: def.lockReason,
          _axis:       group.axis,
          _headerUnit: group.headerUnit,
          _bipolar:    def.bipolar,
          _centerMark: def.centerMark,
        })
      }
    }
  }
  return result
}

initTabModes()
const localRows = ref<LocalRow[]>(buildLocalRows())
const endDisplay = ref<Record<string, string>>({})

watch(() => props.adjustments, () => {
  gearCount.value = storedGearCount()
  initTabModes()
  localRows.value = buildLocalRows()
  endDisplay.value = {}
  suggestCollapsed.value = false
}, { deep: false })

watch(gearCount, () => {
  const nonGear = localRows.value.filter(r => r.tab !== 'gearing')
  localRows.value = [...nonGear, ...buildGearRows()]
  flush()
})

function flush() {
  const active = localRows.value
    .filter(r => !r.locked)
    .map(({ locked, lockReason, _axis, _headerUnit, _bipolar, _centerMark, ...r }) => r)
  // Persist static-mode flags as sentinel rows (only non-default static=true entries)
  const sentinels = Object.entries(tabModes.value)
    .filter(([, isStatic]) => isStatic)
    .map(([tabId]) => ({
      key: `__mode_${tabId}`, tab: tabId, group: '', label: '', unit: '',
      step: 1, min: 0, max: 1, stock: 0, value: 1,
    }))
  emit('update:adjustments', [...active, ...sentinels])
  markDirty()
}

// ── View-mode rows ────────────────────────────────────────────────────────────
const storedRows = computed(() => props.adjustments.filter(r => r?.key && typeof r.tab === 'string' && !r.key.startsWith('__mode_')))

// ── Tabs & sections ───────────────────────────────────────────────────────────
const activeTabs = computed(() => {
  if (ui.isEditing) return CANONICAL_TABS.filter(t => !t.deferred)
  const ids = new Set(storedRows.value.map(r => r.tab))
  return CANONICAL_TABS.filter(t => !t.deferred && ids.has(t.id))
})

const activeTabId = ref('')
watch(activeTabs, tabs => {
  if (!tabs.some(t => t.id === activeTabId.value) && tabs.length) activeTabId.value = tabs[0].id
}, { immediate: true })

const stacked = ref(false)
const suppressStackHover = ref(false)
const taRef = ref<HTMLElement | null>(null)
const taNonstockRef = ref<HTMLElement | null>(null)
const sectionRefs = ref<Record<string, HTMLElement | null>>({})

function collapseToTab(sectionId: string) {
  if (stacked.value) {
    const el = sectionRefs.value[sectionId]
    if (!el) return
    const rect = el.getBoundingClientRect()
    window.scrollTo({ top: Math.max(0, window.scrollY + rect.top - 12), behavior: 'smooth' })
  } else {
    activeTabId.value = sectionId
    nextTick(() => {
      const el = taNonstockRef.value ?? taRef.value
      if (!el) return
      const rect = el.getBoundingClientRect()
      window.scrollTo({ top: Math.max(0, window.scrollY + rect.top - 8), behavior: 'smooth' })
    })
  }
}

function focusRowByKey(key: string) {
  nextTick(() => {
    const slider = document.querySelector<HTMLElement>(`.ta-slider[data-key="${key}"]`)
    if (slider) {
      slider.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
      slider.focus({ preventScroll: true })
    }
    focusedKey.value = key
  })
}

function clickNonstockRow(tabId: string, key: string) {
  if (stacked.value) {
    const el = sectionRefs.value[tabId]
    if (el) window.scrollTo({ top: Math.max(0, window.scrollY + el.getBoundingClientRect().top - 12), behavior: 'smooth' })
    // Focus + highlight without scrollIntoView — viewport stays at the section header
    nextTick(() => {
      const slider = document.querySelector<HTMLElement>(`.ta-slider[data-key="${key}"]`)
      if (slider) slider.focus({ preventScroll: true })
      focusedKey.value = key
    })
  } else {
    activeTabId.value = tabId
    nextTick(() => {
      const el = taNonstockRef.value ?? taRef.value
      if (el) window.scrollTo({ top: Math.max(0, window.scrollY + el.getBoundingClientRect().top - 8), behavior: 'smooth' })
    })
    focusRowByKey(key)
  }
}

function switchToTabView(sectionId: string) {
  stacked.value = false
  activeTabId.value = sectionId
  nextTick(() => {
    const el = taNonstockRef.value ?? taRef.value
    if (!el) return
    const rect = el.getBoundingClientRect()
    window.scrollTo({ top: Math.max(0, window.scrollY + rect.top - 8), behavior: 'smooth' })
  })
}

function onTabClick(tabId: string) {
  if (stacked.value) {
    const el = sectionRefs.value[tabId]
    if (!el) return
    const rect = el.getBoundingClientRect()
    const target = window.scrollY + rect.top - 12
    window.scrollTo({ top: Math.max(0, target), behavior: 'smooth' })
  } else {
    switchToTabView(tabId)
  }
}

function groupsForTab(tabId: string) {
  const storedKeys = ui.isEditing ? null : new Set(storedRows.value.map(r => r.key))
  const source = localRows.value.filter(r =>
    r.tab === tabId && (storedKeys === null || storedKeys.has(r.key))
  )

  const map = new Map<string, LocalRow[]>()
  for (const r of source) {
    if (!map.has(r.group)) map.set(r.group, [])
    map.get(r.group)!.push(r)
  }
  return [...map.entries()].map(([title, rows]) => ({
    title,
    axis:       rows[0]._axis,
    headerUnit: rows[0]._headerUnit,
    rows,
  }))
}

const displaySections = computed(() => {
  const tabs = stacked.value
    ? activeTabs.value
    : activeTabs.value.filter(t => t.id === activeTabId.value)
  return tabs.map(tab => ({ id: tab.id, label: tab.label, groups: groupsForTab(tab.id) }))
})

// ── Changed detection ─────────────────────────────────────────────────────────
function isChanged(r: LocalRow | AdjustmentRow) {
  if ((r as LocalRow).locked) return false
  return Math.abs(r.value - r.stock) > r.step / 2
}
function hasChangedInTab(tabId: string) {
  return localRows.value.filter(r => r.tab === tabId).some(r => isChanged(r))
}
const changedRows = computed(() => localRows.value.filter(r => isChanged(r)))
const TAB_LABEL: Record<string, string> = Object.fromEntries(CANONICAL_TABS.map(t => [t.id, t.label]))
const tweakColCount = ref(5)
let tweakResizeObserver: ResizeObserver | null = null

onMounted(() => {
  if (taNonstockRef.value) {
    tweakResizeObserver = new ResizeObserver(([entry]) => {
      const w = entry.contentRect.width
      tweakColCount.value = w >= 880 ? 5 : w >= 700 ? 4 : w >= 500 ? 3 : w >= 320 ? 2 : 1
    })
    tweakResizeObserver.observe(taNonstockRef.value)
  }
})
onUnmounted(() => tweakResizeObserver?.disconnect())

const changedByTab = computed(() => {
  const tabMap = new Map<string, { tabLabel: string; groups: Map<string, { label: string; rows: typeof changedRows.value }> }>()
  for (const r of changedRows.value) {
    if (!tabMap.has(r.tab)) tabMap.set(r.tab, { tabLabel: TAB_LABEL[r.tab] ?? r.tab, groups: new Map() })
    const { tabLabel, groups } = tabMap.get(r.tab)!
    if (!groups.has(r.group)) {
      const display = tabLabel.toLowerCase() !== r.group.toLowerCase() ? `${tabLabel} / ${r.group}` : r.group
      groups.set(r.group, { label: display, rows: [] })
    }
    groups.get(r.group)!.rows.push(r)
  }
  return tabMap
})
type TabBlock = { tabLabel: string; groups: Map<string, { label: string; rows: LocalRow[] }> }

// Distribute tab blocks across explicit columns using a greedy height-balance algorithm.
// Each tab block's height ≈ number of total rows across its groups + groups (for title rows).
const tweakColumns = computed(() => {
  const n = tweakColCount.value
  const cols: Array<Array<[string, TabBlock]>> = Array.from({ length: n }, () => [])
  const heights = new Array(n).fill(0)
  for (const entry of changedByTab.value) {
    const [, { groups }] = entry
    let h = 0
    for (const { rows } of groups.values()) h += rows.length + 1 // +1 for group title
    const shortest = heights.indexOf(Math.min(...heights))
    cols[shortest].push(entry)
    heights[shortest] += h
  }
  return cols
})

// ── Formatting ────────────────────────────────────────────────────────────────
function decimals(r: AdjustmentRow) { return r.step < 0.1 ? 2 : r.step < 1 ? 1 : 0 }
function fmt(r: AdjustmentRow, val: number) { return val.toFixed(decimals(r)) + r.unit }

function pct(r: AdjustmentRow, val: number) {
  if (r.max === r.min) return 0
  return Math.max(0, Math.min(100, (val - r.min) / (r.max - r.min) * 100))
}
function isBipolar(r: AdjustmentRow) {
  return (r as LocalRow)._bipolar || (r.min < 0 && r.max > 0)
}
function trackStyle(r: AdjustmentRow) {
  const color = isChanged(r) ? 'var(--tabc, var(--magenta))' : 'var(--steel-light)'
  const fill = pct(r, r.value)
  const center = isBipolar(r) ? 50 : 0
  return `--fill: ${fill}%; --center: ${center}%; --track-color: ${color}`
}

function thumbPct(r: AdjustmentRow, val: number) {
  const p = r.max === r.min ? 0 : (val - r.min) / (r.max - r.min)
  return { left: `calc(7px + ${p} * (100% - 14px))` }
}

function stockDotStyle(r: AdjustmentRow) {
  const stockP = pct(r, r.stock)
  const valueP = pct(r, r.value)
  const centerP = isBipolar(r) ? 50 : 0
  const inFill = isBipolar(r)
    ? (stockP >= Math.min(centerP, valueP) && stockP <= Math.max(centerP, valueP))
    : stockP <= valueP
  return {
    ...thumbPct(r, r.stock),
    background: 'var(--panel-edge)',
    border: inFill ? '2px solid var(--steel-light)' : 'none',
  }
}
function centerMarkStyle(r: AdjustmentRow) {
  if (isBipolar(r) || (r as LocalRow)._centerMark) return { left: '50%' }
  return thumbPct(r, 0)
}

// ── Normal edit interactions ──────────────────────────────────────────────────
const focusedKey = ref<string | null>(null)

// ── Undo stack ────────────────────────────────────────────────────────────────
const undoStacks = new Map<string, number[]>()
const MAX_UNDO = 20
let sliderDragStart: { key: string; value: number } | null = null

function pushUndo(key: string, value: number) {
  const stack = undoStacks.get(key) ?? []
  stack.push(value)
  if (stack.length > MAX_UNDO) stack.shift()
  undoStacks.set(key, stack)
}

function onSliderMouseDown(r: LocalRow, e: MouseEvent) {
  if (focusedKey.value !== r.key) {
    focusedKey.value = r.key
    // Allow drag if click landed on/near the thumb — block if it was a track click
    const slider = e.currentTarget as HTMLInputElement
    const rect = slider.getBoundingClientRect()
    const p = r.max === r.min ? 0 : (r.value - r.min) / (r.max - r.min)
    const thumbPx = 7 + p * (rect.width - 14)
    const clickPx = e.clientX - rect.left
    if (Math.abs(clickPx - thumbPx) > 12) {
      e.preventDefault()
      return
    }
  }
  sliderDragStart = { key: r.key, value: r.value }
}

function onSliderDragEnd(r: LocalRow) {
  if (sliderDragStart?.key === r.key) {
    pushUndo(r.key, sliderDragStart.value)
    sliderDragStart = null
  }
}

function onRowFocusOut(r: LocalRow, e: FocusEvent) {
  const row = e.currentTarget as HTMLElement
  if (!row.contains(e.relatedTarget as Node) && focusedKey.value === r.key) {
    focusedKey.value = null
  }
}

function onRowClick(r: LocalRow, e: MouseEvent) {
  focusedKey.value = r.key
  if (!(e.target instanceof HTMLInputElement)) {
    ;(e.currentTarget as HTMLElement).focus({ preventScroll: true })
  }
}

function onRowKeydown(r: LocalRow, e: KeyboardEvent) {
if (!ui.isEditing || r.locked) return

  // Cmd/Ctrl+Z — undo last slider move or nudge
  if ((e.metaKey || e.ctrlKey) && e.key === 'z') {
    e.preventDefault()
    const stack = undoStacks.get(r.key)
    if (stack?.length) {
      r.value = stack.pop()!
      flush()
    }
    return
  }

  // Up/Down: move focus to adjacent row
  if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
    e.preventDefault()
    const rows = localRows.value.filter(row => !row.locked)
    const idx = rows.findIndex(row => row.key === r.key)
    const nextIdx = e.key === 'ArrowUp' ? idx - 1 : idx + 1
    if (nextIdx >= 0 && nextIdx < rows.length) {
      const el = document.querySelector<HTMLInputElement>(`.ta-slider[data-key="${rows[nextIdx].key}"]`)
      el?.focus()
    }
    return
  }

  // Left/Right: adjust slider value
  const dir = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0
  if (!dir) return
  e.preventDefault()
  pushUndo(r.key, r.value)
  r.value = parseFloat(Math.max(r.min, Math.min(r.max, r.value + dir * r.step)).toFixed(decimals(r)))
  flush()
}

function onSliderInput(r: LocalRow, e: Event) {
  r.value = parseFloat((e.target as HTMLInputElement).value)
  if (ui.isEditing) flush()
}
function onMinChange(r: LocalRow, e: Event) {
  const raw = (e.target as HTMLInputElement).value
  const v = parseFloat(raw)
  if (!isNaN(v)) {
    r.min = Math.min(v, r.value)
    endDisplay.value[r.key + ':min'] = fmt(r, r.min)
    flush()
  } else {
    endDisplay.value[r.key + ':min'] = raw
  }
}
function onMaxChange(r: LocalRow, e: Event) {
  const raw = (e.target as HTMLInputElement).value
  const v = parseFloat(raw)
  if (!isNaN(v)) {
    r.max = Math.max(v, r.value)
    endDisplay.value[r.key + ':max'] = fmt(r, r.max)
    flush()
  } else {
    endDisplay.value[r.key + ':max'] = raw
  }
}
function onBadgeChange(r: LocalRow, e: Event) {
  const v = parseFloat((e.target as HTMLInputElement).value)
  if (!isNaN(v)) { r.value = Math.max(r.min, Math.min(r.max, v)); flush() }
  ;(e.target as HTMLInputElement).value = fmt(r, r.value)
}

// ── Stock snapshot ────────────────────────────────────────────────────────────
// One-shot: copies all current values to stock. Use once when creating a card.

const stockConfirmOpen = ref(false)
const stockTargetSection = ref<string | null>(null)

function onSetStockParameters(sectionId: string) {
  stockTargetSection.value = sectionId
  stockConfirmOpen.value = true
}

function confirmSetStock(sectionId: string | null) {
  const rows = sectionId ? localRows.value.filter(r => r.tab === sectionId) : localRows.value
  for (const r of rows) r.stock = r.value
  flush()
  stockConfirmOpen.value = false
}

// ── Tuning presets ────────────────────────────────────────────────────────────

type TuningPreset = { id: number; name: string; values: Record<string, number>; createdAt: string }

const presets = ref<TuningPreset[]>([])
const selectedPresetId = ref<number | null>(null)
const presetNameInput = ref('')
const presetNameOpen = ref(false)
const presetBusy = ref(false)
const presetError = ref<string | null>(null)
const deleteConfirmOpen = ref(false)
const applyConfirmOpen  = ref(false)

async function loadPresets() {
  try { presets.value = await api.listTuningPresets() }
  catch (e: unknown) { presetError.value = e instanceof Error ? e.message : String(e) }
}

function applyPreset() {
  if (!selectedPresetId.value) return
  if (changedRows.value.length > 0) { applyConfirmOpen.value = true; return }
  executeApplyPreset()
}

function executeApplyPreset() {
  const preset = presets.value.find(p => p.id === selectedPresetId.value)
  if (!preset) return
  applyConfirmOpen.value = false
  presetError.value = null
  const savedY = window.scrollY
  localRows.value = localRows.value.map(r => {
    if (r.locked) return r
    const updated = { ...r }
    if (r.key in preset.values) {
      updated.value = preset.values[r.key]
      updated.stock = preset.values[r.key]
    }
    if ((r.key + ':min') in preset.values)    updated.min   = preset.values[r.key + ':min']
    if ((r.key + ':max') in preset.values)    updated.max   = preset.values[r.key + ':max']
    return updated
  })
  endDisplay.value = {}
  flush()
  requestAnimationFrame(() => window.scrollTo({ top: savedY, behavior: 'instant' }))
}

async function saveAsPreset() {
  const name = presetNameInput.value.trim()
  if (!name) return
  presetBusy.value = true
  presetError.value = null
  try {
    const values: Record<string, number> = {}
    for (const r of localRows.value) {
      if (r.locked) continue
      values[r.key]          = r.value
      values[r.key + ':min'] = r.min
      values[r.key + ':max'] = r.max
    }
    const created = await api.createTuningPreset({ name, values })
    presets.value.push(created)
    selectedPresetId.value = created.id
    presetNameOpen.value = false
    presetNameInput.value = ''
  } catch (e: unknown) { presetError.value = e instanceof Error ? e.message : String(e) }
  finally { presetBusy.value = false }
}

function deletePreset() {
  if (!selectedPresetId.value) return
  deleteConfirmOpen.value = true
}
async function confirmDeletePreset() {
  if (!selectedPresetId.value) return
  presetBusy.value = true
  presetError.value = null
  deleteConfirmOpen.value = false
  try {
    await api.deleteTuningPreset(selectedPresetId.value)
    presets.value = presets.value.filter(p => p.id !== selectedPresetId.value)
    selectedPresetId.value = presets.value[0]?.id ?? null
  } catch (e: unknown) { presetError.value = e instanceof Error ? e.message : String(e) }
  finally { presetBusy.value = false }
}

watch(() => ui.isEditing, (editing) => { if (editing && !presets.value.length) loadPresets() }, { immediate: true })

// ── Suggestion panel ──────────────────────────────────────────────────────────

const suggestCollapsed = ref(false)
const suggestModalOpen = ref(false)
const suggestTitle = ref('')
const suggestCredit = ref('')
const suggestBusy = ref(false)
const suggestError = ref<string | null>(null)
const suggestDone = ref(false)

const hasSuggestionData = computed(() => storedRows.value.length > 0)

function openSuggestModal() {
  suggestTitle.value = ''
  suggestCredit.value = ''
  suggestError.value = null
  suggestDone.value = false
  suggestModalOpen.value = true
}

async function submitSuggestion() {
  suggestError.value = null
  if (!props.cardId) { suggestError.value = 'Cannot submit: no card context.'; return }
  const title = suggestTitle.value.trim()
  if (!title) { suggestError.value = 'A title is required.'; return }
  if (title.length > 60) { suggestError.value = 'Title must be 60 characters or less.'; return }

  suggestBusy.value = true
  try {
    const adjustments = localRows.value
      .filter(r => !r.locked)
      .map(({ key, tab, group, label, unit, step, min, max, stock, value }) =>
        ({ key, tab, group, label, unit, step, min, max, stock, value })
      )
    await api.submitSuggestion({
      cardId: props.cardId,
      title,
      credit: suggestCredit.value.trim() || undefined,
      adjustments,
    })
    suggestDone.value = true
  } catch (e: unknown) {
    suggestError.value = e instanceof Error ? e.message : 'Something went wrong.'
  } finally {
    suggestBusy.value = false
  }
}
</script>

<template>
  <!-- List of Tweaks — lives above the widget panel -->
  <details class="ta-nonstock" open ref="taNonstockRef">
      <summary class="ta-nonstock-summary">
        <span>List of Tweaks<template v-if="changedRows.length"> ({{ changedRows.length }})</template></span>
        <span class="ta-nonstock-chev"></span>
      </summary>
      <div class="ta-nonstock-body">
        <template v-if="changedRows.length">
          <div v-for="(col, ci) in tweakColumns" :key="ci" class="ta-nonstock-col">
            <div v-for="[tabId, { tabLabel, groups }] in col" :key="tabId" :class="['ta-nonstock-tab-block', 'ta-tab--' + tabId]" role="button" tabindex="0" @click="collapseToTab(tabId)" @keydown.enter.space.prevent="collapseToTab(tabId)">
              <div class="ta-nonstock-tab-header">{{ tabLabel }}</div>
              <div v-for="[groupKey, { rows }] in groups" :key="groupKey" class="ta-nonstock-subgroup">
                <div class="ta-nonstock-subgroup-title">{{ groupKey }}</div>
                <div v-for="r in rows" :key="r.key" class="ta-nonstock-line" role="button" tabindex="-1" @click.stop="clickNonstockRow(tabId, r.key)">
                  <span class="ta-nonstock-loc">{{ r.label }}</span>
                  <span class="ta-nonstock-vals">{{ fmt(r, r.value) }} <span class="ta-nonstock-stock">← {{ fmt(r, r.stock) }}</span></span>
                </div>
              </div>
            </div>
          </div>
        </template>
        <span v-else class="ta-nonstock-empty">Everything matches stock.</span>
      </div>
    </details>

  <div class="ta" ref="taRef">

    <!-- Preset bar (edit mode only) -->
    <div v-if="ui.isEditing" class="ta-preset-bar">
      <select
        v-model="selectedPresetId"
        class="ta-preset-select"
        :disabled="!presets.length"
      >
        <option :value="null" disabled>{{ presets.length ? 'Select preset…' : 'No presets saved' }}</option>
        <option v-for="p in presets" :key="p.id" :value="p.id">{{ p.name }}</option>
      </select>
      <button
        class="ta-btn-lwb ta-preset-btn"
        :disabled="!selectedPresetId || presetBusy"
        @click="applyPreset"
      >Apply</button>
      <button
        class="ta-btn-lwb ta-preset-btn ta-preset-btn--delete"
        :disabled="!selectedPresetId || presetBusy"
        @click="deletePreset"
      >Delete</button>
      <div class="ta-preset-save">
        <template v-if="presetNameOpen">
          <input
            v-model="presetNameInput"
            class="ta-preset-name-input"
            placeholder="Preset name…"
            @keyup.enter="saveAsPreset"
            @keyup.escape="presetNameOpen = false"
          />
          <button class="ta-btn-lwb ta-preset-btn" :disabled="presetBusy" @click="saveAsPreset">{{ presetBusy ? '…' : 'Save' }}</button>
          <button class="ta-btn-lwb ta-preset-btn" @click="presetNameOpen = false">✕</button>
        </template>
        <button v-else class="ta-btn-lwb ta-preset-btn" @click="presetNameOpen = true">Save Current As Preset</button>
      </div>
      <p v-if="presetError" class="ta-preset-error">{{ presetError }}</p>
    </div>

    <!-- Tab bar -->
    <div v-if="activeTabs.length" class="ta-tabbar">
      <button
        v-for="tab in activeTabs" :key="tab.id"
        :class="['ta-tab-btn', 'ta-tab--' + tab.id, { active: !stacked && tab.id === activeTabId }]"
        @click="onTabClick(tab.id)"
      >
        {{ tab.label }}
        <span v-if="hasChangedInTab(tab.id)" class="ta-tab-dot"></span>
      </button>
      <button class="ta-tab-btn ta-tab-btn--stack" :class="{ active: stacked, 'ta-suppress-hover': suppressStackHover }" @click="stacked = !stacked; suppressStackHover = true"
        @mouseleave="suppressStackHover = false">
        {{ stacked ? 'View As Tabs' : 'View Inline' }}
      </button>
    </div>

    <!-- Content -->
    <div v-if="activeTabs.length" class="ta-content">
      <div v-for="section in displaySections" :key="section.id" :class="'ta-tab--' + section.id">
        <div v-if="stacked" class="ta-stack-header" :ref="(el) => sectionRefs[section.id] = el as HTMLElement | null">
          <div class="ta-stack-header-left">
            <span class="ta-caps-nudge">{{ section.label }}<span v-if="hasChangedInTab(section.id)" class="ta-tab-dot ta-tab-dot--inline"></span></span>
            <button v-if="ui.isEditing" class="ta-btn-lwb ta-stack-stock-btn" @click="onSetStockParameters(section.id)">Set As Stock</button>
          </div>
          <button class="ta-btn-lwb ta-stack-collapse-btn" @click="switchToTabView(section.id)">Tab View</button>
        </div>

        <!-- Static / Variable toggle (edit mode only, not on gearing) -->
        <div v-if="ui.isEditing && section.id !== 'gearing'" class="ta-section-title-bar ta-mode-row">
          <div class="ta-title-label-zone">
            <span class="ta-section-title-text ta-caps-nudge">{{ section.groups[0]?.title }}</span>
          </div>
          <div class="ta-title-slider-zone">
            <span class="ta-slider-zone-title ta-caps-nudge">{{ section.groups[0]?.title }}</span>
            <span class="ta-mode-label-text">Range</span>
            <label class="ta-mode-switch">
              <input type="checkbox" :checked="!isTabStatic(section.id)" @change="setTabMode(section.id, !($event.target as HTMLInputElement).checked)" />
              <span class="ta-mode-track">
                <span class="ta-mode-thumb"></span>
              </span>
            </label>
            <span class="ta-mode-state" :class="{ static: isTabStatic(section.id) }">
              {{ isTabStatic(section.id) ? 'Static' : 'Variable' }}
            </span>
          </div>
        </div>

        <!-- Gear count selector (gearing tab only) -->
        <div v-if="section.id === 'gearing' && ui.isEditing" class="ta-section-title-bar ta-gear-select-row">
          <div class="ta-title-label-zone">
            <span class="ta-section-title-text ta-caps-nudge">Transmission</span>
          </div>
          <div class="ta-title-slider-zone">
            <span class="ta-slider-zone-title ta-caps-nudge">Final Drive</span>
          </div>
        </div>

        <div v-for="(group, gi) in section.groups" :key="group.title" class="ta-group">
          <!-- Dedicated title bar for second+ groups -->
          <div v-if="gi > 0" class="ta-section-title-bar">
            <div class="ta-title-label-zone">
              <template v-if="section.id === 'gearing' && ui.isEditing">
                <select class="ta-gear-select ta-caps-nudge" :value="gearCount" @change="gearCount = parseInt(($event.target as HTMLSelectElement).value)">
                  <option v-for="n in GEAR_OPTIONS" :key="n" :value="n">{{ n }}-Speed</option>
                </select>
              </template>
              <span v-else class="ta-section-title-text ta-caps-nudge">{{ group.title }}</span>
            </div>
            <div class="ta-title-slider-zone">
              <span class="ta-slider-zone-title ta-caps-nudge">{{ group.title }}</span>
            </div>
          </div>

          <div class="ta-group-header">
            <div class="ta-left-section">
              <span class="ta-group-title ta-caps-nudge" :style="gi > 0 || ui.isEditing ? { visibility: 'hidden' } : {}">{{ group.title }}</span>
            </div>
            <div class="ta-right-section">
              <template v-if="group.axis">
                <span class="ta-group-axis-col ta-group-axis-col--min ta-caps-nudge">{{ group.axis[0] }}</span>
                <span class="ta-group-axis-track ta-caps-nudge">{{ group.headerUnit ?? '' }}</span>
                <span class="ta-group-axis-col ta-group-axis-col--max ta-caps-nudge">{{ group.axis[1] }}</span>
              </template>
            </div>
          </div>

          <template v-for="r in group.rows" :key="r.key">
            <!-- Locked row -->
            <div v-if="r.locked" class="ta-row ta-row--locked">
              <div class="ta-left-section">
                <div class="ta-row-label">{{ r.label }}</div>
              </div>
              <div class="ta-right-section">
                <div class="ta-lock-line">🔒 Locked</div>
                <div class="ta-lock-reason">{{ r.lockReason }}</div>
              </div>
            </div>

            <!-- Normal row -->
            <div
              v-else
              class="ta-row"
              :class="{ focused: focusedKey === r.key, changed: isChanged(r) }"
              :data-key="r.key"
              tabindex="-1"
              @click="onRowClick(r, $event)"
              @focusin="focusedKey = r.key; if (($event.target as Element)?.tagName !== 'INPUT') ($event.currentTarget as HTMLElement).focus({ preventScroll: true })"
              @focusout="onRowFocusOut(r, $event)"
              @keydown="onRowKeydown(r, $event)"
            >
              <div class="ta-left-section">
                <div class="ta-row-label">{{ r.label }}</div>
                <div class="ta-val-box">
                  <input v-if="ui.isEditing" type="text" class="ta-val-input"
                    :value="fmt(r, r.value)"
                    @change="onBadgeChange(r, $event)"
                    @focus="focusedKey = r.key"
                  />
                  <span v-else>{{ fmt(r, r.value) }}</span>
                </div>
              </div>

              <div class="ta-right-section">
                <!-- Min -->
                <input v-if="ui.isEditing" type="text" class="ta-end-field"
                  :class="{ 'ta-end-field--static': isTabStatic(r.tab) }"
                  :readonly="isTabStatic(r.tab)"
                  :value="endDisplay[r.key + ':min'] ?? fmt(r, r.min)"
                  @change="onMinChange(r, $event)" @focus="focusedKey = r.key"
                />
                <span v-else class="ta-end-label">{{ fmt(r, r.min) }}</span>

                <!-- Track -->
                <div class="ta-track-wrap">
                  <div v-if="isBipolar(r) || r._centerMark" class="ta-center-mark" :style="centerMarkStyle(r)"></div>
                  <div class="ta-stock-tick" :style="stockDotStyle(r)"></div>
                  <input type="range" class="ta-slider"
                    :class="{ 'ta-slider--stock': !isChanged(r) }"
                    :data-key="r.key"
                    :min="r.min" :max="r.max" :step="r.step"
                    :value="r.value"
                    :style="trackStyle(r)"
                    @mousedown="onSliderMouseDown(r, $event)"
                    @change="onSliderDragEnd(r)"
                    @input="onSliderInput(r, $event)"
                  />
                </div>

                <!-- Max -->
                <input v-if="ui.isEditing" type="text" class="ta-end-field"
                  :class="{ 'ta-end-field--static': isTabStatic(r.tab) }"
                  :readonly="isTabStatic(r.tab)"
                  :value="endDisplay[r.key + ':max'] ?? fmt(r, r.max)"
                  @change="onMaxChange(r, $event)" @focus="focusedKey = r.key"
                />
                <span v-else class="ta-end-label">{{ fmt(r, r.max) }}</span>
              </div>
            </div>
          </template>
        </div>
      </div>
    </div>

    <p v-else-if="!ui.isEditing" class="ta-empty">No adjustments recorded.</p>

    <!-- Apply preset confirm dialog (has unsaved slider changes) -->
    <div v-if="applyConfirmOpen" class="ta-overlay" @click.self="applyConfirmOpen = false">
      <div class="ta-dialog">
        <div class="ta-dialog-head">
          <span>Discard Changes?</span>
          <button class="ta-dialog-close" @click="applyConfirmOpen = false">×</button>
        </div>
        <p class="ta-dialog-body">You have {{ changedRows.length }} modified slider{{ changedRows.length === 1 ? '' : 's' }}. Applying the preset will reset them to the preset defaults.</p>
        <div class="ta-dialog-btns">
          <button class="ta-dlg-discard" @click="executeApplyPreset">Apply Anyway</button>
          <button class="ta-dlg-cancel" @click="applyConfirmOpen = false">Cancel</button>
        </div>
      </div>
    </div>

    <!-- Delete preset confirm dialog -->
    <div v-if="deleteConfirmOpen" class="ta-overlay" @click.self="deleteConfirmOpen = false">
      <div class="ta-dialog">
        <div class="ta-dialog-head">
          <span>Delete Preset</span>
          <button class="ta-dialog-close" @click="deleteConfirmOpen = false">×</button>
        </div>
        <p class="ta-dialog-body">
          Delete "{{ presets.find(p => p.id === selectedPresetId)?.name }}"? This cannot be undone.
          <template v-if="changedRows.length > 0"> Your {{ changedRows.length }} unsaved slider change{{ changedRows.length === 1 ? '' : 's' }} will also be lost.</template>
        </p>
        <div class="ta-dialog-btns">
          <button class="ta-dlg-discard" :disabled="presetBusy" @click="confirmDeletePreset">{{ presetBusy ? '…' : 'Delete' }}</button>
          <button class="ta-dlg-cancel" @click="deleteConfirmOpen = false">Cancel</button>
        </div>
      </div>
    </div>

    <!-- Set Stock Values confirm dialog -->
    <div v-if="stockConfirmOpen" class="ta-overlay" @click.self="stockConfirmOpen = false">
      <div class="ta-dialog">
        <div class="ta-dialog-head">
          <span>Set Stock Values</span>
          <button class="ta-dialog-close" @click="stockConfirmOpen = false">×</button>
        </div>
        <p class="ta-dialog-body">Snapshot current slider positions as stock values. The grey markers will move to match. This cannot be undone.</p>
        <div class="ta-dialog-btns">
          <button class="ta-dlg-keep" @click="confirmSetStock(stockTargetSection)">This Section</button>
          <button class="ta-dlg-keep ta-dlg-keep--all" @click="confirmSetStock(null)">All Sections</button>
          <button class="ta-dlg-cancel" @click="stockConfirmOpen = false">Cancel</button>
        </div>
      </div>
    </div>


  </div>

  <!-- Floating suggestion panel — view mode only, requires tuning data -->
  <Teleport to="body">
    <div v-if="!ui.isEditing && hasSuggestionData" class="ta-suggest-bar" :class="{ collapsed: suggestCollapsed }">
      <div v-if="!suggestCollapsed" class="ta-suggest-message">
        Think you can improve this tune? Make the changes you'd use, and suggest them for testing.
      </div>
      <div class="ta-suggest-actions">
        <button class="ta-suggest-toggle" @click="suggestCollapsed = !suggestCollapsed">
          {{ suggestCollapsed ? '▲' : '▼' }}
        </button>
        <button class="ta-suggest-submit" @click="openSuggestModal">Submit Tune</button>
      </div>
    </div>

    <!-- Suggestion submit modal -->
    <div v-if="suggestModalOpen" class="ta-overlay ta-overlay--fixed" @click.self="suggestModalOpen = false">
      <div class="ta-dialog ta-suggest-dialog">
        <div class="ta-dialog-head">
          <span>Suggest a Tune</span>
          <button class="ta-dialog-close" @click="suggestModalOpen = false">×</button>
        </div>

        <template v-if="suggestDone">
          <p class="ta-dialog-body">Submitted — thanks for the tune.</p>
          <div class="ta-dialog-btns">
            <button class="ta-dlg-cancel" @click="suggestModalOpen = false">Close</button>
          </div>
        </template>

        <template v-else>
          <p class="ta-dialog-body">Give your tune a short title, then let me know where to catch you so I can give credit if I use it.</p>
          <div class="ta-suggest-fields">
            <input
              v-model="suggestTitle"
              class="ta-suggest-input"
              type="text"
              placeholder="Title (60 chars max)"
              maxlength="60"
              :disabled="suggestBusy"
            />
            <input
              v-model="suggestCredit"
              class="ta-suggest-input"
              type="text"
              placeholder="Where to find you — Discord, Reddit, email… (optional)"
              :disabled="suggestBusy"
            />
          </div>
          <p v-if="suggestError" class="ta-suggest-error">{{ suggestError }}</p>
          <div class="ta-dialog-btns">
            <button class="ta-dlg-keep" :disabled="suggestBusy" @click="submitSuggestion">
              {{ suggestBusy ? 'Submitting…' : 'Submit Tune' }}
            </button>
            <button class="ta-dlg-cancel" @click="suggestModalOpen = false">Cancel</button>
          </div>
        </template>
      </div>
    </div>
  </Teleport>

</template>

<style scoped>
.ta {
  --col-label: 72px;
  --col-val: 52px;
  --col-end: 48px;
  --col-gap: 8px;
  --col-left-zone: 231px;
  position: relative;
  background: var(--panel-well);
  border: 1px solid var(--panel-edge);
  border-radius: 10px;
  padding: 12px;
  margin-top: 10px;
}

.ta-preset-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  margin-bottom: 8px;
  background: color-mix(in srgb, var(--panel-well) 60%, transparent);
  border: 1px solid var(--panel-edge);
  border-radius: 6px;
}
.ta-preset-select {
  background: transparent;
  border: 1px solid var(--panel-edge);
  border-radius: 4px;
  color: var(--steel);
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  letter-spacing: 0.05em;
  padding: 3px 6px;
  cursor: pointer;
  outline: none;
  min-width: 140px;
}
.ta-preset-select option { background: var(--panel); color: var(--paper); }
.ta-preset-btn { font-size: 9px; padding: 3px 8px; }
.ta-preset-btn:disabled { opacity: 0.3; cursor: default; pointer-events: none; }
.ta-preset-btn--delete { color: color-mix(in srgb, #cc0000 70%, var(--steel)); border-color: color-mix(in srgb, #cc0000 30%, transparent); }
.ta-preset-btn--delete:hover { color: #ff4444; border-color: #cc0000; }
.ta-preset-save { margin-left: auto; display: flex; align-items: center; gap: 6px; }
.ta-preset-error {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: #ff6b6b;
  margin: 0;
  padding: 0 2px;
  flex-basis: 100%;
}
.ta-preset-name-input {
  background: var(--panel-well);
  border: 1px solid var(--gold);
  border-radius: 4px;
  color: var(--paper);
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  padding: 3px 8px;
  outline: none;
  width: 160px;
}
.ta-toggle-btn, .ta-stock-btn {
  font-size: 11px;
  padding: 5px 10px;
  border-radius: 6px;
  border: 1px solid var(--panel-edge);
  background: color-mix(in srgb, var(--panel) 70%, transparent);
  color: var(--steel);
  cursor: pointer;
  transition: color 0.12s, border-color 0.12s, background 0.12s;
}
.ta-toggle-btn:hover { color: var(--paper); border-color: var(--steel); }
.ta-stock-btn:hover  { color: var(--steel-light); border-color: var(--steel); }



.ta-nonstock {
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  margin-bottom: 10px;
}
.ta-nonstock-summary {
  list-style: none;
  cursor: pointer;
  padding: 9px 14px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 12.5px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--magenta);
  display: flex;
  align-items: center;
  justify-content: space-between;
  transition: background 0.2s ease;
  user-select: none;
}
.ta-nonstock-summary:hover { background: color-mix(in srgb, var(--magenta) 12%, transparent); color: var(--gold); }
.ta-nonstock-summary:hover .ta-nonstock-chev::before { border-right-color: var(--gold); }
.ta-nonstock[open] > .ta-nonstock-summary { background: color-mix(in srgb, var(--magenta) 10%, transparent); }
.ta-nonstock-summary::-webkit-details-marker { display: none; }

.ta-nonstock-chev {
  width: 9px; height: 9px;
  display: flex; align-items: center; justify-content: center;
  transform-origin: 50% 50%;
  transition: transform 0.2s ease;
  transform: rotate(0deg);
}
.ta-nonstock[open] .ta-nonstock-chev { transform: rotate(-90deg); }
.ta-nonstock-chev::before {
  content: '';
  width: 0; height: 0;
  border-top: 4.5px solid transparent;
  border-bottom: 4.5px solid transparent;
  border-right: 9px solid var(--magenta);
}

.ta-nonstock-body  {
  padding: 8px 14px 12px;
  display: flex;
  gap: 16px;
  align-items: start;
  font-size: 12px;
  color: var(--paper);
  line-height: 1.75;
}
.ta-nonstock-col { flex: 1; display: flex; flex-direction: column; gap: 8px; min-width: 0; }
.ta-nonstock-tab-block {
  display: flex;
  flex-direction: column;
  background: color-mix(in srgb, var(--tabc, var(--panel-edge)) 8%, var(--panel-well));
  border: 1px solid color-mix(in srgb, var(--tabc, var(--panel-edge)) 22%, transparent);
  border-radius: 3px;
  overflow: hidden;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
  outline: none;
}
.ta-nonstock-tab-block:hover:not(:has(.ta-nonstock-line:hover)),
.ta-nonstock-tab-block:focus-visible {
  background: color-mix(in srgb, var(--tabc, var(--panel-edge)) 16%, var(--panel-well));
  border-color: color-mix(in srgb, var(--tabc, var(--panel-edge)) 50%, transparent);
}
.ta-nonstock-tab-header {
  text-align: center;
  font-size: 0.85em;
  letter-spacing: 0.07em;
  text-transform: uppercase;
  color: var(--steel-light);
  background: color-mix(in srgb, var(--tabc, var(--panel-edge)) 28%, var(--panel-edge));
  padding: 2px 6px;
}
.ta-nonstock-subgroup { display: flex; flex-direction: column; padding: 3px 0 4px; }
.ta-nonstock-subgroup + .ta-nonstock-subgroup {
  padding-top: 2px;
  border-top: 1px solid color-mix(in srgb, var(--tabc, var(--panel-edge)) 18%, transparent);
}
.ta-nonstock-subgroup-title {
  font-size: 0.75em;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--steel);
  background: color-mix(in srgb, var(--panel-edge) 35%, transparent);
  padding: 1px 6px;
  margin-bottom: 2px;
}
.ta-nonstock-line  { display: flex; gap: 5px; align-items: baseline; min-width: 0; overflow: hidden; padding: 0 6px; cursor: pointer; transition: color 0.1s; }
.ta-nonstock-line:hover { color: var(--gold); }
.ta-nonstock-loc   { color: var(--steel-light); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; flex-shrink: 1; min-width: 0; }
.ta-nonstock-vals  { flex-shrink: 0; white-space: nowrap; font-weight: 600; }
.ta-nonstock-stock { color: var(--steel); font-weight: 400; }
.ta-nonstock-empty { color: var(--steel); }

.ta-tabbar {
  display: flex;
  gap: 6px;
  overflow-x: auto;
  padding-bottom: 10px;
  margin-bottom: 12px;
  border-bottom: 1px solid var(--panel-edge);
  scrollbar-width: thin;
}
.ta-tab-btn {
  flex-shrink: 0;
  position: relative;
  padding: 5px 10px;
  border-radius: 6px;
  border: 1px solid var(--panel-edge);
  background: color-mix(in srgb, var(--panel) 60%, transparent);
  color: var(--steel);
  font-size: 11px;
  cursor: pointer;
  white-space: nowrap;
  transition: color 0.12s, background 0.12s, border-color 0.12s;
}
.ta-tab-btn:hover { color: var(--paper); border-color: var(--tabc, var(--steel)); }
.ta-tab-btn.active { background: var(--tabc, var(--magenta)); border-color: var(--tabc, var(--magenta)); color: #fff; font-weight: 600; }
.ta-tab-dot {
  position: absolute;
  top: 2px; right: 2px;
  width: 5px; height: 5px;
  border-radius: 50%;
  background: var(--magenta);
}
.ta-tab-btn.active .ta-tab-dot { background: #fff; }
.ta-tab-btn--stack { margin-left: auto; color: var(--magenta); border-color: color-mix(in srgb, var(--magenta) 40%, transparent); }
.ta-tab-btn--stack:hover:not(.ta-suppress-hover) { color: #fff; border-color: var(--magenta); background: color-mix(in srgb, var(--magenta) 10%, transparent); }
.ta-tab-btn--stack.ta-suppress-hover:hover { color: var(--magenta); border-color: color-mix(in srgb, var(--magenta) 40%, transparent); background: color-mix(in srgb, var(--panel) 60%, transparent); }
.ta-tab-btn--stack.active.ta-suppress-hover:hover { background: var(--magenta); border-color: var(--magenta); color: #fff; }

.ta-stack-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--paper);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 10px 0;
  border-bottom: 1px solid var(--panel-edge);
  margin: 16px 0 12px;
}
.ta-stack-header::before {
  content: '';
  display: block;
  flex-shrink: 0;
  width: 14px;
  height: 28px;
  background: var(--tabc, var(--panel-edge));
  clip-path: polygon(0 0, 65% 0, 100% 50%, 65% 100%, 0 100%, 35% 50%);
}
.ta-stack-header:first-child { margin-top: 0; }
.ta-stack-header-left {
  width: var(--col-left-zone);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.ta-tab-dot--inline {
  position: relative;
  display: inline-block;
  width: 5px; height: 5px;
  border-radius: 50%;
  background: var(--magenta);
  margin-left: 4px;
  vertical-align: middle;
  top: -1px;
}
/* Light Weight Border button — reusable style for subtle action buttons */
.ta-btn-lwb {
  background: none;
  border: 1px solid color-mix(in srgb, var(--gold) 35%, transparent);
  border-radius: 4px;
  color: var(--gold);
  font-family: 'JetBrains Mono', monospace;
  font-size: 9px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  padding: 3px 8px;
  cursor: pointer;
  transition: color 0.12s, border-color 0.12s;
}
.ta-btn-lwb:hover { color: var(--gold-bright, var(--gold)); border-color: var(--gold); }

/* Tab color tokens — set --tabc on the element, everything inside inherits it */
.ta-tab--tires        { --tabc: var(--tabc-tires); }
.ta-tab--gearing      { --tabc: var(--tabc-gearing); }
.ta-tab--alignment    { --tabc: var(--tabc-alignment); }
.ta-tab--arb          { --tabc: var(--tabc-arb); }
.ta-tab--springs      { --tabc: var(--tabc-springs); }
.ta-tab--damping      { --tabc: var(--tabc-damping); }
.ta-tab--aero         { --tabc: var(--tabc-aero); }
.ta-tab--brakes       { --tabc: var(--tabc-brakes); }
.ta-tab--differential { --tabc: var(--tabc-differential); }

.ta-stack-collapse-btn { margin-left: auto; color: var(--tabc, var(--magenta)); border-color: color-mix(in srgb, var(--tabc, var(--magenta)) 40%, transparent); }
.ta-stack-collapse-btn:hover { color: var(--paper); border-color: var(--tabc, var(--magenta)); }

.ta-group {
  margin-bottom: 16px;
  overflow: hidden;
  border-radius: 0 0 0 4px;
  --track-color: var(--tabc, var(--magenta));
}
.ta-group-header {
  display: flex;
  align-items: baseline;
  margin-bottom: 6px;
}
.ta-section-title-bar + .ta-group-header,
.ta-group > .ta-group-header:first-child {
  margin-bottom: 0;
}
.ta-group-header .ta-caps-nudge { transform: translateY(0.26em); }
.ta-group-title {
  font-size: 11px;
  color: var(--steel);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* Structural two-section layout shared by rows, headers */
.ta-left-section {
  width: min(var(--col-left-zone), 40%);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 12px;
  background: var(--panel-edge);
}
.ta-right-section {
  flex: 1;
  display: flex;
  align-items: center;
  gap: var(--col-gap);
  padding: 4px 8px;
}
/* Header sections sit at baseline */
.ta-group-header .ta-left-section,
.ta-group-header .ta-right-section {
  align-items: baseline;
  padding-top: 0;
  padding-bottom: 0;
}
.ta-group-axis-col {
  flex-shrink: 0;
  text-align: center;
  font-size: 10px;
  color: var(--steel);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  white-space: nowrap;
}
/* min-side: fixed width to align with the min end-label below */
.ta-group-axis-col--min { width: var(--col-end); }
/* max-side: allow overflow outward — doesn't affect track position */
.ta-group-axis-col--max { min-width: var(--col-end); }
.ta-group-axis-track {
  flex: 1;
  min-width: 80px;
  text-align: center;
  font-size: 10px;
  color: var(--steel-light);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.ta-row {
  display: flex;
  align-items: center;
  cursor: pointer;
  transition: background 0.1s, box-shadow 0.1s;
}
.ta-row.focused    { background: color-mix(in srgb, var(--panel-well) 80%, #000); box-shadow: inset 0 0 0 1px var(--magenta); outline: none; }
.ta-row:focus      { outline: none; }
.ta-row--locked    { border-radius: 6px; border: 1px dashed var(--panel-edge); }
.ta-group > .ta-row:not(:last-child):not(.ta-row--locked) { border-bottom: 2px solid var(--panel); }

.ta-row-label {
  width: var(--col-label);
  flex-shrink: 0;
  font-size: 11px;
  color: var(--paper);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ta-lock-line   { font-size: 11px; color: var(--gold); }
.ta-lock-reason { font-size: 10px; color: var(--steel); margin-left: 4px; }

.ta-end-field {
  width: var(--col-end);
  background: color-mix(in srgb, var(--panel) 80%, transparent);
  border: 1px solid var(--panel-edge);
  color: var(--paper);
  font-size: 11px;
  font-family: 'JetBrains Mono', monospace;
  border-radius: 5px;
  padding: 3px 4px;
  text-align: center;
  flex-shrink: 0;
}
.ta-end-field:focus { outline: 1px solid var(--magenta); outline-offset: 1px; }
.ta-end-field::-webkit-inner-spin-button,
.ta-end-field::-webkit-outer-spin-button { -webkit-appearance: none; }
.ta-end-field { -moz-appearance: textfield; }

.ta-end-label {
  width: var(--col-end);
  flex-shrink: 0;
  font-size: 10px;
  font-family: 'JetBrains Mono', monospace;
  color: var(--paper);
  text-align: center;
  opacity: 0.5;
}
.ta-end-field--static {
  opacity: 0.35;
  cursor: default;
}

/* Shared title bar — used by toggle row, gear row, and secondary group rows */
.ta-section-title-bar {
  display: flex;
  overflow: hidden;
}
/* Left zone: spans label/val/end columns, tab color bg, holds section title */
.ta-title-label-zone {
  width: min(var(--col-left-zone), 40%);
  flex-shrink: 0;
  box-sizing: border-box;
  background: var(--tabc, var(--magenta));
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px 8px;
}
/* Right zone: slider track area, panel-edge bg, holds controls or is empty */
.ta-title-slider-zone {
  flex: 1;
  background: var(--panel-edge);
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 4px 8px;
  gap: 8px;
  position: relative;
}
/* Group subtitle centered over the slider zone */
/* Optical nudge for all-caps text vertical centering */
.ta-caps-nudge { transform: translateY(0.15em); }
.ta-slider-zone-title {
  position: absolute;
  left: 0;
  right: 0;
  text-align: center;
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--steel-light);
  pointer-events: none;
}
.ta-section-title-text {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--paper);
  pointer-events: none;
}

/* Static / Variable mode toggle */
.ta-mode-row {
  /* layout handled by child zones */
}
.ta-mode-label-text {
  font-family: 'JetBrains Mono', monospace;
  font-size: 9px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--steel-light);
}
.ta-mode-switch {
  position: relative;
  display: inline-flex;
  cursor: pointer;
}
.ta-mode-switch input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
}
.ta-mode-track {
  display: inline-block;
  width: 28px;
  height: 14px;
  border-radius: 7px;
  background: var(--steel-light);
  position: relative;
  transition: background 0.2s;
}
.ta-mode-switch input:checked + .ta-mode-track {
  background: var(--tabc, var(--magenta));
}
.ta-mode-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: #fff;
  transition: left 0.2s;
}
.ta-mode-switch input:checked + .ta-mode-track .ta-mode-thumb {
  left: 16px;
}
.ta-mode-state {
  font-family: 'JetBrains Mono', monospace;
  font-size: 9px;
  letter-spacing: 0.08em;
  color: var(--tabc, var(--magenta));
}
.ta-mode-state.static {
  color: var(--steel-light);
}

.ta-val-box {
  flex: 0 1 var(--col-val);
  min-width: 28px;
  max-width: var(--col-val);
  height: 22px;
  background: color-mix(in srgb, var(--panel-well) 90%, #000);
  border: 1px solid var(--panel-edge);
  color: var(--paper);
  font-size: 11px;
  font-family: 'JetBrains Mono', monospace;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.ta-row.changed .ta-val-box {
  background: #7a5800;
  border-color: #a07800;
  color: #fff;
}
.ta-val-input {
  background: none; border: none; color: inherit;
  font-family: inherit; font-size: inherit;
  width: 100%; text-align: center; padding: 0; outline: none;
}

.ta-track-wrap {
  position: relative;
  flex: 1;
  min-width: 80px;
  display: flex;
  align-items: center;
}

.ta-stock-tick {
  position: absolute;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 14px; height: 14px;
  box-sizing: border-box;
  border-radius: 50%;
  background: var(--panel-edge);
  z-index: 2;
  pointer-events: none;
}
.ta-center-mark {
  position: absolute;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 2px; height: 14px;
  background: var(--steel-light);
  z-index: 4;
  pointer-events: none;
  border-radius: 1px;
}

.ta-slider {
  -webkit-appearance: none;
  appearance: none;
  width: 100%; height: 20px;
  flex: 1;
  background: transparent;
  outline: none;
  position: relative;
  z-index: 3;
  cursor: pointer;
  padding: 0; margin: 0;
}
.ta-slider.readonly { pointer-events: none; cursor: default; }
.ta-slider::-webkit-slider-runnable-track {
  -webkit-appearance: none;
  height: 6px; border-radius: 3px;
  background: linear-gradient(to right,
    var(--panel-edge) 0%,
    var(--panel-edge) min(var(--center, 0%), var(--fill, 0%)),
    var(--track-color, var(--magenta)) min(var(--center, 0%), var(--fill, 0%)),
    var(--track-color, var(--magenta)) max(var(--center, 0%), var(--fill, 0%)),
    var(--panel-edge) max(var(--center, 0%), var(--fill, 0%)),
    var(--panel-edge) 100%
  );
}
.ta-slider::-moz-range-track    { height: 6px; border-radius: 3px; background: var(--panel-edge); }
.ta-slider::-moz-range-progress { height: 6px; border-radius: 3px; background: var(--track-color, var(--magenta)); }
.ta-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 14px; height: 14px; border-radius: 50%;
  margin-top: -4px;
  background: var(--track-color, var(--magenta)); cursor: pointer;
  position: relative; z-index: 2;
  box-shadow: 0 0 0 2px var(--panel-well);
}
.ta-slider.readonly::-webkit-slider-thumb { cursor: default; }
.ta-slider--stock::-webkit-slider-thumb { background: var(--steel-light); }
.ta-slider::-moz-range-thumb {
  width: 14px; height: 14px; border-radius: 50%;
  background: var(--track-color, var(--magenta)); border: none; cursor: pointer;
  box-shadow: 0 0 0 2px var(--panel-well);
}
.ta-slider--stock::-moz-range-thumb { background: var(--steel-light); }
.ta-slider--stock::-moz-range-progress { background: var(--steel-light); }

.ta-gear-select-row {
  margin-bottom: 14px;
}
.ta-gear-select {
  background: transparent;
  border: none;
  color: var(--paper);
  font-size: 10px;
  font-family: 'JetBrains Mono', monospace;
  font-weight: 600;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  padding: 0;
  cursor: pointer;
  outline: none;
}
.ta-gear-select option { background: var(--panel); color: var(--paper); }

.ta-empty {
  font-size: 12px; color: var(--steel); opacity: 0.5;
  margin: 4px 0 0; text-align: center; padding: 8px 0;
}

.ta-overlay {
  position: absolute; inset: 0;
  border-radius: 10px;
  background: rgba(0,0,0,0.65);
  display: flex; align-items: center; justify-content: center;
  z-index: 130;
}
.ta-overlay--fixed { position: fixed; border-radius: 0; }
.ta-dialog {
  background: var(--panel);
  border: 1px solid var(--panel-edge);
  border-radius: 8px;
  padding: 20px;
  max-width: 320px; width: 90%;
}
.ta-dialog-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-family: 'JetBrains Mono', monospace;
  font-size: 13px;
  color: var(--paper);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 14px;
}
.ta-dialog-close {
  background: transparent; border: none;
  color: var(--steel); font-size: 24px; line-height: 1; cursor: pointer;
}
.ta-dialog-body {
  color: var(--steel);
  font-size: 13px;
  line-height: 1.5;
  margin: 0 0 16px;
}
.ta-dialog-btns {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.ta-dialog-btns button {
  width: 100%;
  color: #fff;
  font-family: 'JetBrains Mono', monospace;
  font-size: 13px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  padding: 12px 0;
  border-radius: 5px;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
}
.ta-dlg-keep    { background: #1e3d00; border: 2px solid #2e5f00; }
.ta-dlg-keep:hover    { background: #74b050; border-color: #a8d888; box-shadow: 0 0 16px rgba(116,176,80,0.85); }
.ta-dlg-keep--all { background: #7a5800; border: 2px solid #a07800; }
.ta-dlg-keep--all:hover { background: #ffc200; border-color: #ffe870; box-shadow: 0 0 16px rgba(255,194,0,0.85); }
.ta-dlg-discard { background: #5c0000; border: 2px solid #7a0000; }
.ta-dlg-discard:hover { background: #cc0000; border-color: #ff4444; box-shadow: 0 0 16px rgba(200,0,0,0.85); }
.ta-dlg-cancel  { background: #5c0000; border: 2px solid #7a0000; }
.ta-dlg-cancel:hover  { background: #cc0000; border-color: #ff4444; box-shadow: 0 0 16px rgba(200,0,0,0.85); }

/* ── Suggestion floating panel ──────────────────────────────────────────────── */
.ta-suggest-bar {
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 120;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  background: color-mix(in srgb, var(--panel) 92%, transparent);
  border: 1px solid var(--panel-edge);
  border-radius: 10px;
  padding: 14px 20px;
  max-width: 480px;
  width: calc(100vw - 40px);
  backdrop-filter: blur(8px);
  box-shadow: 0 8px 32px rgba(0,0,0,0.45);
  transition: padding 0.2s;
}
.ta-suggest-bar.collapsed {
  padding: 10px 20px;
}
.ta-suggest-message {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--steel);
  text-align: center;
  line-height: 1.5;
  letter-spacing: 0.03em;
}
.ta-suggest-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}
.ta-suggest-toggle {
  background: none;
  border: none;
  color: var(--steel);
  font-size: 10px;
  cursor: pointer;
  padding: 2px 4px;
  opacity: 0.5;
  transition: opacity 0.12s;
}
.ta-suggest-toggle:hover { opacity: 1; }
.ta-suggest-submit {
  background: none;
  border: 1px solid color-mix(in srgb, var(--gold) 50%, transparent);
  border-radius: 6px;
  color: var(--gold);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  padding: 6px 16px;
  cursor: pointer;
  transition: background 0.12s, border-color 0.12s, box-shadow 0.12s;
}
.ta-suggest-submit:hover {
  background: color-mix(in srgb, var(--gold) 12%, transparent);
  border-color: var(--gold);
  box-shadow: 0 0 12px color-mix(in srgb, var(--gold) 30%, transparent);
}

/* Suggestion modal specifics */
.ta-suggest-dialog { max-width: 440px; }
.ta-suggest-fields {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 0 2px 4px;
}
.ta-suggest-input {
  background: var(--panel-well);
  border: 1px solid var(--panel-edge);
  border-radius: 5px;
  color: var(--paper);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  padding: 7px 10px;
  outline: none;
  transition: border-color 0.12s;
}
.ta-suggest-input:focus { border-color: var(--gold); }
.ta-suggest-input::placeholder { color: var(--steel); opacity: 0.5; }
.ta-suggest-input:disabled { opacity: 0.5; }
.ta-suggest-error {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: #ff6b6b;
  margin: 0;
  padding: 0 2px;
}
</style>
