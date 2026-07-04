<script setup lang="ts">
import { ref, watch, computed, onMounted, nextTick } from 'vue'
import 'vanilla-colorful/hex-color-picker.js'

const props = defineProps<{
  modelValue: string
}>()
const emit = defineEmits<{ 'update:modelValue': [v: string] }>()

const pickerEl = ref<(HTMLElement & { color: string }) | null>(null)
const hexInput = ref(props.modelValue)

function hexToRgb(h: string): { r: number; g: number; b: number } {
  const n = parseInt(h.replace('#', ''), 16)
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 }
}

function rgbToHex(r: number, g: number, b: number): string {
  return '#' + [r, g, b]
    .map(v => Math.min(255, Math.max(0, Math.round(v))).toString(16).padStart(2, '0'))
    .join('')
}

function hexToHsl(hex: string): { h: number; s: number; l: number } {
  const { r, g, b } = hexToRgb(hex)
  const r1 = r / 255, g1 = g / 255, b1 = b / 255
  const max = Math.max(r1, g1, b1), min = Math.min(r1, g1, b1)
  const l = (max + min) / 2
  if (max === min) return { h: 0, s: 0, l: Math.round(l * 100) }
  const d = max - min
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
  let h = 0
  if (max === r1) h = ((g1 - b1) / d + (g1 < b1 ? 6 : 0)) / 6
  else if (max === g1) h = ((b1 - r1) / d + 2) / 6
  else h = ((r1 - g1) / d + 4) / 6
  return { h: Math.round(h * 360), s: Math.round(s * 100), l: Math.round(l * 100) }
}

function hslToHex(h: number, s: number, l: number): string {
  const s1 = s / 100, l1 = l / 100
  const a = s1 * Math.min(l1, 1 - l1)
  const f = (n: number) => {
    const k = (n + h / 30) % 12
    return Math.round(255 * (l1 - a * Math.max(-1, Math.min(k - 3, 9 - k, 1))))
  }
  return rgbToHex(f(0), f(8), f(4))
}

function luminance(hex: string): number {
  const { r, g, b } = hexToRgb(hex)
  const weights = [0.2126, 0.7152, 0.0722]
  return [r, g, b].reduce((sum, c, i) => {
    const s = c / 255
    return sum + (s <= 0.04045 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4)) * weights[i]
  }, 0)
}

function contrastRatio(a: string, b: string): number {
  try {
    const lums = [luminance(a), luminance(b)].sort((x, y) => y - x)
    return (lums[0] + 0.05) / (lums[1] + 0.05)
  } catch {
    return 21
  }
}

const rgb = computed(() => hexToRgb(hexInput.value))
const hsl = computed(() => hexToHsl(hexInput.value))

const hueGradient = 'linear-gradient(to right,#f00 0%,#ff0 17%,#0f0 33%,#0ff 50%,#00f 67%,#f0f 83%,#f00 100%)'
const satGradient = computed(() => {
  const { h, l } = hsl.value
  return `linear-gradient(to right,hsl(${h},0%,${l}%),hsl(${h},100%,${l}%))`
})
const litGradient = computed(() => {
  const { h, s } = hsl.value
  return `linear-gradient(to right,hsl(${h},${s}%,0%),hsl(${h},${s}%,50%),hsl(${h},${s}%,100%))`
})

