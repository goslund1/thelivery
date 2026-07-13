import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Theme } from '../types'
import { api } from '../api'
import { errMsg } from '../utils/errMsg'

export interface ThemeColors {
  base:    string
  panel:      string
  panelEdge:  string
  accent:       string
  highlight:    string
  fg:      string
  muted:      string
  panelWell:  string
  mutedLight: string
}

export interface ThemeTuning {
  tires:        string
  gearing:      string
  alignment:    string
  arb:          string
  springs:      string
  damping:      string
  aero:         string
  brakes:       string
  differential: string
}

export interface ThemeFonts {
  mono:    string
  display: string
}

export interface ThemeEffects {
  glassOpacity:  number   // 0–100
  glassColor?:   string   // base color for glass panels; defaults to --panel
  pickerOpacity: number   // 0–100
  pickerColor?:  string   // base color for picker panel; defaults to --panel
  scrollDur:     number   // ms — card-jump scroll animation duration
}

export interface ThemeData {
  colors:   ThemeColors
  tuning:   ThemeTuning
  fonts:    ThemeFonts
  ambiance: Theme
  effects:  ThemeEffects
}

// Default colors per ambiance — used when switching base theme to reset the palette.
export const AMBIANCE_DEFAULTS: Record<Theme, ThemeColors> = {
  dark:    { base: '#0b0b0d', panel: '#15151a', panelEdge: '#23232b', accent: '#c9a227', highlight: '#d6478f', fg: '#ece9e4', muted: '#7a7e87', panelWell: '#0e0e11', mutedLight: '#a8a4ab' },
  light:   { base: '#f3efe6', panel: '#ffffff',  panelEdge: '#ddd8cd', accent: '#ebd150', highlight: '#30acb8', fg: '#201f1c', muted: '#6b6f76', panelWell: '#f0ede4', mutedLight: '#56565c' },
  rainbow: { base: '#0b0b14', panel: '#16151f',  panelEdge: '#2a2838', accent: '#e83d9c', highlight: '#3dc7e8', fg: '#f1edf7', muted: '#8d87a3', panelWell: '#100f18', mutedLight: '#b3adc6' },
  clouds:  { base: '#eef3f7', panel: '#f8fafc',  panelEdge: '#d7e3ec', accent: '#5b8fb0', highlight: '#8a7bc4', fg: '#2c3e4a', muted: '#7188a0', panelWell: '#e9eff4', mutedLight: '#5c7488' },
  stormy:  { base: '#2a2e33', panel: '#353a40',  panelEdge: '#454b52', accent: '#7badc9', highlight: '#a596d6', fg: '#e4e7ea', muted: '#9aa2a9', panelWell: '#232629', mutedLight: '#c5cbd1' },
}

const COLOR_VAR_MAP: Record<keyof ThemeColors, string> = {
  base:    '--base',
  panel:      '--panel',
  panelEdge:  '--panel-edge',
  accent:       '--accent',
  highlight:    '--highlight',
  fg:      '--fg',
  muted:      '--muted',
  panelWell:  '--panel-well',
  mutedLight: '--muted-light',
}

const TUNING_VAR_MAP: Record<keyof ThemeTuning, string> = {
  tires:        '--tabc-tires',
  gearing:      '--tabc-gearing',
  alignment:    '--tabc-alignment',
  arb:          '--tabc-arb',
  springs:      '--tabc-springs',
  damping:      '--tabc-damping',
  aero:         '--tabc-aero',
  brakes:       '--tabc-brakes',
  differential: '--tabc-differential',
}

function deepClone<T>(v: T): T {
  return JSON.parse(JSON.stringify(v))
}

const EFFECTS_DEFAULTS: ThemeEffects = { glassOpacity: 82, pickerOpacity: 18, scrollDur: 250 }

function applyEffects(effects: ThemeEffects) {
  const root = document.documentElement
  root.style.setProperty('--glass-opacity', `${effects.glassOpacity}%`)
  root.style.setProperty('--scroll-dur', `${effects.scrollDur ?? 250}ms`)
  if (effects.glassColor) {
    root.style.setProperty('--glass-bg', `color-mix(in srgb, ${effects.glassColor} ${effects.glassOpacity}%, transparent)`)
  } else {
    root.style.removeProperty('--glass-bg')
  }
  const pickerBase = effects.pickerColor ?? effects.glassColor
  if (pickerBase) {
    root.style.setProperty('--picker-glass-bg', `color-mix(in srgb, ${pickerBase} ${effects.pickerOpacity}%, transparent)`)
  } else {
    root.style.setProperty('--picker-glass-bg', `color-mix(in srgb, var(--panel) ${effects.pickerOpacity}%, transparent)`)
  }
}

