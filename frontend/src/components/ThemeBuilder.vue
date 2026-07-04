<script setup lang="ts">
import { ref, computed } from 'vue'
import { useThemeStore } from '../stores/theme'
import type { ThemeColors, ThemeTuning } from '../stores/theme'
import ColorPicker from './ColorPicker.vue'
import type { Theme } from '../types'

const emit = defineEmits<{ close: [] }>()

const theme = useThemeStore()

const paletteOpen = ref(true)
const tuningOpen  = ref(false)

const COLOR_LABELS: Record<keyof ThemeColors, string> = {
  asphalt:   'Page background',
  panel:     'Card / panel',
  panelEdge: 'Panel border',
  gold:      'Gold accent',
  magenta:   'Magenta accent',
  paper:     'Primary text',
  steel:     'Muted text',
}

const TUNING_LABELS: Record<keyof ThemeTuning, string> = {
  tires:        'Tires',
  gearing:      'Gearing',
  alignment:    'Alignment',
  arb:          'ARB',
  springs:      'Springs',
  damping:      'Damping',
  aero:         'Aero',
  brakes:       'Brakes',
  differential: 'Differential',
}

const THEMES: Theme[] = ['dark', 'light', 'rainbow', 'clouds', 'stormy']
const THEME_LABELS: Record<Theme, string> = {
  dark: 'Dark', light: 'Light', rainbow: 'Rainbow', clouds: 'Clouds', stormy: 'Stormy',
}

const activeColor = ref<{ group: 'colors' | 'tuning'; key: string } | null>(null)

const activeValue = computed<string>(() => {
  if (!activeColor.value || !theme.current) return '#000000'
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
  if (!activeColor.value) return
  const { group, key } = activeColor.value
  if (group === 'colors') theme.setColor(key as keyof ThemeColors, val)
  else theme.setTuningColor(key as keyof ThemeTuning, val)
}

function selectColor(group: 'colors' | 'tuning', key: string) {
  if (activeColor.value?.group === group && activeColor.value?.key === key) {
    activeColor.value = null
  } else {
    activeColor.value = { group, key }
  }
}

function onAmbianceChange(t: Theme) {
  theme.setAmbiance(t)
  activeColor.value = null
}

async function onSave() {
  await theme.save()
}

function onReset() {
  theme.reset()
  activeColor.value = null
}
</script>

<template>
  <div class="tb-panel" v-scroll-contain>
    <div class="tb-header">
      <span class="tb-title">Theme Builder</span>
      <button class="tb-close" type="button" @click="emit('close')">×</button>
    </div>

    <div class="tb-body">
    <!-- Ambiance base -->
    <div class="tb-section">
      <div class="tb-section-label">Base ambiance</div>
      <div class="tb-ambiance-row">
        <button
          v-for="t in THEMES"
          :key="t"
          class="tb-ambiance-btn"
          :class="{ active: theme.current?.ambiance === t }"
          type="button"
          @click="onAmbianceChange(t)"
        >{{ THEME_LABELS[t] }}</button>
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
          v-for="(label, key) in COLOR_LABELS"
          :key="key"
          class="tb-color-row"
          :class="{ active: activeColor?.group === 'colors' && activeColor?.key === key }"
          type="button"
          @click="selectColor('colors', key)"
        >
          <span
            class="tb-swatch"
            :style="{ background: theme.current?.colors[key as keyof ThemeColors] }"
          />
          <span class="tb-color-label">{{ label }}</span>
          <span class="tb-color-hex">{{ theme.current?.colors[key as keyof ThemeColors] }}</span>
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
          v-for="(label, key) in TUNING_LABELS"
          :key="key"
          class="tb-color-row"
          :class="{ active: activeColor?.group === 'tuning' && activeColor?.key === key }"
          type="button"
          @click="selectColor('tuning', key)"
        >
          <span
            class="tb-swatch"
            :style="{ background: theme.current?.tuning[key as keyof ThemeTuning] }"
          />
          <span class="tb-color-label">{{ label }}</span>
          <span class="tb-color-hex">{{ theme.current?.tuning[key as keyof ThemeTuning] }}</span>
        </button>
      </div>
    </div>

    <!-- Inline color picker (appears when a swatch row is selected) -->
    <div v-if="activeColor" class="tb-picker-area">
      <ColorPicker
        :label="activeLabel"
        :model-value="activeValue"
        @update:model-value="onPickerUpdate"
      />
    </div>

    </div><!-- /.tb-body -->

    <!-- Footer -->
    <div class="tb-footer">
      <span v-if="theme.error" class="tb-error">{{ theme.error }}</span>
      <div class="tb-footer-actions">
        <button
          class="tb-btn-reset"
          type="button"
          :disabled="!theme.isDirty"
          @click="onReset"
        >Reset</button>
        <button
          class="tb-btn-save"
          type="button"
          :disabled="!theme.isDirty || theme.saving"
          @click="onSave"
        >{{ theme.saving ? 'Saving…' : 'Save →' }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tb-panel {
  width: 300px;
  max-height: 80vh;
  background: color-mix(in srgb, var(--panel) 82%, transparent);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border: 1px solid color-mix(in srgb, var(--panel-edge) 70%, transparent);
  border-radius: 6px;
  box-shadow: 0 8px 32px rgba(0,0,0,0.5);
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

.tb-picker-area {
  padding: 12px 14px;
  border-bottom: 1px solid var(--panel-edge);
  background: var(--panel-well);
}

.tb-footer {
  padding: 10px 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex-shrink: 0;
  border-top: 1px solid var(--panel-edge);
}
.tb-error {
  color: #c0392b;
  font-size: 10px;
}
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
.tb-btn-save:hover:not(:disabled) { background: var(--gold-tint-14); filter: brightness(1.15); }
.tb-btn-save:disabled { opacity: 0.35; cursor: default; }
</style>
