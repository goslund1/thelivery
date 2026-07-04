<script setup lang="ts">
import { ref, computed } from 'vue'
import { useThemeStore } from '../stores/theme'
import type { ThemeColors, ThemeTuning } from '../stores/theme'
import ColorPicker from './ColorPicker.vue'
import type { Theme } from '../types'

const emit = defineEmits<{ close: [] }>()

const theme = useThemeStore()

const pickerBg = computed(() => {
  const hex = (theme.current?.colors.panel ?? '#15151a').replace('#', '')
  const n = parseInt(hex.length === 3 ? hex.split('').map(c => c+c).join('') : hex, 16)
  return `rgba(${(n>>16)&255},${(n>>8)&255},${n&255},0.18)`
})

const paletteOpen  = ref(true)
const tuningOpen   = ref(false)
const advancedOpen = ref(false)
const pickerOpen   = ref(false)

const COLOR_LABELS: Record<keyof ThemeColors, string> = {
  asphalt:    'Page background',
  panel:      'Card / panel',
  panelEdge:  'Panel border',
  gold:       'Gold accent',
  magenta:    'Magenta accent',
  paper:      'Primary text',
  steel:      'Muted text',
  panelWell:  'Inset / well',
  steelLight: 'Light muted text',
}

const TUNING_LABELS: Record<keyof ThemeTuning, string> = {
  tires: 'Tires', gearing: 'Gearing', alignment: 'Alignment',
  arb: 'ARB', springs: 'Springs', damping: 'Damping',
  aero: 'Aero', brakes: 'Brakes', differential: 'Differential',
}

const MAIN_KEYS:     (keyof ThemeColors)[] = ['asphalt', 'panel', 'panelEdge', 'gold', 'magenta', 'paper', 'steel']
const ADVANCED_KEYS: (keyof ThemeColors)[] = ['panelWell', 'steelLight']

const THEMES: Theme[] = ['dark', 'light', 'rainbow', 'clouds', 'stormy']
const THEME_LABELS: Record<Theme, string> = {
  dark: 'Dark', light: 'Light', rainbow: 'Rainbow', clouds: 'Clouds', stormy: 'Stormy',
}

const activeColor = ref<{ group: 'colors' | 'tuning'; key: string } | null>(null)

// pickerColor is the picker's own state — independent of what's selected in the list
const pickerColor = ref(theme.current?.colors.gold ?? '#d4a017')

// Swatch in the header reflects the active item's current store value
const activeValue = computed<string>(() => {
  if (!activeColor.value || !theme.current) return pickerColor.value
  const { group, key } = activeColor.value
  return group === 'colors'
    ? theme.current.colors[key as keyof ThemeColors]
    : theme.current.tuning[key as keyof ThemeTuning]
})

const activeLabel = computed<string>(() => {
  if (!activeColor.value) return ''
  const { group, key } = activeColor.value
  return group === 'colors'
    ? COLOR_LABELS[key as keyof ThemeColors]
    : TUNING_LABELS[key as keyof ThemeTuning]
})

function onPickerUpdate(val: string) {
  pickerColor.value = val
  // Live-update the active item while dragging
  if (!activeColor.value) return
  const { group, key } = activeColor.value
  if (group === 'colors') theme.setColor(key as keyof ThemeColors, val)
  else theme.setTuningColor(key as keyof ThemeTuning, val)
}

function selectColor(group: 'colors' | 'tuning', key: string) {
  if (pickerOpen.value) {
    // Picker is open: stamp current pickerColor onto the clicked item
    if (group === 'colors') theme.setColor(key as keyof ThemeColors, pickerColor.value)
    else theme.setTuningColor(key as keyof ThemeTuning, pickerColor.value)
    activeColor.value = { group, key }
  } else {
    // Picker is closed: load the item's color and open
    const val = group === 'colors'
      ? (theme.current?.colors[key as keyof ThemeColors] ?? pickerColor.value)
      : (theme.current?.tuning[key as keyof ThemeTuning] ?? pickerColor.value)
    pickerColor.value = val
    activeColor.value = { group, key }
    pickerOpen.value = true
  }
}

function isActive(group: 'colors' | 'tuning', key: string) {
  return activeColor.value?.group === group && activeColor.value?.key === key
}