function colorName(hex: string): string {
  const { h, s, l } = hexToHsl(hex)

  if (l < 5)  return 'Near Black'
  if (l > 96) return 'Near White'

  // Truly achromatic — no hue cast at all
  if (s < 5) {
    if (l < 18) return 'Charcoal'
    if (l < 38) return 'Dark Grey'
    if (l < 60) return 'Grey'
    if (l < 80) return 'Light Grey'
    return 'Silver'
  }

  // Tinted neutrals — even a subtle cast gets named (this is where the nuance lives)
  if (s < 50) {
    const tint =
      (h < 30 || h >= 335) ? 0 :  // warm (red/orange)
      h < 75               ? 1 :  // earthy (yellow/khaki)
      h < 110              ? 2 :  // grey-green (olive)
      h < 165              ? 3 :  // sage (muted green)
      h < 200              ? 4 :  // slate (teal-grey)
      h < 265              ? 5 :  // cool (blue-grey)
      h < 300              ? 6 :  // mauve (purple-grey)
                             0    // pink → warm
    //                  l<20            l<40            l<62          l<80          l>=80
    const names = [
      ['Dark Warm Grey',   'Warm Dark Grey',  'Warm Grey',   'Warm Silver', 'Blush'     ],
      ['Dark Earthy Grey', 'Earthy Grey',     'Warm Taupe',  'Warm Ivory',  'Ivory'     ],
      ['Dark Grey-Green',  'Muted Olive',     'Dusty Green', 'Pale Green',  'Pale Green'],
      ['Dark Sage',        'Sage Grey',       'Sage',        'Pale Sage',   'Pale Sage' ],
      ['Dark Slate',       'Slate',           'Slate Blue',  'Pale Slate',  'Ice Blue'  ],
      ['Dark Cool Grey',   'Cool Dark Grey',  'Cool Grey',   'Cool Silver', 'Ice'       ],
      ['Dark Mauve',       'Dusty Mauve',     'Mauve',       'Pale Mauve',  'Lavender'  ],
    ]
    const band = l < 20 ? 0 : l < 40 ? 1 : l < 62 ? 2 : l < 80 ? 3 : 4
    return names[tint][band]
  }

  // Fully saturated
  const hueName =
    h < 16  ? 'Red'    : h < 40  ? 'Orange' : h < 65  ? 'Yellow' :
    h < 80  ? 'Lime'   : h < 150 ? 'Green'  : h < 185 ? 'Teal'   :
    h < 210 ? 'Cyan'   : h < 255 ? 'Blue'   : h < 280 ? 'Violet' :
    h < 315 ? 'Purple' : h < 345 ? 'Pink'   : 'Red'
  const light = l < 22 ? 'Deep' : l < 40 ? 'Dark' : l > 78 ? 'Pale' : l > 62 ? 'Light' : ''
  const sat   = s > 75 ? 'Vivid' : ''
  return [light, sat, hueName].filter(Boolean).join(' ')
}

const generatedName = computed(() => colorName(hexInput.value))

const panelColor = computed(() =>
  getComputedStyle(document.documentElement).getPropertyValue('--panel').trim() || '#15151a'
)

const lowContrast = computed(() => contrastRatio(hexInput.value, panelColor.value) < 4.5)

function emit_(hex: string) {
  hexInput.value = hex
  if (pickerEl.value) pickerEl.value.color = hex
  emit('update:modelValue', hex)
}

onMounted(() => {
  const el = pickerEl.value
  if (!el) return
  el.color = hexInput.value
  el.addEventListener('color-changed', (e: Event) => {
    const v = (e as CustomEvent<{ value: string }>).detail.value
    hexInput.value = v
    emit('update:modelValue', v)
  })
})

watch(() => props.modelValue, (v) => {
  hexInput.value = v
  if (pickerEl.value && pickerEl.value.color !== v) pickerEl.value.color = v
})