const LEGACY_COLOR_MAP: Record<string, keyof ThemeColors> = {
  asphalt: 'base', gold: 'accent', magenta: 'highlight',
  paper: 'fg', steel: 'muted', steelLight: 'mutedLight',
}

function normalize(data: ThemeData): ThemeData {
  const ambiance = data.ambiance ?? 'dark'
  const defaults = AMBIANCE_DEFAULTS[ambiance] ?? AMBIANCE_DEFAULTS.dark
  const raw = data.colors as unknown as Record<string, string>
  const migrated: Partial<ThemeColors> = {}
  for (const [k, v] of Object.entries(raw)) {
    const mapped = LEGACY_COLOR_MAP[k] ?? k as keyof ThemeColors
    migrated[mapped] = v
  }
  return {
    ...data,
    colors:  { ...defaults, ...migrated } as ThemeColors,
    effects: { ...EFFECTS_DEFAULTS, ...(data.effects ?? {}) },
  }
}

function applyColors(colors: ThemeColors) {
  const root = document.documentElement
  for (const [key, cssVar] of Object.entries(COLOR_VAR_MAP)) {
    root.style.setProperty(cssVar, colors[key as keyof ThemeColors])
  }
}

function applyTuning(tuning: ThemeTuning) {
  const root = document.documentElement
  for (const [key, cssVar] of Object.entries(TUNING_VAR_MAP)) {
    root.style.setProperty(cssVar, tuning[key as keyof ThemeTuning])
  }
}

export const useThemeStore = defineStore('theme', () => {
  const current  = ref<ThemeData | null>(null)
  const saved    = ref<ThemeData | null>(null)
  const loading  = ref(false)
  const saving   = ref(false)
  const error    = ref('')

  const isDirty = computed(() =>
    JSON.stringify(current.value) !== JSON.stringify(saved.value)
  )

  function applyAll(data: ThemeData) {
    applyColors(data.colors)
    applyTuning(data.tuning)
    applyEffects(data.effects)
    // Sync ambiance to the ui store's theme (which sets data-theme on <html>).
    // Import lazily to avoid circular dep at module load time.
    import('./ui').then(({ useUiStore }) => {
      useUiStore().theme = data.ambiance
    })
  }

  async function load() {
    loading.value = true
    error.value = ''
    try {
      const raw = await api.getTheme()
      const data = normalize(raw as unknown as ThemeData)
      current.value = deepClone(data)
      saved.value   = deepClone(data)
      applyAll(data)
    } catch (e) {
      error.value = errMsg(e)
    } finally {
      loading.value = false
    }
  }

  function setColor(key: keyof ThemeColors, value: string) {
    if (!current.value) return
    current.value.colors[key] = value
    document.documentElement.style.setProperty(COLOR_VAR_MAP[key], value)
  }

  function setGlassOpacity(value: number) {
    if (!current.value) return
    current.value.effects.glassOpacity = value
    applyEffects(current.value.effects)
  }

  function setPickerOpacity(value: number) {
    if (!current.value) return
    current.value.effects.pickerOpacity = value
  }

  function setScrollDur(value: number) {
    if (!current.value) return
    current.value.effects.scrollDur = value
    applyEffects(current.value.effects)
  }

  function setEffectColor(key: 'glassColor' | 'pickerColor', value: string) {
    if (!current.value) return
    current.value.effects[key] = value
    applyEffects(current.value.effects)
  }

  function setTuningColor(key: keyof ThemeTuning, value: string) {
    if (!current.value) return
    current.value.tuning[key] = value
    document.documentElement.style.setProperty(TUNING_VAR_MAP[key], value)
  }

  function setAmbiance(ambiance: Theme) {
    if (!current.value) return
    current.value.ambiance = ambiance
    current.value.colors = deepClone(AMBIANCE_DEFAULTS[ambiance])
    applyColors(current.value.colors)
    import('./ui').then(({ useUiStore }) => {
      useUiStore().theme = ambiance
    })
  }

  function reset() {
    if (!saved.value) return
    current.value = deepClone(saved.value)
    applyAll(current.value)
  }

  async function save() {
    if (!current.value) return
    saving.value = true
    error.value = ''
    try {
      const raw = await api.putTheme(current.value as unknown as Record<string, unknown>)
      const data = raw as unknown as ThemeData
      current.value = deepClone(data)
      saved.value   = deepClone(data)
    } catch (e) {
      error.value = errMsg(e)
    } finally {
      saving.value = false
    }
  }

  return {
    current, saved, loading, saving, error, isDirty,
    load, setColor, setTuningColor, setGlassOpacity, setPickerOpacity, setScrollDur, setEffectColor, setAmbiance, reset, save,
  }
})