function onAmbianceChange(t: Theme) {
  theme.setAmbiance(t)
  activeColor.value = null
}

function togglePicker() {
  pickerOpen.value = !pickerOpen.value
}

async function onSave() { await theme.save() }
function onReset() { theme.reset() }
</script>

<template>
  <div class="tb-wrap">
    <!-- Picker pane — single glass surface containing wing + tab -->
    <div class="tb-picker-pane" :class="{ open: pickerOpen }" :style="{ background: pickerBg }">
      <div class="tb-picker-wing" v-scroll-contain>
        <div class="tb-picker-header">
          <span class="tb-picker-for">
            <span
              v-if="activeColor"
              class="tb-picker-swatch"
              :style="{ background: activeValue }"
            />
            {{ activeLabel || 'Select a color →' }}
          </span>
        </div>
        <div class="tb-picker-body">
          <ColorPicker
            :model-value="pickerColor"
            @update:model-value="onPickerUpdate"
          />
        </div>
      </div>

      <!-- Toggle tab — right edge of the pane, always visible -->
      <button
        class="tb-picker-tab"
        :class="{ open: pickerOpen }"
        type="button"
        :title="pickerOpen ? 'Hide color picker' : 'Show color picker'"
        @click="togglePicker"
      >‹</button>
    </div>

    <!-- Main list panel -->
    <div class="tb-panel" v-scroll-contain>

      <div class="tb-header">
        <span class="tb-title">Theme Builder</span>
        <button class="tb-close" type="button" @click="emit('close')">×</button>
      </div>

      <div class="tb-body">

        <!-- Ambiance -->
        <div class="tb-section">
          <div class="tb-section-label">Base ambiance</div>
          <div class="tb-ambiance-row">
            <button
              v-for="t in THEMES" :key="t"
              class="tb-ambiance-btn"
              :class="{ active: theme.current?.ambiance === t }"
              type="button"
              @click="onAmbianceChange(t)"
            >{{ THEME_LABELS[t] }}</button>
          </div>
        </div>

        <!-- Effects -->
        <div class="tb-section">
          <div class="tb-section-label">Effects</div>
          <div class="tb-effect-row">
            <span class="tb-effect-label">Panel opacity</span>
            <input
              class="tb-slider" type="range" min="20" max="100"
              :value="theme.current?.effects.glassOpacity ?? 82"
              @input="theme.setGlassOpacity(+($event.target as HTMLInputElement).value)"
            />
            <span class="tb-effect-val">{{ theme.current?.effects.glassOpacity ?? 82 }}%</span>
          </div>
        </div>

        <!-- Main palette -->
        <div class="tb-section">
          <button class="tb-section-toggle" type="button" @click="paletteOpen = !paletteOpen">
            <span class="tb-section-label">Main palette</span>
            <span class="tb-chevron" :class="{ open: paletteOpen }">›</span>
          </button>
          <div v-if="paletteOpen" class="tb-swatches-grid">
            <button
              v-for="key in MAIN_KEYS" :key="key"
              class="tb-color-row" :class="{ active: isActive('colors', key) }"
              type="button" @click="selectColor('colors', key)"
            >
              <span class="tb-swatch" :style="{ background: theme.current?.colors[key] }" />
              <span class="tb-color-label">{{ COLOR_LABELS[key] }}</span>
              <span class="tb-color-hex">{{ theme.current?.colors[key] }}</span>
            </button>
          </div>
        </div>

        <!-- Advanced -->
        <div class="tb-section">
          <button class="tb-section-toggle" type="button" @click="advancedOpen = !advancedOpen">
            <span class="tb-section-label">Advanced</span>
            <span class="tb-chevron" :class="{ open: advancedOpen }">›</span>
          </button>
          <div v-if="advancedOpen" class="tb-swatches-grid">
            <button
              v-for="key in ADVANCED_KEYS" :key="key"
              class="tb-color-row" :class="{ active: isActive('colors', key) }"
              type="button" @click="selectColor('colors', key)"
            >
              <span class="tb-swatch" :style="{ background: theme.current?.colors[key] }" />
              <span class="tb-color-label">{{ COLOR_LABELS[key] }}</span>
              <span class="tb-color-hex">{{ theme.current?.colors[key] }}</span>
            </button>
          </div>
        </div>

        <!-- Tuning palette -->
        <div class="tb-section">
          <button class="tb-section-toggle" type="button" @click="tuningOpen = !tuningOpen">
            <span class="tb-section-label">Tuning palette</span>
            <span class="tb-chevron" :class="{ open: tuningOpen }">›</span>
          </button>
          <div v-if="tuningOpen" class="tb-swatches-grid">
            <button
              v-for="(label, key) in TUNING_LABELS" :key="key"
              class="tb-color-row" :class="{ active: isActive('tuning', key) }"
              type="button" @click="selectColor('tuning', key)"
            >
              <span class="tb-swatch" :style="{ background: theme.current?.tuning[key as keyof ThemeTuning] }" />
              <span class="tb-color-label">{{ label }}</span>
              <span class="tb-color-hex">{{ theme.current?.tuning[key as keyof ThemeTuning] }}</span>
            </button>
          </div>
        </div>

      </div><!-- /.tb-body -->

      <div class="tb-footer">
        <span v-if="theme.error" class="tb-error">{{ theme.error }}</span>
        <div class="tb-footer-actions">
          <button class="tb-btn-reset" type="button" :disabled="!theme.isDirty" @click="onReset">Reset</button>
          <button class="tb-btn-save" type="button" :disabled="!theme.isDirty || theme.saving" @click="onSave">
            {{ theme.saving ? 'Saving…' : 'Save →' }}
          </button>
        </div>
      </div>

    </div><!-- /.tb-panel -->
  </div>
