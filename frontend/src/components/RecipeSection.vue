<script setup lang="ts">
import { computed, inject, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import type { CardCar, CardTune, ForzaRecipeSection, Tune, UpgradeCategory } from '../types'
import { api } from '../api'
import { useUiStore } from '../stores/ui'
import { useFilterStore } from '../stores/filters'
import { useCardsStore } from '../stores/cards'
import { useModalStore } from '../stores/modal'
import { useCarsStore } from '../stores/cars'
import { useTunesStore } from '../stores/tunes'
import { MarkDirtyKey } from '../keys'
import EditableText from './EditableText.vue'
import UpgradesPicker from './UpgradesPicker.vue'
import TuningAdjustments from './TuningAdjustments.vue'
import CarPicker from './CarPicker.vue'
import rawUpgrades from '../data/fh_upgrades.json'
import { impliedUpgrades, applyImpliedUpgrades, applySpringsChoice, type ImpliedUpgradesResult } from '../constants/tuning'
import { formatShareCode } from '../utils/shareCode'

const props = defineProps<{
  recipe: ForzaRecipeSection
  cardId?: string
  carId?: string | null
  images?: import('../types').CardImage[]
  initialKitOpen?: boolean
  resetToken?: number
  forceEdit?: boolean
}>()
const emit = defineEmits<{
  'update:recipe': [recipe: ForzaRecipeSection]
  'update:carId': [id: string | null]
  'update:activeCarId': [carId: string | null]
}>()
const ui = useUiStore()
const isEditing = computed(() => ui.isEditing || !!props.forceEdit)
const modal = useModalStore()
const filters = useFilterStore()
const carsStore = useCarsStore()
const tunesStore = useTunesStore()
const markDirty = inject(MarkDirtyKey, () => {})

// Active car owns the car identity when cars[] exists; single slot uses the card-level carId.
const effectiveCarId = computed(() =>
  hasCars.value ? (local.cars?.[activeCarIndex.value]?.carId ?? null) : (props.carId ?? null)
)
const linkedCar = computed(() => effectiveCarId.value ? carsStore.byId(effectiveCarId.value) : undefined)

const variantFigureImage = computed(() => {
  if (!hasCars.value || !effectiveCarId.value || !props.images?.length) return null
  return props.images
    .filter(img => img.carId === effectiveCarId.value)
    .sort((a, b) => a.order - b.order)[0] ?? null
})
const variantFigurePath = computed(() => {
  const img = variantFigureImage.value
  return img ? (img.thumbPath ?? img.path) : null
})

const tuneDisplayName = computed(() => {
  if (local.tuneName) return local.tuneName
  if (!hasCars.value || !linkedCar.value) return ''
  const car = linkedCar.value
  const yr = car.year ? `'${String(car.year).slice(-2)} ` : ''
  return `${yr}${car.model}`
})

function onVariantCarIdUpdate(id: string | null) {
  if (hasCars.value && local.cars?.[activeCarIndex.value]) {
    local.cars[activeCarIndex.value].carId = id ?? ''
    emit('update:activeCarId', id)
    markDirty()
    flush()
  } else {
    markDirty()
    emit('update:carId', id)
  }
}
const taRef = ref<InstanceType<typeof TuningAdjustments> | null>(null)

const CORE_SPEC_KEYS = ['Drivetrain', 'Engine', 'Transmission', 'Tires', 'Suspension']

function cloneRecipe(r: ForzaRecipeSection): ForzaRecipeSection {
  return JSON.parse(JSON.stringify(r))
}

const local = reactive<ForzaRecipeSection>(cloneRecipe(props.recipe))

// Normalize any null/undefined values to '' so the select binding always gets a string.
for (const k of CORE_SPEC_KEYS) {
  if (local.coreSpecs[k] == null) local.coreSpecs[k] = ''
}

// ── Multi-car / multi-tune support ───────────────────────────────────────────
// hasCars:   cars[] has 1+ entries
// isMultiCar: cars[] has 2+ entries (shows CarTabs strip)
// activeCar: the car currently selected by CarTabs
// hasTunes:  active car has 2+ tunes (shows TuneTabs strip)
const hasCars    = computed(() => (local.cars?.length ?? 0) >= 1)
const isMultiCar = computed(() => (local.cars?.length ?? 0) >= 2)
const activeCar  = computed(() => hasCars.value ? local.cars![activeCarIndex.value] : null)
const hasTunes   = computed(() => (activeCar.value?.tunes.length ?? 0) >= 2)

const activeCarIndex  = ref(0)
const activeTuneIndex = ref(0)

function applyVariant(carIdx: number, tuneIdx = activeTuneIndex.value) {
  const tune = local.cars?.[carIdx]?.tunes[tuneIdx]
  if (!tune) return
  local.tuneName = tune.tuneName
  local.shareCode = tune.shareCode
  for (const k of Object.keys(tune.coreSpecs)) local.coreSpecs[k] = tune.coreSpecs[k]
  local.upgrades = tune.upgrades
  local.adjustments = tune.adjustments
}

function carLabel(car: CardCar): string {
  const c = carsStore.byId(car.carId)
  if (!c) return car.carId || '(no car)'
  return `${c.year ? c.year + ' ' : ''}${c.make} ${c.model}`
}

function carShortLabel(car: CardCar): string {
  const c = carsStore.byId(car.carId)
  if (!c) return car.carId || '(no car)'
  return c.model
}

// ── Edit-mode variant management ──────────────────────────────────────────────
const showAddVariantPicker = ref(false)

// Auto-propose tabs when card images span 2+ distinct cars and no cars[] exist yet
const cardsStore = useCardsStore()
const autoProposeCarIds = computed<string[]>(() => {
  if (hasCars.value || !props.cardId) return []
  const card = cardsStore.cards.find(c => c.id === props.cardId)
  if (!card) return []
  const counts = new Map<string, number>()
  for (const img of card.images) {
    if (img.carId) counts.set(img.carId, (counts.get(img.carId) ?? 0) + 1)
  }
  if (counts.size < 2) return []
  return [...counts.entries()].sort((a, b) => b[1] - a[1]).map(([id]) => id)
})
const autoProposesDismissed = ref(false)
const showAutoPropose = computed(() => isEditing.value && autoProposeCarIds.value.length >= 2 && !autoProposesDismissed.value)

function acceptAutoPropose() {
  const ids = autoProposeCarIds.value
  const currentCarId = props.carId ?? ''
  const matchIdx = ids.indexOf(currentCarId)
  const anchorIdx = matchIdx >= 0 ? matchIdx : 0
  local.cars = ids.map((id, i) => {
    if (i === anchorIdx) {
      return {
        carId: id,
        tunes: [{
          tuneName: local.tuneName,
          shareCode: local.shareCode,
          coreSpecs: { ...local.coreSpecs },
          upgrades: [...local.upgrades],
          adjustments: [...local.adjustments],
        }],
      }
    }
    return makeEmptyCar(id)
  })
  activeCarIndex.value = anchorIdx
  autoProposesDismissed.value = true
  markDirty()
  flush()
}

// ── Car Tabs setup wizard ─────────────────────────────────────────────────────
type TuningPreset = { id: number; name: string; values: Record<string, number>; kind: string }
const showSetupWizard   = ref(false)
const wizardStep        = ref(0)
const wizardNonAnchorIds = ref<string[]>([])
const wizardAnchorIdx   = ref(0)
const wizardAllIds      = ref<string[]>([])
const wizardPresets     = ref<TuningPreset[]>([])
const wizardLoading     = ref(false)
const wizardSelections  = ref<Record<string, number | null>>({})

async function beginSetupWizard() {
  const ids = autoProposeCarIds.value
  if (ids.length < 2) return
  const currentCarId = props.carId ?? ''
  const matchIdx = ids.indexOf(currentCarId)
  const anchorIdx = matchIdx >= 0 ? matchIdx : 0
  wizardAllIds.value = ids
  wizardAnchorIdx.value = anchorIdx
  wizardNonAnchorIds.value = ids.filter((_, i) => i !== anchorIdx)
  wizardStep.value = 0
  wizardSelections.value = Object.fromEntries(wizardNonAnchorIds.value.map(id => [id, null]))
  showSetupWizard.value = true  // show immediately; presets load behind the loading state
  wizardLoading.value = true
  try { wizardPresets.value = await api.listTuningPresets() } catch { /* proceed without presets */ } finally { wizardLoading.value = false }
}

function finishWizard() {
  const ids = wizardAllIds.value
  const anchorIdx = wizardAnchorIdx.value
  local.cars = ids.map((id, i) => {
    if (i === anchorIdx) {
      return {
        carId: id,
        tunes: [{
          tuneName: local.tuneName,
          shareCode: local.shareCode,
          coreSpecs: { ...local.coreSpecs },
          upgrades: [...local.upgrades],
          adjustments: [...local.adjustments],
        }],
      } as CardCar
    }
    const car = makeEmptyCar(id)
    const presetId = wizardSelections.value[id]
    if (presetId != null) car.tunes[0].pendingPresetId = presetId
    return car
  })
  activeCarIndex.value = anchorIdx
  autoProposesDismissed.value = true
  showSetupWizard.value = false
  markDirty()
  flush()
}

function wizardCarLabel(carId: string): string {
  const car = carsStore.byId(carId)
  return car ? `${car.year} ${car.make} ${car.model}` : carId
}
const pendingRemoveIdx = ref<number | null>(null)

function carIsEmpty(car: CardCar): boolean {
  const t = car.tunes[0]
  if (!t) return true
  return !t.tuneName.trim() && !t.shareCode.trim()
    && t.upgrades.every(c => c.parts.length === 0)
    && t.adjustments.length === 0
}

function makeEmptyCar(carId: string): CardCar {
  return {
    carId,
    tunes: [{
      tuneName: '',
      shareCode: '',
      coreSpecs: Object.fromEntries(CORE_SPEC_KEYS.map(k => [k, ''])),
      upgrades: [],
      adjustments: [],
    }],
  }
}

function addCar(carId: string | null) {
  if (!carId) { showAddVariantPicker.value = false; return }
  showAddVariantPicker.value = false
  if (!hasCars.value) {
    // Promote: current recipe fields → cars[0], new car → cars[1]
    local.cars = [
      {
        carId: props.carId ?? '',
        tunes: [{
          tuneName: local.tuneName,
          shareCode: local.shareCode,
          coreSpecs: { ...local.coreSpecs },
          upgrades: [...local.upgrades],
          adjustments: [...local.adjustments],
        }],
      },
      makeEmptyCar(carId),
    ]
    activeCarIndex.value = 1
    applyVariant(1)
  } else {
    local.cars = [...(local.cars ?? []), makeEmptyCar(carId)]
    const newIdx = local.cars.length - 1
    activeCarIndex.value = newIdx
    applyVariant(newIdx)
  }
  markDirty()
  flush()
}

function makeEmptyTune(): CardTune {
  return {
    tuneName: '',
    shareCode: '',
    coreSpecs: Object.fromEntries(CORE_SPEC_KEYS.map(k => [k, ''])),
    upgrades: [],
    adjustments: [],
  }
}

function tuneIsEmpty(tune: CardTune): boolean {
  return !tune.tuneName.trim() && !tune.shareCode.trim()
    && tune.upgrades.every(c => c.parts.length === 0)
    && tune.adjustments.length === 0
}

function addTuneVariant() {
  if (!hasCars.value) {
    // Promote single-slot recipe to cars[0] with 2 tunes.
    local.cars = [{
      carId: props.carId ?? '',
      tunes: [
        { tuneName: local.tuneName, shareCode: local.shareCode,
          coreSpecs: { ...local.coreSpecs }, upgrades: [...local.upgrades], adjustments: [...local.adjustments] },
        makeEmptyTune(),
      ],
    }]
    activeCarIndex.value = 0
    activeTuneIndex.value = 1
    applyVariant(0, 1)
  } else {
    const car = local.cars![activeCarIndex.value]
    car.tunes.push(makeEmptyTune())
    activeTuneIndex.value = car.tunes.length - 1
    applyVariant(activeCarIndex.value, activeTuneIndex.value)
  }
  markDirty()
  flush()
}

const pendingRemoveTuneIdx = ref<number | null>(null)

function removeTune(idx: number) {
  pendingRemoveTuneIdx.value = null
  const car = local.cars?.[activeCarIndex.value]
  if (!car || car.tunes.length <= 1) return
  car.tunes.splice(idx, 1)
  if (activeTuneIndex.value >= car.tunes.length) activeTuneIndex.value = car.tunes.length - 1
  // Demote back to single-slot if the last car now has only 1 tune and there's only 1 car.
  if (!isMultiCar.value && car.tunes.length === 1) {
    applyVariant(0, 0)
    local.cars = undefined
    activeCarIndex.value = 0
    activeTuneIndex.value = 0
    emit('update:activeCarId', null)
  } else {
    applyVariant(activeCarIndex.value, activeTuneIndex.value)
  }
  markDirty()
  flush()
}

function removeCar(idx: number) {
  pendingRemoveIdx.value = null
  if (!local.cars) return
  if (local.cars.length <= 1) {
    // Last car tab — demote to single slot. Root fields already in sync via applyVariant.
    local.cars = undefined
    activeCarIndex.value = 0
    emit('update:activeCarId', null)
  } else {
    local.cars.splice(idx, 1)
    if (activeCarIndex.value >= local.cars.length) {
      activeCarIndex.value = local.cars.length - 1
    }
    applyVariant(activeCarIndex.value)
    emit('update:activeCarId', local.cars[activeCarIndex.value]?.carId ?? null)
  }
  markDirty()
  flush()
}

// ── Tune import offer (step 10) ───────────────────────────────────────────────
// Set when addVariantWithLookup finds existing tunes for the incoming car.
// Cleared when user accepts or dismisses.
const pendingTuneImport = ref<{ carId: string; tunes: Tune[] } | null>(null)
const pendingVariantCarId = ref<string | null>(null)

function acceptTuneImport(tune: Tune) {
  const carId = pendingVariantCarId.value
  pendingTuneImport.value = null
  pendingVariantCarId.value = null
  if (!carId) return
  addCar(carId)
  const newIdx = (local.cars?.length ?? 1) - 1
  const t = local.cars?.[newIdx]?.tunes[0]
  if (!t) return
  t.tuneName = tune.officialName ?? ''
  t.shareCode = tune.shareCode ?? ''
  try { if (tune.coreSpecs) Object.assign(t.coreSpecs, JSON.parse(tune.coreSpecs)) } catch {}
  try { if (tune.upgrades) t.upgrades = JSON.parse(tune.upgrades) } catch {}
  try { if (tune.adjustments) t.adjustments = JSON.parse(tune.adjustments) } catch {}
  applyVariant(newIdx)
  markDirty()
  flush()
}

function dismissTuneImport() {
  const carId = pendingVariantCarId.value
  pendingTuneImport.value = null
  pendingVariantCarId.value = null
  if (carId) addCar(carId)
}

// Called by CardView when the multi-car interrupt fires via ui store.
async function addVariantWithLookup(carId: string) {
  const tunes = await tunesStore.loadForCar(carId)
  if (tunes.length) {
    pendingVariantCarId.value = carId
    pendingTuneImport.value = { carId, tunes }
  } else {
    addCar(carId)
  }
}

defineExpose({ addVariantWithLookup, acceptAutoPropose, beginSetupWizard })

// Suppress the upgrades sync-flush during a variant switch — the TA widget
// hasn't re-read the new adjustments yet, so getAdjustments() would return
// stale data from the previous variant.
let suppressFlush = false

watch(activeCarIndex, async (idx, prevIdx) => {
  // Snapshot the outgoing car's active tune before switching
  if (prevIdx !== undefined && local.cars?.[prevIdx]?.tunes[activeTuneIndex.value] && taRef.value) {
    local.cars[prevIdx].tunes[activeTuneIndex.value].adjustments = taRef.value.getAdjustments()
  }
  activeTuneIndex.value = 0
  suppressFlush = true
  applyVariant(idx, 0)
  suppressFlush = false
  emit('update:activeCarId', local.cars?.[idx]?.carId ?? null)

  // Auto-apply a pending preset the first time this car tab is opened
  const tune = local.cars?.[idx]?.tunes[0]
  if (tune?.pendingPresetId != null) {
    if (!wizardPresets.value.length) {
      try { wizardPresets.value = await api.listTuningPresets() } catch { return }
    }
    if (activeCarIndex.value !== idx) return  // user switched tabs during load
    const preset = wizardPresets.value.find(p => p.id === tune.pendingPresetId)
    if (preset && taRef.value) {
      await nextTick()
      if (activeCarIndex.value !== idx) return  // user switched tabs during tick
      taRef.value.applyPresetValues(preset.values, preset.kind)
      delete tune.pendingPresetId
      flush()
      markDirty()
    }
  }
})

onMounted(() => {
  if (hasCars.value) {
    const idx = activeCarIndex.value
    applyVariant(idx)
    emit('update:activeCarId', local.cars?.[idx]?.carId ?? null)
  }
})

watch(activeTuneIndex, (idx, prevIdx) => {
  const car = local.cars?.[activeCarIndex.value]
  if (prevIdx !== undefined && car?.tunes[prevIdx] && taRef.value) {
    car.tunes[prevIdx].adjustments = taRef.value.getAdjustments()
  }
  suppressFlush = true
  applyVariant(activeCarIndex.value, idx)
  suppressFlush = false
})

// resetToken: parent increments this to signal a genuine external reset (history
// restore, cancel/discard). Watching the token instead of props.recipe directly
// means our own flush → store update → prop change cycle never triggers a re-sync.
watch(() => props.resetToken, () => {
  activeCarIndex.value = 0
  activeTuneIndex.value = 0
  Object.assign(local, cloneRecipe(props.recipe))
  if (hasCars.value) applyVariant(0, 0)
})

function flush() {
  // Keep active car/tune in sync with local fields before cloning
  if (hasCars.value && local.cars?.[activeCarIndex.value]) {
    const tune = local.cars[activeCarIndex.value].tunes[activeTuneIndex.value]
    if (tune) {
      tune.tuneName = local.tuneName
      tune.shareCode = local.shareCode
      Object.assign(tune.coreSpecs, local.coreSpecs)
      tune.upgrades = local.upgrades
      tune.adjustments = local.adjustments
    }
  }
  const clone = JSON.parse(JSON.stringify(local)) as ForzaRecipeSection
  // JSON.stringify drops undefined — restore explicitly so CardView's Object.assign
  // clears section.cars when demoted back to single-slot.
  if (!local.cars) clone.cars = undefined
  if (taRef.value) {
    const liveAdj = taRef.value.getAdjustments()
    clone.adjustments = liveAdj
    if (hasCars.value && clone.cars?.[activeCarIndex.value]) {
      clone.cars[activeCarIndex.value].tunes[activeTuneIndex.value].adjustments = liveAdj
      local.cars![activeCarIndex.value].tunes[activeTuneIndex.value].adjustments = liveAdj
    }
  }
  emit('update:recipe', clone)
}

function onImpliedUpgrades(result: ImpliedUpgradesResult) {
  if (result.toAdd.length) {
    applyImpliedUpgrades(local.upgrades, result.toAdd)
    if (isEditing.value) flush()
  }
}

function onRemoveUpgrade(part: string) {
  for (const cat of local.upgrades) {
    const idx = cat.parts.indexOf(part)
    if (idx !== -1) { cat.parts.splice(idx, 1); if (isEditing.value) flush(); break }
  }
}

function onSpringsChoice(tier: 'Race' | 'Rally' | 'Drift') {
  applySpringsChoice(local.upgrades, tier)
}

// Set of upgrade part base names (e.g. 'Brakes', 'Springs and Dampers') implied
// by the current slider state. Recomputed reactively; used by UpgradesPicker to
// show the auto-populate indicator.
const SPRINGS_TABS = new Set(['alignment', 'springs', 'damping'])
const impliedPartNames = computed<Set<string>>(() => {
  if (!isEditing.value) return new Set()
  const result = impliedUpgrades(local.adjustments, [])
  const names = new Set<string>(result.toAdd.map(x => x.part))
  if (local.adjustments.some(r => SPRINGS_TABS.has(r.tab) && r.value !== r.stock)) {
    names.add('Springs and Dampers')
  }
  return names
})

// UpgradesPicker mutates local.upgrades in-place; detect those mutations and flush.
watch(() => local.upgrades, () => {
  if (suppressFlush) return
  flush()
  markDirty()
}, { deep: true, flush: 'sync' })

const hasNonStockSpecs = computed(() =>
  CORE_SPEC_KEYS.some(k => !!local.coreSpecs[k]?.trim()),
)
const partCount = computed(() =>
  local.upgrades.reduce((n, c) => n + c.parts.length, 0),
)

// Full upgrade part list for "Show Stock" view mode and cost tallying
type UpgJPart = { part: string; tiers: string[] | 'stepped' | 'cosmetic'; specialTiers?: string[]; tierCosts?: Record<string, number> }
type UpgJCat  = { name: string; parts: UpgJPart[] }

const SPECIAL_STATES = new Set(['No Upgrade', 'Not Available'])
const allUpgCats = rawUpgrades.categories as UpgJCat[]

const CAT_ORDER = [
  'Body Kits and Conversions',
  'Engine',
  'Drivetrain',
  'Platform and Handling',
  'Aero and Appearance',
  'Tires and Wheels',
]
const allStockCats = CAT_ORDER.map(n => allUpgCats.find(c => c.name === n)).filter(Boolean) as UpgJCat[]
const COL3_BREAK = new Set(['Drivetrain', 'Tires and Wheels'])
const COL2_BREAK = new Set(['Platform and Handling'])
const STEPPED_SET = new Set([
  'Front Tire Width', 'Rear Tire Width',
  'Front Rim Size', 'Rear Rim Size',
  'Front Track Width', 'Rear Track Width',
])
// Flat set of all installed part strings, rebuilt only when upgrades change.
// Turns viewInstalledTier from O(categories × tiers) into O(tiers).
const installedParts = computed(() => {
  const s = new Set<string>()
  for (const cat of local.upgrades) for (const p of cat.parts) s.add(p)
  return s
})
function viewInstalledTier(tiers: string[], specialTiers?: string[]): string | null {
  const all = specialTiers ? [...tiers, ...specialTiers] : tiers
  return all.find(t => installedParts.value.has(t)) ?? null
}
function viewPartLabel(part: string, tiers: string[], specialTiers?: string[]): string {
  const tier = viewInstalledTier(tiers, specialTiers)
  if (!tier || tier === 'Stock' || tier === 'No Upgrade') return 'Stock ' + part
  return tier
}
function isCustomTier(tiers: string[], specialTiers?: string[]): boolean {
  const tier = viewInstalledTier(tiers, specialTiers)
  return !!tier && tier !== 'Stock' && !SPECIAL_STATES.has(tier)
}
function viewSteppedValue(partName: string): number {
  for (const cat of local.upgrades) {
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
  const tier = viewInstalledTier(p.tiers, p.specialTiers)
  if (!tier || tier === 'Stock' || SPECIAL_STATES.has(tier)) return null
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
      const hit = viewInstalledTier(p.tiers, p.specialTiers)
      if (hit && !SPECIAL_STATES.has(hit)) total += p.tierCosts[hit] ?? 0
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
  local.coreSpecs[key] = (e.target as HTMLSelectElement).value
  flush()
  markDirty()
}

const codeCopied = ref(false)
let codeCopiedTimer: ReturnType<typeof setTimeout> | null = null
function copyShareCode() {
  if (!local.shareCode) return
  navigator.clipboard.writeText(local.shareCode).then(() => {
    codeCopied.value = true
    if (codeCopiedTimer) clearTimeout(codeCopiedTimer)
    codeCopiedTimer = setTimeout(() => { codeCopied.value = false }, 1500)
  })
}

function onShareCodeInput(e: Event) {
  const input = e.target as HTMLInputElement
  const formatted = formatShareCode(input.value)
  local.shareCode = formatted
  // Rewrite value to insert spaces; cursor goes to end which is acceptable for a code field.
  input.value = formatted
  flush()
  markDirty()
}

// The Upgrades sub-list follows its own filter checkbox + expand/collapse-all.
const kitOpen = ref(props.initialKitOpen ?? false)
watch(() => filters.upgradesExpanded, (v) => (kitOpen.value = v))
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
const showStock      = ref(local.showStock ?? false)
const presetBarEl    = ref<HTMLElement | null>(null)

function loadPresets() {
  try { presets.value = JSON.parse(localStorage.getItem(STORE_KEY) ?? '[]') }
  catch { presets.value = [] }
}
loadPresets()

function persistPresets() { localStorage.setItem(STORE_KEY, JSON.stringify(presets.value)) }

function applyPreset(p: Preset) {
  local.upgrades.splice(0, local.upgrades.length, ...JSON.parse(JSON.stringify(p.upgrades)))
  activeName.value = p.name
  // flush() + markDirty() handled by the local.upgrades sync watcher
  showPresetMenu.value = false
}

function saveAsPreset() {
  const name = saveNameInput.value.trim()
  if (!name) return
  loadPresets()
  presets.value.push({ name, upgrades: JSON.parse(JSON.stringify(local.upgrades)) })
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
  local.upgrades.splice(0)
  activeName.value = ''
  // flush() + markDirty() handled by the local.upgrades sync watcher
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

    <!-- Car tab strip — renders for 2+ cars in view mode, or always in edit mode -->
    <div v-if="isEditing || (local.cars?.length ?? 0) >= 2" class="rs-variant-tabs">
      <template v-if="hasCars">
        <div
          v-for="(car, i) in local.cars"
          :key="(car.carId || '') + i"
          class="rs-variant-tab-wrap"
          :class="{ 'rs-variant-tab-wrap--active': activeCarIndex === i }"
        >
          <button
            class="rs-variant-tab"
            :class="{
              'rs-variant-tab--active': activeCarIndex === i,
              'rs-variant-tab--suggested': car.tunes[0]?.isSuggested,
            }"
            type="button"
            :title="carLabel(car)"
            @click="activeCarIndex = i"
          >
            {{ carShortLabel(car) }}
            <span v-if="car.tunes[0]?.isSuggested" class="rs-tab-suggested-badge">Suggested</span>
          </button>
          <button
            v-if="isEditing && !car.tunes[0]?.isSuggested"
            class="rs-variant-remove"
            type="button"
            :title="`Remove ${carLabel(car)}`"
            @click.stop="carIsEmpty(car) ? removeCar(i) : (pendingRemoveIdx = i)"
          >×</button>
        </div>
      </template>

      <!-- Auto-propose tabs when images span multiple cars -->
      <div v-if="showAutoPropose" class="rs-autopropose">
        <span class="rs-autopropose-msg">{{ autoProposeCarIds.length }} cars detected in photos</span>
        <button class="rs-autopropose-accept" type="button" @click="beginSetupWizard">Set up tabs</button>
        <button class="rs-autopropose-dismiss" type="button" @click="autoProposesDismissed = true">×</button>
      </div>

      <!-- Add car button (edit mode only) — always goes to CarPicker -->
      <div v-if="isEditing" class="rs-add-variant-wrap">
        <template v-if="!showAddVariantPicker">
          <button
            class="rs-variant-tab rs-variant-tab--add"
            type="button"
            @click="showAddVariantPicker = true"
          >+</button>
        </template>
        <div v-else class="rs-add-picker-inline">
          <CarPicker :car-id="null" @update:car-id="addCar" />
          <button class="rs-add-picker-cancel" type="button" @click="showAddVariantPicker = false">×</button>
        </div>
      </div>
    </div>

    <!-- TuneTabs strip — renders below CarTabs; visible in view mode at 2+ tunes, always in edit mode -->
    <div v-if="isEditing || hasTunes" class="rs-variant-tabs rs-tune-tabs">
      <template v-if="activeCar">
        <div
          v-for="(tune, i) in activeCar.tunes"
          :key="i"
          class="rs-variant-tab-wrap"
          :class="{ 'rs-variant-tab-wrap--active': activeTuneIndex === i }"
        >
          <button
            class="rs-variant-tab"
            :class="{ 'rs-variant-tab--active': activeTuneIndex === i }"
            type="button"
            :title="tune.tuneName || `Tune ${i + 1}`"
            @click="activeTuneIndex = i"
          >{{ tune.tuneName || `Tune ${i + 1}` }}</button>
          <button
            v-if="isEditing && activeCar.tunes.length > 1"
            class="rs-variant-remove"
            type="button"
            :title="`Remove tune ${i + 1}`"
            @click.stop="tuneIsEmpty(tune) ? removeTune(i) : (pendingRemoveTuneIdx = i)"
          >×</button>
        </div>
      </template>
      <button
        v-if="isEditing"
        class="rs-variant-tab rs-variant-tab--add"
        type="button"
        @click="addTuneVariant"
      >+ Tune</button>
    </div>

    <!-- Tune import offer (step 10) — shown when a variant is added for a car that already has tunes -->
    <div v-if="pendingTuneImport" class="rs-tune-import">
      <span class="rs-tune-import-msg">
        Found {{ pendingTuneImport.tunes.length }} existing tune{{ pendingTuneImport.tunes.length > 1 ? 's' : '' }} for
        <strong>{{ carsStore.byId(pendingTuneImport.carId) ? `${carsStore.byId(pendingTuneImport.carId)!.year} ${carsStore.byId(pendingTuneImport.carId)!.make} ${carsStore.byId(pendingTuneImport.carId)!.model}` : pendingTuneImport.carId }}</strong>
        — import one?
      </span>
      <div class="rs-tune-import-options">
        <button
          v-for="t in pendingTuneImport.tunes"
          :key="t.id"
          class="rs-tune-import-btn"
          @click="acceptTuneImport(t)"
        >{{ t.officialName || t.serial }}</button>
        <button class="rs-tune-import-skip" @click="dismissTuneImport">Skip — start fresh</button>
      </div>
    </div>

    <!-- Remove car confirm -->
    <div v-if="pendingRemoveIdx !== null" class="rs-remove-confirm">
      <span>Remove <strong>{{ carLabel(local.cars![pendingRemoveIdx]) }}</strong> and its data?</span>
      <button type="button" class="rs-remove-yes" @click="removeCar(pendingRemoveIdx!)">Remove</button>
      <button type="button" class="rs-remove-no" @click="pendingRemoveIdx = null">Cancel</button>
    </div>

    <!-- Remove tune confirm -->
    <div v-if="pendingRemoveTuneIdx !== null" class="rs-remove-confirm">
      <span>Remove tune <strong>{{ activeCar?.tunes[pendingRemoveTuneIdx]?.tuneName || `Tune ${pendingRemoveTuneIdx! + 1}` }}</strong> and its data?</span>
      <button type="button" class="rs-remove-yes" @click="removeTune(pendingRemoveTuneIdx!)">Remove</button>
      <button type="button" class="rs-remove-no" @click="pendingRemoveTuneIdx = null">Cancel</button>
    </div>

    <div class="tune-header">
      <div class="tune-name-group">
        <div v-if="variantFigurePath" class="rs-variant-figure-wrap">
          <img class="rs-variant-figure" :src="variantFigurePath" :alt="linkedCar?.model ?? ''" @click="modal.openLightbox(variantFigurePath, variantFigureImage!.path)" />
          <img class="rs-variant-preview" :src="variantFigurePath" :alt="linkedCar?.model ?? ''" aria-hidden="true" />
        </div>
        <EditableText tag="p" class="tune-name" :modelValue="isEditing ? local.tuneName : tuneDisplayName" :placeholder="isEditing ? tuneDisplayName : undefined" @update:modelValue="v => { local.tuneName = v; flush() }" />
      </div>
      <div class="plate">
        SHARE CODE:
        <input
          v-if="isEditing"
          class="share-code-input"
          :value="local.shareCode"
          @input="onShareCodeInput"
          placeholder="000 000 000"
          maxlength="11"
          spellcheck="false"
        />
        <b
          v-else
          :class="{ 'share-code-copyable': local.shareCode, 'share-code-copied': codeCopied }"
          @click="copyShareCode"
        >{{ codeCopied ? 'Copied!' : (local.shareCode || '—') }}</b>
      </div>
      <!-- Car identity — uses variant's carId in multi-car mode -->
      <div v-if="isEditing" class="rs-car-row">
        <CarPicker :car-id="effectiveCarId" @update:car-id="onVariantCarIdUpdate" />
      </div>
      <div v-else-if="linkedCar" class="rs-car-badge">
        <span class="rs-game-badge">{{ linkedCar.game }}</span>
        <span class="rs-car-name">{{ linkedCar.year }} {{ linkedCar.make }} {{ linkedCar.model }}</span>
      </div>
    </div>

    <table v-if="isEditing || hasNonStockSpecs" class="recipe-table">
      <tbody>
        <tr>
          <th v-for="k in CORE_SPEC_KEYS" :key="k">{{ k }}</th>
        </tr>
        <tr>
          <td v-for="k in CORE_SPEC_KEYS" :key="k">
            <template v-if="isEditing">
              <select
                v-if="SPEC_OPTIONS[k]"
                class="spec-select"
                @change="onSpecChange(k, $event)"
              >
                <option value="" :selected="!local.coreSpecs[k]">Stock</option>
                <option v-for="opt in SPEC_OPTIONS[k]" :key="opt" :value="opt" :selected="local.coreSpecs[k] === opt">{{ opt }}</option>
              </select>
              <EditableText v-else :modelValue="local.coreSpecs[k]" @update:modelValue="v => { local.coreSpecs[k] = v; flush() }" />
            </template>
            <span v-else>{{ local.coreSpecs[k] || 'Stock' }}</span>
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
      <div v-if="isEditing" class="kit-preset-bar" ref="presetBarEl">
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

      <div class="kit-body" :class="{ 'kit-body--grid': isEditing || showStock }">
        <UpgradesPicker v-if="isEditing" :upgrades="local.upgrades" :show-stock="showStock" :implied-parts="impliedPartNames" />
        <template v-else-if="showStock">
          <!-- Show Stock: full list, Engine pinned left, other cats balanced across cols 2-3 -->
          <div class="upgrades-grid">
            <div
              v-for="cat in allStockCats"
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
                  <li
                    v-if="Array.isArray(p.tiers) && viewInstalledTier(p.tiers, p.specialTiers) !== 'Not Available'"
                    :class="{ 'kit-item--buy': isCustomTier(p.tiers, p.specialTiers) }"
                  >
                    {{ viewPartLabel(p.part, p.tiers, p.specialTiers) }}<span v-if="viewPartCost(p) !== null" class="kit-item-cost"> · CR {{ viewPartCost(p)!.toLocaleString() }}</span>
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
            <div v-for="(cat, ci) in local.upgrades" :key="ci" class="kit-cat">
              <p class="kit-cat-label">{{ cat.category }}</p>
              <ul class="kit-list">
                <li v-for="(part, pi) in cat.parts.filter(p => !SPECIAL_STATES.has(p))" :key="pi">{{ part }}</li>
              </ul>
            </div>
            <p v-if="!local.upgrades.length" class="kit-cat-label" style="opacity:0.35">No upgrades recorded</p>
          </div>
        </template>
      </div>
    </details>

    <p class="kit-cat-label adj-label">Tune Adjustments</p>
    <TuningAdjustments
      ref="taRef"
      :adjustments="local.adjustments"
      :card-id="props.cardId"
      :upgrades="local.upgrades"
      :core-specs="local.coreSpecs"
      @change="flush()"
      @implied-upgrades="onImpliedUpgrades"
      @remove-upgrade="onRemoveUpgrade"
      @springs-choice="onSpringsChoice"
    />
  </div>

  <!-- Car Tabs setup wizard — floating modal -->
  <Teleport to="body">
    <div v-if="showSetupWizard" class="wiz-backdrop" @click.self="showSetupWizard = false">
      <div class="wiz-panel">
        <div class="wiz-header">
          <span class="wiz-title">Set up Car Tabs</span>
          <button class="wiz-close" type="button" @click="showSetupWizard = false">×</button>
        </div>
        <div v-if="wizardLoading" class="wiz-loading">Loading presets…</div>
        <template v-else>
          <div class="wiz-step-label">Car {{ wizardStep + 1 }} of {{ wizardNonAnchorIds.length }}</div>
          <p class="wiz-car-name">{{ wizardCarLabel(wizardNonAnchorIds[wizardStep]) }}</p>
          <label class="wiz-select-label">Starting tune preset</label>
          <select
            class="wiz-select"
            :value="wizardSelections[wizardNonAnchorIds[wizardStep]] ?? ''"
            @change="wizardSelections[wizardNonAnchorIds[wizardStep]] = ($event.target as HTMLSelectElement).value ? Number(($event.target as HTMLSelectElement).value) : null"
          >
            <option value="">None — start blank</option>
            <option v-for="p in wizardPresets" :key="p.id" :value="p.id">{{ p.name }}</option>
          </select>
          <div class="wiz-nav">
            <button
              v-if="wizardStep > 0"
              class="wiz-btn"
              type="button"
              @click="wizardStep--"
            >← Back</button>
            <button
              v-if="wizardStep < wizardNonAnchorIds.length - 1"
              class="wiz-btn wiz-btn--primary"
              type="button"
              @click="wizardStep++"
            >Next →</button>
            <button
              v-else
              class="wiz-btn wiz-btn--primary"
              type="button"
              @click="finishWizard"
            >Set up {{ wizardAllIds.length }} tabs</button>
          </div>
        </template>
      </div>
    </div>
  </Teleport>
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
  outline: 1px solid var(--accent);
  outline-offset: 1px;
}
.kit-item-cost {
  color: var(--accent);
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
  width: 9em;
}
.share-code-input:focus {
  outline: none;
  border-bottom-color: var(--accent);
}
.share-code-input::placeholder { opacity: 0.35; font-weight: normal; }

/* Kill catalog.css's multi-column on kit-body so our grid controls layout */
.kit-body {
  column-width: auto;
  column-count: auto;
}

/* Car identity row (edit) and badge (view) */
.rs-car-row {
  margin-top: 6px;
}
.rs-car-badge {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-top: 4px;
}
.rs-game-badge {
  font: 700 10px/1 'Oswald', sans-serif;
  letter-spacing: 0.08em;
  padding: 2px 5px;
  border-radius: 3px;
  background: var(--muted-light, #444);
  color: var(--text-muted, #aaa);
}
.rs-car-name {
  font: 12px/1 'Oswald', sans-serif;
  color: var(--text-muted, #999);
  letter-spacing: 0.03em;
}

.kit-body--grid {
  column-width: unset;
  column-gap: unset;
}

.upgrades-grid {
  columns: 3;
  column-gap: 16px;
}
.upgrades-grid .kit-cat {
  break-inside: avoid;
  margin-bottom: 16px;
  min-width: 0;
}
.upgrades-grid .up-col3-break { break-before: column; }
@media (max-width: 800px) {
  .upgrades-grid { columns: 2; }
  .upgrades-grid .up-col3-break { break-before: auto; }
  .upgrades-grid .up-col2-break { break-before: column; }
}
@media (max-width: 540px) {
  .upgrades-grid { columns: 1; }
  .upgrades-grid .up-col2-break { break-before: auto; }
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
  color: var(--muted);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: 0.04em;
  padding: 0;
  text-align: left;
  cursor: pointer;
  opacity: 0.5;
  transition: opacity 0.12s, color 0.12s;
}
.kit-preset-trigger:hover { opacity: 1; color: var(--accent); }
/* Make the label take all available space so the button + chevron sit together on the right */
.kit-label-group { flex: 1; }

.kit-stock-btn {
  background: none;
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: var(--muted);
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
.kit-stock-btn.active { color: var(--accent); border-color: var(--accent); opacity: 1; }

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
  color: var(--muted); font-size: 12px; padding: 5px 4px; cursor: pointer; border-radius: 3px;
}
.up-preset-apply:hover { color: var(--accent); }
.up-preset-del {
  background: none; border: none; color: var(--muted); opacity: 0.4;
  font-size: 14px; cursor: pointer; padding: 2px 4px; line-height: 1;
}
.up-preset-del:hover { opacity: 1; color: #e03030; }
.up-preset-divider { height: 1px; background: var(--panel-edge); margin: 4px 8px; }
.up-preset-empty { font-size: 11px; color: var(--muted); opacity: 0.4; padding: 4px 12px 8px; }
.up-preset-save-link {
  background: none; border: none; color: var(--accent); font-size: 11px;
  padding: 4px 12px; cursor: pointer; width: 100%; text-align: left; opacity: 0.8;
}
.up-preset-save-link:hover { opacity: 1; }
.up-preset-save-row { display: flex; align-items: center; gap: 4px; padding: 4px 8px; }
.up-preset-name {
  flex: 1;
  background: color-mix(in srgb, var(--panel) 70%, #000);
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: var(--fg); font-size: 11px; padding: 3px 6px;
}
.up-preset-confirm {
  background: none; border: 1px solid var(--accent); border-radius: 3px;
  color: var(--accent); font-size: 10px; padding: 3px 8px; cursor: pointer;
}
.up-preset-cancel {
  background: none; border: none; color: var(--muted); opacity: 0.5;
  font-size: 14px; padding: 2px 4px; cursor: pointer; line-height: 1;
}
.up-preset-clear {
  background: none; border: none; color: var(--muted); font-size: 11px;
  opacity: 0.4; cursor: pointer; padding: 5px 12px; width: 100%; text-align: left;
}
.up-preset-clear:hover { opacity: 0.9; color: #e03030; }

.kit-item--buy { color: var(--accent); }

/* ── Multi-car variant tab strip ──────────────────────────────────────────── */
.rs-variant-tabs {
  --tune-header-bg: var(--panel);
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 4px;
  margin-bottom: 0;
  padding-top: 5px;
  border-bottom: 1px solid var(--accent);
}
.rs-tune-tabs {
  padding-top: 3px;
  border-bottom-color: color-mix(in srgb, var(--accent) 50%, transparent);
  font-size: 0.88em;
  opacity: 0.85;
}
.rs-variant-tab-wrap {
  display: flex;
  align-items: flex-end;
}
.rs-variant-tab {
  background: transparent;
  border: 1px solid var(--muted);
  border-bottom-color: var(--accent);
  border-radius: 4px 4px 0 0;
  color: var(--muted);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  letter-spacing: .06em;
  text-transform: uppercase;
  padding: 5px 12px;
  cursor: pointer;
  transition: color .15s, border-color .15s;
  margin-bottom: -1px;
}
.rs-variant-tab--active {
  background: var(--tune-header-bg);
  border-color: var(--accent);
  border-bottom-color: var(--tune-header-bg);
  color: var(--accent);
}
.rs-variant-tab:not(.rs-variant-tab--active):hover {
  border-color: color-mix(in srgb, var(--fg) 50%, transparent);
  border-bottom-color: var(--accent);
  color: var(--fg);
}
.rs-variant-tab--add {
  border: 1px dashed var(--accent);
  border-radius: 4px;
  color: var(--accent);
  margin-bottom: 4px;
  opacity: 0.65;
}
.rs-variant-tab--add:hover { opacity: 1; }
.rs-variant-remove {
  background: none;
  border: none;
  color: var(--muted);
  opacity: 0.35;
  cursor: pointer;
  font-size: 14px;
  padding: 0 3px;
  line-height: 1;
  transition: opacity .12s, color .12s;
}
.rs-variant-remove:hover { opacity: 1; color: #e03030; }

.rs-add-variant-wrap { display: flex; align-items: center; gap: 4px; margin-bottom: 4px; }

.rs-autopropose {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px 4px 10px;
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  border: 1px solid color-mix(in srgb, var(--accent) 25%, transparent);
  border-radius: 4px;
  margin-right: 6px;
}
.rs-autopropose-msg {
  font: 10px/1 'JetBrains Mono', monospace;
  color: var(--accent);
  white-space: nowrap;
}
.rs-autopropose-accept {
  font: 700 10px/1 'Oswald', sans-serif;
  letter-spacing: .04em;
  padding: 3px 8px;
  border-radius: 3px;
  border: 1px solid var(--accent);
  background: var(--accent);
  color: #000;
  cursor: pointer;
  white-space: nowrap;
}
.rs-autopropose-dismiss {
  background: none;
  border: none;
  color: var(--muted);
  font-size: 13px;
  cursor: pointer;
  padding: 0;
  line-height: 1;
}
.rs-autopropose-dismiss:hover { color: var(--fg); }

.rs-variant-tab--suggested {
  border-style: dashed;
  color: color-mix(in srgb, var(--accent) 70%, var(--muted));
  opacity: 0.85;
}
.rs-variant-tab--suggested.rs-variant-tab--active {
  opacity: 1;
}
.rs-tab-suggested-badge {
  display: inline-block;
  font-size: 8px;
  font-family: 'JetBrains Mono', monospace;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  padding: 1px 4px;
  margin-left: 5px;
  border-radius: 2px;
  background: color-mix(in srgb, var(--accent) 15%, transparent);
  color: var(--accent);
  vertical-align: middle;
}
.rs-add-picker-inline { display: flex; align-items: center; gap: 6px; }
.rs-add-picker-cancel {
  background: none; border: none; color: var(--muted); opacity: 0.5;
  font-size: 16px; cursor: pointer; padding: 0 4px; line-height: 1;
}
.rs-add-picker-cancel:hover { opacity: 1; }

.rs-tune-import {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 9px 12px;
  margin-bottom: 10px;
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
  border-radius: 5px;
  font-size: 12px;
  color: var(--muted);
}
.rs-tune-import-msg strong { color: var(--fg); }
.rs-tune-import-options { display: flex; flex-wrap: wrap; gap: 5px; align-items: center; }
.rs-tune-import-btn {
  background: none;
  border: 1px solid var(--accent);
  border-radius: 3px;
  color: var(--accent);
  font-size: 11px;
  font-family: 'JetBrains Mono', monospace;
  padding: 3px 10px;
  cursor: pointer;
  transition: background 0.12s;
}
.rs-tune-import-btn:hover { background: color-mix(in srgb, var(--accent) 15%, transparent); }
.rs-tune-import-skip {
  background: none; border: 1px solid var(--panel-edge);
  border-radius: 3px; color: var(--muted); font-size: 11px;
  padding: 3px 10px; cursor: pointer; opacity: 0.7;
}
.rs-tune-import-skip:hover { opacity: 1; color: var(--fg); }

.rs-remove-confirm {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 10px;
  margin-bottom: 10px;
  background: color-mix(in srgb, var(--danger, #e03030) 8%, transparent);
  border: 1px solid color-mix(in srgb, var(--danger, #e03030) 30%, transparent);
  border-radius: 4px;
  font-size: 12px;
  color: var(--muted);
}
.rs-remove-confirm strong { color: var(--fg); }
.rs-remove-yes {
  background: none; border: 1px solid var(--danger, #e03030);
  border-radius: 3px; color: var(--danger, #e03030); font-size: 11px;
  padding: 3px 10px; cursor: pointer; white-space: nowrap;
}
.rs-remove-yes:hover { background: color-mix(in srgb, var(--danger, #e03030) 15%, transparent); }
.rs-remove-no {
  background: none; border: 1px solid var(--panel-edge);
  border-radius: 3px; color: var(--muted); font-size: 11px;
  padding: 3px 10px; cursor: pointer;
}
.rs-remove-no:hover { border-color: var(--fg); color: var(--fg); }
</style>

<style>
.wiz-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,.55);
  z-index: 9000;
  display: flex;
  align-items: center;
  justify-content: center;
}
.wiz-panel {
  background: var(--bg, #111);
  border: 1px solid var(--accent, gold);
  border-radius: 6px;
  padding: 20px 24px 24px;
  width: min(400px, 90vw);
  box-shadow: 0 8px 32px rgba(0,0,0,.6);
}
.wiz-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}
.wiz-title {
  font: 700 14px/1 'Oswald', sans-serif;
  letter-spacing: .08em;
  text-transform: uppercase;
  color: var(--accent, gold);
}
.wiz-close {
  background: none;
  border: none;
  color: var(--muted, #888);
  font-size: 18px;
  cursor: pointer;
  padding: 0;
  line-height: 1;
}
.wiz-close:hover { color: var(--fg, #eee); }
.wiz-step-label {
  font: 10px/1 'JetBrains Mono', monospace;
  color: var(--muted, #888);
  margin-bottom: 6px;
}
.wiz-car-name {
  font: 600 15px/1.2 'Oswald', sans-serif;
  color: var(--fg, #eee);
  margin: 0 0 14px;
}
.wiz-select-label {
  display: block;
  font: 10px/1 'JetBrains Mono', monospace;
  color: var(--muted, #888);
  margin-bottom: 6px;
}
.wiz-select {
  width: 100%;
  padding: 6px 8px;
  font: 12px/1 'JetBrains Mono', monospace;
  background: var(--bg, #111);
  color: var(--fg, #eee);
  border: 1px solid var(--muted, #888);
  border-radius: 3px;
  margin-bottom: 18px;
}
.wiz-loading {
  font: 11px/1 'JetBrains Mono', monospace;
  color: var(--muted, #888);
  padding: 16px 0;
}
.wiz-nav {
  display: flex;
  gap: 8px;
  align-items: center;
}
.wiz-btn {
  font: 700 11px/1 'Oswald', sans-serif;
  letter-spacing: .05em;
  padding: 5px 14px;
  border-radius: 3px;
  border: 1px solid var(--muted, #888);
  background: none;
  color: var(--fg, #eee);
  cursor: pointer;
}
.wiz-btn--primary {
  border-color: var(--accent, gold);
  background: var(--accent, gold);
  color: #000;
}
.wiz-btn--primary:hover { opacity: .85; }

.share-code-copyable {
  cursor: pointer;
}
.share-code-copyable:hover {
  color: var(--accent, gold);
}
.share-code-copied {
  color: var(--accent, gold);
}

.tune-header {
  background: var(--panel);
  border-radius: 3px;
  padding: 12px;
  margin: 0;
  align-items: center;
}

.tune-name-group {
  display: flex;
  align-items: center;
  gap: 8px;
}
.rs-variant-figure-wrap {
  position: relative;
  flex-shrink: 0;
}
.rs-variant-figure {
  display: block;
  height: 48px;
  width: auto;
  object-fit: cover;
  border-radius: 3px;
  cursor: pointer;
}
.rs-variant-preview {
  display: none;
  position: absolute;
  bottom: calc(100% + 6px);
  left: 0;
  width: 200px;
  border-radius: 4px;
  box-shadow: 0 4px 20px rgba(0,0,0,0.6);
  pointer-events: none;
  z-index: 10;
}
.rs-variant-figure-wrap:hover .rs-variant-preview {
  display: block;
}
</style>
