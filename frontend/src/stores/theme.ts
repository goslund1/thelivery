import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Theme } from '../types'
import { api } from '../api'

export interface ThemeColors {
  asphalt:   string
  panel:     string
  panelEdge: string
  gold:      string
  magenta:   string
  paper:     string
  steel:     string
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

export interface ThemeData {
  colors:   ThemeColors
  tuning:   ThemeTuning
  fonts:    ThemeFonts
  ambiance: Theme
}

// Default colors per ambiance — used when switching base theme to reset the palette.
export const AMBIANCE_DEFAULTS: Record<Theme, ThemeColors> = {
  dark:    { asphalt: '#0b0b0d', panel: '#15151a', panelEdge: '#23232b', gold: '#c9a227', magenta: '#d6478f', paper: '#ece9e4', steel: '#7a7e87' },
  light:   { asphalt: '#f3efe6', panel: '#ffffff',  panelEdge: '#ddd8cd', gold: '#ebd150', magenta: '#30acb8', paper: '#201f1c', steel: '#6b6f76' },
  rainbow: { asphalt: '#0b0b14', panel: '#16151f',  panelEdge: '#2a2838', gold: '#e83d9c', magenta: '#3dc7e8', paper: '#f1edf7', steel: '#8d87a3' },
  clouds:  { asphalt: '#eef3f7', panel: '#f8fafc',  panelEdge: '#d7e3ec', gold: '#5b8fb0', magenta: '#8a7bc4', paper: '#2c3e4a', steel: '#7188a0' },
  stormy:  { asphalt: '#2a2e33', panel: '#353a40',  panelEdge: '#454b52', gold: '#7badc9', magenta: '#a596d6', paper: '#e4e7ea', steel: '#9aa2a9' },
}

const COLOR_VAR_MAP: Record<keyof ThemeColors, string> = {
  asphalt:   '--asphalt',
  panel:     '--panel',
  panelEdge: '--panel-edge',
  gold:      '--gold',
  magenta:   '--magenta',
  paper:     '--paper',
  steel:     '--steel',
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
      const data = raw as unknown as ThemeData
      current.value = deepClone(data)
      saved.value   = deepClone(data)
      applyAll(data)
    } catch (e) {
      error.value = (e as Error).message
    } finally {
      loading.value = false
    }
  }

  function setColor(key: keyof ThemeColors, value: string) {
    if (!current.value) return
    current.value.colors[key] = value
    document.documentElement.style.setProperty(COLOR_VAR_MAP[key], value)
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
      error.value = (e as Error).message
    } finally {
      saving.value = false
    }
  }

  return {
    current, saved, loading, saving, error, isDirty,
    load, setColor, setTuningColor, setAmbiance, reset, save,
  }
})