</template>

<style scoped>
.tb-wrap {
  display: flex;
  flex-direction: row;
  align-items: stretch;
  min-width: 0;
  height: 600px;
  max-height: 85vh;
}

/* ── Picker pane — single unified glass surface (wing + tab) ── */
.tb-picker-pane {
  display: flex;
  flex-direction: row;
  align-items: stretch;
  width: 14px;
  overflow: hidden;
  transition: width 0.22s ease;
  flex-shrink: 0;
  align-self: stretch;
  margin-top: 4px;
  margin-bottom: 4px;
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  border: 1px solid var(--glass-border);
  border-right: none;
  border-radius: 6px 0 0 0;
}
.tb-picker-pane.open {
  width: 286px; /* 272px wing + 14px tab */
}

/* ── Picker wing — transparent child, fills pane ── */
.tb-picker-wing {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
}
.tb-picker-header {
  padding: 12px 14px 10px;
  border-bottom: 1px solid var(--panel-edge);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  white-space: nowrap;
}
.tb-picker-for {
  color: var(--paper);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: .07em;
  display: flex;
  align-items: center;
  gap: 7px;
}
.tb-picker-swatch {
  width: 14px;
  height: 14px;
  border-radius: 3px;
  border: 1px solid rgba(255,255,255,0.15);
  flex-shrink: 0;
}
.tb-picker-body {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  overscroll-behavior: contain;
  padding: 14px;
}

/* ── Toggle tab — right edge of the pane, same plane as wing ── */
.tb-picker-tab {
  flex-shrink: 0;
  width: 14px;
  align-self: stretch;
  background: transparent;
  border: none;
  border-left: 1px solid rgba(255,255,255,0.06);
  color: var(--steel);
  font-size: 13px;
  cursor: pointer;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding: 10px 0 0;
  transition: color .15s, transform .22s;
}
.tb-picker-tab:hover { color: var(--gold); }
.tb-picker-tab {
  position: relative;
}
.tb-picker-tab::after {
  content: '';
  position: absolute;
  left: -1px;
  right: 0;
  top: 36px;
  height: 1px;
  background: var(--panel-edge);
  opacity: 0;
  transition: opacity 0s 0.22s;
}
.tb-picker-pane.open .tb-picker-tab::after {
  opacity: 1;
  transition: opacity 0s;
}
.tb-picker-tab.open { transform: scaleX(-1); }

