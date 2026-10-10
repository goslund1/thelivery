import { reactive } from 'vue'
import type { Theme } from '../types'

// A visitor's memory (work order Part B, D5/D7): what this browser keeps for the
// person using it — their favourite stars, theme pick and text size. Nothing is
// sent to the server. One versioned localStorage key, read once at start and
// written on every change. Storage can be unavailable (private mode, blocked,
// full): every access is guarded and failure costs only the memory.
//
// load() and save() are the only two functions that touch the backing store, so
// it can become the cookie the planned guest theme adjuster wants without
// changing any caller.

const KEY = 'livery:visitor'
const VERSION = 1
const THEMES: Theme[] = ['dark', 'light', 'rainbow', 'clouds', 'stormy']

interface Stored {
  v: number
  favorites: string[]
  theme?: Theme
  textDelta?: number
}

function load(): Stored {
  const empty: Stored = { v: VERSION, favorites: [] }
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? 'null')
    if (!raw || raw.v !== VERSION) return empty
    return {
      v: VERSION,
      favorites: Array.isArray(raw.favorites) ? raw.favorites.filter((id: unknown) => typeof id === 'string') : [],
      theme: THEMES.includes(raw.theme) ? raw.theme : undefined,
      textDelta: Number.isInteger(raw.textDelta) ? raw.textDelta : undefined,
    }
  } catch {
    return empty // unavailable or corrupt
  }
}

function save(m: Stored) {
  try {
    localStorage.setItem(KEY, JSON.stringify(m))
  } catch { /* unavailable or full — the memory just isn't kept */ }
}

// Module-level, so every component sees the same memory.
const memory = reactive(load())

export function isFavorite(cardId: string): boolean {
  return memory.favorites.includes(cardId)
}

export function toggleFavorite(cardId: string) {
  const i = memory.favorites.indexOf(cardId)
  if (i >= 0) memory.favorites.splice(i, 1)
  else memory.favorites.push(cardId)
  save(memory)
}

export function rememberedTheme(): Theme | undefined {
  return memory.theme
}

export function rememberTheme(t: Theme) {
  memory.theme = t
  save(memory)
}

export function rememberedTextDelta(): number | undefined {
  return memory.textDelta
}

export function rememberTextDelta(d: number) {
  memory.textDelta = d
  save(memory)
}