function onHexInput(e: Event) {
  const raw = (e.target as HTMLInputElement).value
  if (/^#[0-9a-fA-F]{6}$/.test(raw)) emit_(raw)
}

function onRgbInput(channel: 'r' | 'g' | 'b', e: Event) {
  const v = parseInt((e.target as HTMLInputElement).value)
  if (isNaN(v)) return
  const cur = hexToRgb(hexInput.value)
  cur[channel] = v
  emit_(rgbToHex(cur.r, cur.g, cur.b))
}

function onHslInput(channel: 'h' | 's' | 'l', e: Event) {
  const v = parseInt((e.target as HTMLInputElement).value)
  if (isNaN(v)) return
  const cur = hsl.value
  const next = { ...cur, [channel]: v }
  emit_(hslToHex(next.h, next.s, next.l))
}

const FH_PALETTE = [
  { name: 'Bright Tokyo red',         hex: '#FF3B2F' },
  { name: 'Rising sun red',           hex: '#D6432C' },
  { name: 'Lantern red',              hex: '#C81E3A' },
  { name: 'Deep maroon',              hex: '#8C2A22' },
  { name: 'Sunset orange',            hex: '#E8650F' },
  { name: 'Horizon orange',           hex: '#F4831F' },
  { name: 'Gold',                     hex: '#C9A227' },
  { name: 'Horizon Tour gold',        hex: '#EAA63C' },
  { name: 'Checkpoint yellow',        hex: '#F5D033' },
  { name: 'Eliminator neon green',    hex: '#7FFF3C' },
  { name: 'Battle green',             hex: '#5BDB2E' },
  { name: 'Circuit green',            hex: '#3FBE3E' },
  { name: 'Forest eliminator green',  hex: '#2A9D4A' },
  { name: 'Deep eliminator green',    hex: '#167A3E' },
  { name: 'Danger sign teal',         hex: '#1FD1A5' },
  { name: 'Speed flare cyan',         hex: '#29C5F6' },
  { name: 'Speed trap blue',          hex: '#1E6FE0' },
  { name: 'Indigo night sky',         hex: '#2A2F6B' },
  { name: 'Drift zone purple',        hex: '#8A2BE2' },
  { name: 'Festival night pink',      hex: '#E63DD0' },
  { name: 'Hot pink',                 hex: '#D6478F' },
  { name: 'Style flare magenta',      hex: '#FF2D7A' },
  { name: 'Sakura pink',              hex: '#F2A6C8' },
  { name: 'Pure white',               hex: '#FFFFFF' },
  { name: 'Panel slate',              hex: '#2B2B33' },
  { name: 'Menu charcoal',            hex: '#16161A' },
  { name: 'Checkered black',          hex: '#0A0A0A' },
]

type Swatch = { name: string; hex: string; builtin: boolean }

const PALETTE_KEY = 'cp-palette'

const DEFAULT_PALETTE: Swatch[] = FH_PALETTE.map(s => ({ ...s, builtin: true }))

function loadPalette(): Swatch[] {
  try {
    const stored = localStorage.getItem(PALETTE_KEY)
    if (stored) return JSON.parse(stored)
    // migrate old separate user-swatches if present
    const old = JSON.parse(localStorage.getItem('cp-user-swatches') ?? '[]')
    return [...DEFAULT_PALETTE, ...old.map((s: { name: string; hex: string }) => ({ ...s, builtin: false }))]
  } catch { return [...DEFAULT_PALETTE] }
}

const palette = ref<Swatch[]>(loadPalette())

function savePalette() {
  localStorage.setItem(PALETTE_KEY, JSON.stringify(palette.value))
}

function snapToNearest() {
  const cur = hexToRgb(props.modelValue)
  let best = palette.value[0]?.hex ?? FH_PALETTE[0].hex
  let bestDist = Infinity
  for (const sw of palette.value) {
    const s = hexToRgb(sw.hex)
    const d = Math.sqrt((cur.r - s.r) ** 2 + (cur.g - s.g) ** 2 + (cur.b - s.b) ** 2)
    if (d < bestDist) { bestDist = d; best = sw.hex }
  }
  emit_(best)
}

const selectedSwatch = ref<Swatch | null>(null)

const swatchDeviated = computed(() =>
  !!selectedSwatch.value &&
  hexInput.value.toLowerCase() !== selectedSwatch.value.hex.toLowerCase()
)

function isActive(hex: string): boolean {
  return selectedSwatch.value?.hex.toLowerCase() === hex.toLowerCase()
}

function clickSwatch(sw: Swatch) {
  if (selectedSwatch.value?.hex.toLowerCase() === sw.hex.toLowerCase()) {
    selectedSwatch.value = null
  } else {
    selectedSwatch.value = sw
    emit_(sw.hex)
  }
}

const addDialogOpen = ref(false)
const addDialogName = ref('')
const addDialogNameInput = ref<HTMLInputElement | null>(null)

function openAddDialog() {
  addDialogName.value = hexInput.value
  addDialogOpen.value = true
  nextTick(() => { addDialogNameInput.value?.select() })
}

function confirmAddSwatch() {
  const hex = hexInput.value
  const name = addDialogName.value.trim() || hex
  if (!palette.value.some(s => s.hex.toLowerCase() === hex.toLowerCase())) {
    palette.value.push({ name, hex, builtin: false })
    savePalette()
  }
  addDialogOpen.value = false
}

function cancelAddSwatch() {
  addDialogOpen.value = false
}

function removeSwatch(hex: string) {
  const i = palette.value.findIndex(s => s.hex === hex && !s.builtin)
  if (i !== -1) { palette.value.splice(i, 1); savePalette() }
}

const isDragging = ref(false)
const draggedHex = ref<string | null>(null)
const liveItems = ref<Swatch[]>([])

function onSwatchPointerDown(e: PointerEvent, hex: string) {
  const startX = e.clientX
  const startY = e.clientY
  let dragged = false
  let reorderPending = false

  const onMove = (ev: PointerEvent) => {
    if (!dragged && Math.hypot(ev.clientX - startX, ev.clientY - startY) > 5) {
      dragged = true
      isDragging.value = true
      draggedHex.value = hex
      liveItems.value = [...palette.value]
    }
    if (!dragged || reorderPending) return
    const el = document.elementFromPoint(ev.clientX, ev.clientY)
    const targetHex = (el?.closest('[data-hex]') as HTMLElement | null)?.dataset.hex
    if (!targetHex || targetHex === draggedHex.value) return
    const from = liveItems.value.findIndex(s => s.hex === draggedHex.value)
    const to = liveItems.value.findIndex(s => s.hex === targetHex)
    if (from === -1 || to === -1) return
    reorderPending = true
    const items = [...liveItems.value]
    const [moved] = items.splice(from, 1)
    items.splice(to, 0, moved)
    liveItems.value = items
    // wait two frames so the DOM settles before the next reorder can fire
    requestAnimationFrame(() => requestAnimationFrame(() => { reorderPending = false }))
  }

  const onUp = () => {
    document.removeEventListener('pointermove', onMove)
    document.removeEventListener('pointerup', onUp)
    if (dragged) {
      palette.value = liveItems.value
      savePalette()
    }
    liveItems.value = []
    draggedHex.value = null
    isDragging.value = false
  }

  document.addEventListener('pointermove', onMove)
  document.addEventListener('pointerup', onUp)
}
</script>

<template>
  <div class="cp-wrap">
    <!-- Title / selection indicator -->
    <div class="cp-title">
      <span
        class="cp-title-swatch"
        :style="{ background: selectedSwatch?.hex ?? 'transparent', border: selectedSwatch ? 'none' : '1px solid var(--steel)' }"
        :class="{ 'cp-title-swatch--clickable': selectedSwatch && swatchDeviated }"
        :title="selectedSwatch && swatchDeviated ? 'Reset to ' + selectedSwatch.name : undefined"
        @click="selectedSwatch && swatchDeviated && emit_(selectedSwatch.hex)"
      />
      <span class="cp-title-name" :class="{ 'cp-title-name--empty': !selectedSwatch }">
        {{ selectedSwatch?.name ?? generatedName }}
      </span>
      <span v-if="swatchDeviated" class="cp-title-modified">+</span>
      <button v-if="selectedSwatch" class="cp-title-clear" type="button" title="Deselect swatch" @click="selectedSwatch = null">×</button>
    </div>

    <!-- Gradient picker -->
    <hex-color-picker ref="pickerEl" class="cp-picker" />

    <!-- Inputs -->
    <div class="cp-fields">
      <div class="cp-hex-row">
        <label class="cp-field-label">Hex</label>
        <input
          class="cp-hex-input"
          :value="hexInput"
          maxlength="7"
          spellcheck="false"
          @input="onHexInput"
        />
        <button
          class="cp-snap"
          type="button"
          title="Snap to nearest FH color"
          @click="snapToNearest"
        >⌖</button>
        <span v-if="lowContrast" class="cp-warn" title="Low contrast against panel background">⚠</span>
      </div>
      <div class="cp-rgb-row">
        <label class="cp-field-label">R</label>
        <input class="cp-rgb-input" type="number" min="0" max="255" :value="rgb.r" @change="onRgbInput('r', $event)" />
        <label class="cp-field-label">G</label>
        <input class="cp-rgb-input" type="number" min="0" max="255" :value="rgb.g" @change="onRgbInput('g', $event)" />
        <label class="cp-field-label">B</label>
        <input class="cp-rgb-input" type="number" min="0" max="255" :value="rgb.b" @change="onRgbInput('b', $event)" />
      </div>

      <div class="cp-hsl-row">
        <label class="cp-field-label">H</label>
        <div class="cp-slider-wrap" :style="{ '--track-bg': hueGradient }">
          <input class="cp-slider" type="range" min="0" max="360" :value="hsl.h" @input="onHslInput('h', $event)" />
        </div>
        <span class="cp-hsl-val">{{ hsl.h }}°</span>
      </div>
      <div class="cp-hsl-row">
        <label class="cp-field-label">S</label>
        <div class="cp-slider-wrap" :style="{ '--track-bg': satGradient }">
          <input class="cp-slider" type="range" min="0" max="100" :value="hsl.s" @input="onHslInput('s', $event)" />
        </div>
        <span class="cp-hsl-val">{{ hsl.s }}%</span>
      </div>
      <div class="cp-hsl-row">
        <label class="cp-field-label">L</label>
        <div class="cp-slider-wrap" :style="{ '--track-bg': litGradient }">
          <input class="cp-slider" type="range" min="0" max="100" :value="hsl.l" @input="onHslInput('l', $event)" />
        </div>
        <span class="cp-hsl-val">{{ hsl.l }}%</span>
      </div>
    </div>

    <!-- Palette -->
    <div class="cp-palette-header">
      <span class="cp-palette-label">Palette</span>
      <button class="cp-add-swatch" type="button" title="Add current color to palette" @click="openAddDialog">+</button>
    </div>
    <div class="cp-swatches-scroll">
      <TransitionGroup tag="div" class="cp-swatches" name="sw">
        <button
          v-for="sw in (isDragging ? liveItems : palette)"
          :key="sw.hex"
          class="cp-swatch"
          :class="{ 'cp-swatch--active': isActive(sw.hex), 'cp-swatch--dragging': sw.hex === draggedHex }"
          type="button"
          :data-hex="sw.hex"
          :title="sw.name"
          :style="{ background: sw.hex }"
          @click="clickSwatch(sw)"
          @pointerdown="onSwatchPointerDown($event, sw.hex)"
        >
          <span v-if="isActive(sw.hex)" class="cp-swatch-dot" />
          <span v-if="!sw.builtin" class="cp-swatch-remove" @click.stop="removeSwatch(sw.hex)">×</span>
        </button>
      </TransitionGroup>
    </div>

    <!-- Add swatch dialog -->
    <div v-if="addDialogOpen" class="cp-dialog">
      <div class="cp-dialog-preview" :style="{ background: hexInput }" />
      <div class="cp-dialog-info">
        <span class="cp-dialog-hex">{{ hexInput }}</span>
        <span class="cp-dialog-rgb">{{ rgb.r }}, {{ rgb.g }}, {{ rgb.b }}</span>
        <span class="cp-dialog-hsl">{{ hsl.h }}° {{ hsl.s }}% {{ hsl.l }}%</span>
      </div>
      <input
        ref="addDialogNameInput"
        class="cp-dialog-name"
        type="text"
        placeholder="Name this swatch…"
        :value="addDialogName"
        @input="addDialogName = ($event.target as HTMLInputElement).value"
        @keydown.enter="confirmAddSwatch"
        @keydown.esc="cancelAddSwatch"
      />
      <div class="cp-dialog-actions">
        <button class="cp-dialog-btn cp-dialog-btn--cancel" type="button" @click="cancelAddSwatch">Cancel</button>
        <button class="cp-dialog-btn cp-dialog-btn--add" type="button" @click="confirmAddSwatch">Add</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cp-wrap {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  height: 100%;
}
.cp-title {
  display: flex;
  align-items: center;
  gap: 6px;
  min-height: 18px;
}
.cp-title-swatch {
  width: 20px;
  height: 20px;
  border-radius: 3px;
  flex-shrink: 0;
}
.cp-title-swatch--clickable {
  cursor: pointer;
  outline: 2px solid var(--gold);
  outline-offset: 1px;
}
.cp-title-name {
  flex: 1;
  font-family: 'Oswald', sans-serif;
  font-size: 14px;
  text-transform: uppercase;
  color: var(--paper);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  letter-spacing: .06em;
}
.cp-title-name--empty {
  color: var(--steel);
}
.cp-title-modified {
  color: var(--gold);
  font-size: 10px;
  line-height: 1;
  flex-shrink: 0;
}
.cp-title-clear {
  background: none;
  border: none;
  color: var(--steel);
  cursor: pointer;
  font-size: 13px;
  line-height: 1;
  padding: 0;
  flex-shrink: 0;
  opacity: 0.6;
}
.cp-title-clear:hover {
  color: var(--paper);
  opacity: 1;
}
.cp-picker {
  width: 100%;
  height: 160px;
}
.cp-fields {
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.cp-hex-row, .cp-rgb-row {
  display: flex;
  align-items: center;
  gap: 5px;
}
.cp-field-label {
  color: var(--steel);
  min-width: 12px;
}
.cp-hex-input {
  flex: 1;
  background: var(--panel-well);
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: var(--paper);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  padding: 3px 5px;
}
.cp-hex-input:focus { outline: none; border-color: var(--gold); }
.cp-rgb-input {
  flex: 1;
  background: var(--panel-well);
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: var(--paper);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  padding: 3px 5px;
  -moz-appearance: textfield;
}
.cp-rgb-input::-webkit-outer-spin-button,
.cp-rgb-input::-webkit-inner-spin-button { -webkit-appearance: none; }
.cp-rgb-input:focus { outline: none; border-color: var(--gold); }
.cp-snap {
  background: none;
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: var(--steel);
  font-size: 13px;
  line-height: 1;
  padding: 2px 4px;
  cursor: pointer;
}
.cp-snap:hover { border-color: var(--gold); color: var(--gold); }
.cp-warn {
  color: #f4a636;
  font-size: 12px;
}
.cp-palette-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: 4px;
  border-top: 1px solid var(--panel-edge);
}
.cp-palette-label {
  color: var(--steel);
  text-transform: uppercase;
  letter-spacing: .08em;
}
.cp-add-swatch {
  background: none;
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: var(--steel);
  font-size: 15px;
  line-height: 1;
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  padding: 0;
  transition: border-color .15s, color .15s;
}
.cp-add-swatch:hover { border-color: var(--gold); color: var(--gold); }
.cp-swatches-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  overscroll-behavior: contain;
}
.cp-swatches-scroll::-webkit-scrollbar { width: 4px; }
.cp-swatches-scroll::-webkit-scrollbar-track { background: transparent; }
.cp-swatches-scroll::-webkit-scrollbar-thumb { background: var(--panel-edge); border-radius: 2px; }
.cp-swatches {
  display: flex;
  flex-wrap: wrap;
  gap: 3px;
  padding: 4px 0;
}
.sw-move {
  transition: transform 0.15s ease;
}
.cp-swatch {
  width: 44px;
  height: 44px;
  border-radius: 4px;
  border: 1px solid rgba(255,255,255,0.1);
  cursor: grab;
  touch-action: none;
  position: relative;
  padding: 0;
}
.cp-swatch--active {
  box-shadow: 0 0 0 2px var(--gold);
}
.cp-swatch--dragging {
  cursor: grabbing;
  box-shadow: 0 0 0 2px var(--gold), 0 0 10px 2px rgba(201,162,39,0.5);
  z-index: 1;
}
.cp-swatch .cp-swatch-remove {
  position: absolute;
  top: 2px;
  right: 2px;
  width: 14px;
  height: 14px;
  display: none;
  align-items: center;
  justify-content: center;
  background: rgba(0,0,0,0.7);
  color: #fff;
  font-size: 11px;
  line-height: 1;
  border-radius: 2px;
}
.cp-swatch:hover .cp-swatch-remove {
  display: flex;
}
.cp-swatch-dot {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}
.cp-swatch-dot::after {
  content: '';
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: rgba(255,255,255,0.9);
  box-shadow: 0 0 0 1px rgba(0,0,0,0.4);
}

