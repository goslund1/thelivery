<script setup lang="ts">
import { ref, watch, computed, onMounted } from 'vue'
import 'vanilla-colorful/hex-color-picker.js'

const props = defineProps<{
  modelValue: string
  label?: string
}>()
const emit = defineEmits<{ 'update:modelValue': [v: string] }>()

const pickerEl = ref<(HTMLElement & { value: string }) | null>(null)
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

const rgb = computed(() => hexToRgb(props.modelValue))
const hsl = computed(() => hexToHsl(props.modelValue))

const hueGradient = 'linear-gradient(to right,#f00 0%,#ff0 17%,#0f0 33%,#0ff 50%,#00f 67%,#f0f 83%,#f00 100%)'
const satGradient = computed(() => {
  const { h, l } = hsl.value
  return `linear-gradient(to right,hsl(${h},0%,${l}%),hsl(${h},100%,${l}%))`
})
const litGradient = computed(() => {
  const { h, s } = hsl.value
  return `linear-gradient(to right,hsl(${h},${s}%,0%),hsl(${h},${s}%,50%),hsl(${h},${s}%,100%))`
})

const panelColor = computed(() =>
  getComputedStyle(document.documentElement).getPropertyValue('--panel').trim() || '#15151a'
)

const lowContrast = computed(() => contrastRatio(props.modelValue, panelColor.value) < 4.5)

function emit_(hex: string) {
  hexInput.value = hex
  if (pickerEl.value) pickerEl.value.value = hex
  emit('update:modelValue', hex)
}

onMounted(() => {
  const el = pickerEl.value
  if (!el) return
  el.value = props.modelValue
  el.addEventListener('color-changed', (e: Event) => {
    const v = (e as CustomEvent<{ value: string }>).detail.value
    hexInput.value = v
    emit('update:modelValue', v)
  })
})

watch(() => props.modelValue, (v) => {
  hexInput.value = v
  if (pickerEl.value && pickerEl.value.value !== v) pickerEl.value.value = v
})

function onHexInput(e: Event) {
  const raw = (e.target as HTMLInputElement).value
  if (/^#[0-9a-fA-F]{6}$/.test(raw)) emit_(raw)
}

function onRgbInput(channel: 'r' | 'g' | 'b', e: Event) {
  const v = parseInt((e.target as HTMLInputElement).value)
  if (isNaN(v)) return
  const cur = hexToRgb(props.modelValue)
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

function snapToNearest() {
  const cur = hexToRgb(props.modelValue)
  let best = FH_PALETTE[0].hex
  let bestDist = Infinity
  for (const sw of FH_PALETTE) {
    const s = hexToRgb(sw.hex)
    const d = Math.sqrt((cur.r - s.r) ** 2 + (cur.g - s.g) ** 2 + (cur.b - s.b) ** 2)
    if (d < bestDist) { bestDist = d; best = sw.hex }
  }
  emit_(best)
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

function isActive(sw: string): boolean {
  return props.modelValue.toLowerCase() === sw.toLowerCase()
}
</script>

<template>
  <div class="cp-wrap">
    <div class="cp-label" v-if="label">{{ label }}</div>

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

    <!-- FH palette swatches -->
    <div class="cp-swatches">
      <button
        v-for="sw in FH_PALETTE"
        :key="sw.hex"
        class="cp-swatch"
        :class="{ 'cp-swatch--active': isActive(sw.hex) }"
        type="button"
        :title="sw.name"
        :style="{ background: sw.hex }"
        @click="emit_(sw.hex)"
      >
        <span v-if="isActive(sw.hex)" class="cp-swatch-dot" />
      </button>
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
}
.cp-label {
  color: var(--steel);
  text-transform: uppercase;
  letter-spacing: .08em;
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
.cp-swatches {
  display: grid;
  grid-template-columns: repeat(auto-fill, 22px);
  gap: 3px;
  padding-top: 4px;
  border-top: 1px solid var(--panel-edge);
}
.cp-swatch {
  width: 22px;
  height: 22px;
  border-radius: 3px;
  border: 1px solid rgba(255,255,255,0.1);
  cursor: pointer;
  position: relative;
  padding: 0;
}
.cp-swatch--active {
  box-shadow: 0 0 0 2px var(--gold);
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
</style>
