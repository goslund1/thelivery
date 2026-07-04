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
</style>