.cp-hsl-row {
  display: flex;
  align-items: center;
  gap: 6px;
}
.cp-slider-wrap {
  flex: 1;
  height: 14px;
  display: flex;
  align-items: center;
}
.cp-slider {
  width: 100%;
  -webkit-appearance: none;
  appearance: none;
  background: transparent;
  cursor: pointer;
}
.cp-slider::-webkit-slider-runnable-track {
  height: 8px;
  border-radius: 4px;
  background: var(--track-bg);
  border: 1px solid rgba(0,0,0,0.2);
}
.cp-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #fff;
  border: 1px solid rgba(0,0,0,0.35);
  box-shadow: 0 1px 3px rgba(0,0,0,0.5);
  margin-top: -4px;
}
.cp-slider::-moz-range-track {
  height: 8px;
  border-radius: 4px;
  background: var(--track-bg);
  border: 1px solid rgba(0,0,0,0.2);
}
.cp-slider::-moz-range-thumb {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #fff;
  border: 1px solid rgba(0,0,0,0.35);
  box-shadow: 0 1px 3px rgba(0,0,0,0.5);
}
.cp-hsl-val {
  color: var(--steel);
  font-size: 10px;
  min-width: 30px;
  text-align: right;
}

/* Add-swatch dialog */
.cp-dialog {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px;
  background: var(--panel-well);
  border: 1px solid var(--panel-edge);
  border-radius: 5px;
  margin-top: 2px;
}
.cp-dialog-preview {
  width: 100%;
  height: 36px;
  border-radius: 3px;
  border: 1px solid rgba(0,0,0,0.2);
  flex-shrink: 0;
}
.cp-dialog-info {
  display: flex;
  gap: 8px;
  align-items: center;
}
.cp-dialog-hex {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--paper);
  letter-spacing: .04em;
}
.cp-dialog-rgb,
.cp-dialog-hsl {
  font-size: 10px;
  color: var(--steel);
  white-space: nowrap;
}
.cp-dialog-name {
  width: 100%;
  box-sizing: border-box;
  background: var(--glass-bg);
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: var(--paper);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  padding: 5px 7px;
}
.cp-dialog-name:focus { outline: none; border-color: var(--gold); }
.cp-dialog-actions {
  display: flex;
  gap: 6px;
  justify-content: flex-end;
}
.cp-dialog-btn {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  padding: 4px 10px;
  border-radius: 3px;
  border: 1px solid var(--panel-edge);
  cursor: pointer;
  text-transform: uppercase;
  letter-spacing: .06em;
  transition: border-color .15s, color .15s, background .15s;
}
.cp-dialog-btn--cancel {
  background: none;
  color: var(--steel);
}
.cp-dialog-btn--cancel:hover { border-color: var(--steel); color: var(--paper); }
.cp-dialog-btn--add {
  background: var(--gold);
  border-color: var(--gold);
  color: #000;
}
.cp-dialog-btn--add:hover { filter: brightness(1.15); }
</style>