/* ── Main panel ── */
.tb-panel {
  width: 300px;
  flex-shrink: 1;
  min-width: 200px;
  height: 600px;
  max-height: 85vh;
  background: var(--glass-bg);
  backdrop-filter: var(--glass-blur);
  -webkit-backdrop-filter: var(--glass-blur);
  border: 1px solid var(--glass-border);
  border-radius: 0 6px 6px 0;
  box-shadow: 0 8px 32px rgba(0,0,0,0.5);
  clip-path: inset(0 -60px -60px 0);
  display: flex;
  flex-direction: column;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  overflow: hidden;
}
.tb-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px 10px;
  border-bottom: 1px solid var(--panel-edge);
  flex-shrink: 0;
}
.tb-body {
  overflow-y: auto;
  flex: 1;
  min-height: 0;
  overscroll-behavior: contain;
}
.tb-title {
  color: var(--paper);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: .1em;
}
.tb-close {
  background: none;
  border: none;
  color: var(--steel);
  font-size: 18px;
  line-height: 1;
  cursor: pointer;
  padding: 0;
}
.tb-close:hover { color: var(--paper); }

.tb-section {
  border-bottom: 1px solid var(--panel-edge);
  padding: 10px 14px;
}
.tb-section-label {
  color: var(--steel);
  text-transform: uppercase;
  letter-spacing: .08em;
  font-size: 10px;
}
.tb-section-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  background: none;
  border: none;
  cursor: pointer;
  padding: 0;
  margin-bottom: 2px;
}
.tb-section-toggle:hover .tb-section-label { color: var(--paper); }
.tb-chevron {
  color: var(--steel);
  font-size: 14px;
  transition: transform .15s;
  display: inline-block;
}
.tb-chevron.open { transform: rotate(90deg); }

.tb-effect-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
}
.tb-effect-label {
  color: var(--paper);
  font-size: 11px;
  flex: 1;
}
.tb-slider {
  flex: 1;
  accent-color: var(--gold);
  cursor: pointer;
}
.tb-effect-val {
  color: var(--steel);
  font-size: 10px;
  min-width: 32px;
  text-align: right;
}

.tb-ambiance-row {
  display: flex;
  gap: 5px;
  margin-top: 7px;
  flex-wrap: wrap;
}
.tb-ambiance-btn {
  background: var(--panel-well);
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: var(--steel);
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: .05em;
  padding: 4px 8px;
  cursor: pointer;
  transition: border-color .12s, color .12s;
}
.tb-ambiance-btn:hover { border-color: var(--gold); color: var(--gold); }
.tb-ambiance-btn.active { border-color: var(--gold); color: var(--gold); background: var(--gold-tint-06); }

.tb-swatches-grid {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-top: 8px;
}
.tb-color-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 6px;
  border-radius: 3px;
  border: 1px solid transparent;
  background: none;
  cursor: pointer;
  width: 100%;
  text-align: left;
  transition: background .1s, border-color .1s;
}
.tb-color-row:hover { background: var(--panel-well); }
.tb-color-row.active {
  background: var(--gold-tint-06);
  border-color: var(--gold);
}
.tb-swatch {
  width: 16px;
  height: 16px;
  border-radius: 3px;
  border: 1px solid rgba(255,255,255,0.12);
  flex-shrink: 0;
}
.tb-color-label {
  flex: 1;
  color: var(--paper);
  font-size: 11px;
}
.tb-color-hex {
  color: var(--steel);
  font-size: 10px;
}

.tb-footer {
  padding: 10px 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex-shrink: 0;
  border-top: 1px solid var(--panel-edge);
}
.tb-error { color: #c0392b; font-size: 10px; }
.tb-footer-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
.tb-btn-reset {
  background: none;
  border: 1px solid var(--panel-edge);
  border-radius: 3px;
  color: var(--steel);
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: .06em;
  padding: 5px 12px;
  cursor: pointer;
  transition: border-color .12s, color .12s;
}
.tb-btn-reset:hover:not(:disabled) { border-color: var(--paper); color: var(--paper); }
.tb-btn-reset:disabled { opacity: 0.35; cursor: default; }
.tb-btn-save {
  background: var(--gold-tint-14);
  border: 1px solid var(--gold);
  border-radius: 3px;
  color: var(--gold);
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: .06em;
  padding: 5px 12px;
  cursor: pointer;
  transition: background .12s;
}
.tb-btn-save:hover:not(:disabled) { filter: brightness(1.15); }
.tb-btn-save:disabled { opacity: 0.35; cursor: default; }
</style>
